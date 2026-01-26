use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::audit::{AuditAction, AuditLogEntry};
use crate::auth::{get_claims_from_request, jwt_validator, Claims, JwtManager};
use crate::domain::models::SensorReading;
use crate::domain::store::AppState;
use crate::errors::AppError;
use crate::fhir::FhirObservation;
use crate::ml_client::MlClient;
use crate::ws::{ws_live, WsHub};

pub fn configure(cfg: &mut web::ServiceConfig) {
    let (tx, _rx) = broadcast::channel::<FhirObservation>(256);

    // Initialize ML client if ML_SERVICE_URL is set
    let ml_client = std::env::var("ML_SERVICE_URL")
        .ok()
        .map(|url| Arc::new(MlClient::new(url)));

    if let Some(ref client) = ml_client {
        cfg.app_data(web::Data::new(client.clone()));
    }

    // JWT authentication middleware
    let auth_middleware = HttpAuthentication::bearer(jwt_validator);

    cfg.app_data(web::Data::new(WsHub { tx }))
        // Public endpoints (no auth required)
        .route("/healthz", web::get().to(healthz))
        .route("/auth/login", web::post().to(login))
        .route("/auth/token", web::post().to(generate_device_token))
        .route("/ws/live", web::get().to(ws_live))  // WebSocket endpoint (public for browser compatibility)
        .route("/ingest", web::post().to(ingest_public))   // Public ingest for simulator/mock data
        // Protected endpoints (JWT required)
        .service(
            web::scope("/api")
                .wrap(auth_middleware)
                .route("/ingest", web::post().to(ingest))
                .route("/fhir/Observation", web::get().to(get_observations))
                // ML endpoints
                .route("/ml/predict", web::get().to(ml_predict))
                .route("/ml/analysis", web::get().to(ml_analysis))
                .route("/ml/train", web::post().to(ml_train))
                .route("/ml/health", web::get().to(ml_health)),
        );
}

async fn healthz(
    state: web::Data<Arc<Mutex<AppState>>>,
    ml_client: Option<web::Data<Arc<MlClient>>>,
) -> Result<HttpResponse, AppError> {
    // Check database connection if configured
    let st = state.lock().await;
    st.health_check().await?;

    let mut response = serde_json::json!({
        "status": "ok",
        "database": if st.has_database() { "connected" } else { "in-memory-only" },
        "authentication": "JWT enabled"
    });

    // Check ML service if configured
    if let Some(client) = ml_client {
        match client.health_check().await {
            Ok(ml_health) => {
                response["ml_service"] = serde_json::json!({
                    "status": ml_health.status,
                    "connected": true,
                    "classifier_loaded": ml_health.classifier_loaded,
                    "anomaly_detector_loaded": ml_health.anomaly_detector_loaded
                });
            }
            Err(e) => {
                tracing::warn!("ML service health check failed: {}", e);
                response["ml_service"] = serde_json::json!({
                    "status": "error",
                    "connected": false,
                    "error": e
                });
            }
        }
    } else {
        response["ml_service"] = serde_json::json!({
            "status": "not_configured",
            "connected": false
        });
    }

    Ok(HttpResponse::Ok().json(response))
}

// Authentication endpoints

#[derive(serde::Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(serde::Serialize)]
struct LoginResponse {
    token: String,
    expires_in: i64,
    role: String,
}

async fn login(body: web::Json<LoginRequest>) -> Result<HttpResponse, AppError> {
    // In production, validate against database with hashed passwords
    // For now, using environment variable for demo
    let valid_username = std::env::var("AUTH_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let valid_password = std::env::var("AUTH_PASSWORD").unwrap_or_else(|_| "admin123".to_string());

    if body.username != valid_username || body.password != valid_password {
        tracing::warn!("Failed login attempt for user: {}", body.username);
        return Err(AppError::Unauthorized);
    }

    // Generate JWT token
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "default_secret_change_in_production".to_string());
    
    let jwt_manager = JwtManager::new(jwt_secret);
    let expires_in_hours = 24;
    
    let claims = Claims::new(
        body.username.clone(),
        "admin".to_string(),
        None,
        expires_in_hours,
    );

    match jwt_manager.generate_token(claims) {
        Ok(token) => {
            tracing::info!("User {} logged in successfully", body.username);
            Ok(HttpResponse::Ok().json(LoginResponse {
                token,
                expires_in: expires_in_hours * 3600, // in seconds
                role: "admin".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("Failed to generate token: {}", e);
            Err(AppError::Internal)
        }
    }
}

#[derive(serde::Deserialize)]
struct DeviceTokenRequest {
    device_id: String,
    secret: String, // Admin secret to generate device tokens
}

async fn generate_device_token(body: web::Json<DeviceTokenRequest>) -> Result<HttpResponse, AppError> {
    // Verify admin secret
    let admin_secret = std::env::var("DEVICE_TOKEN_SECRET")
        .unwrap_or_else(|_| "change_this_secret".to_string());

    if body.secret != admin_secret {
        tracing::warn!("Invalid device token generation attempt for device: {}", body.device_id);
        return Err(AppError::Unauthorized);
    }

    // Generate JWT token for device
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "default_secret_change_in_production".to_string());
    
    let jwt_manager = JwtManager::new(jwt_secret);
    let expires_in_hours = 8760; // 1 year for devices
    
    let claims = Claims::new(
        format!("device_{}", body.device_id),
        "device".to_string(),
        Some(body.device_id.clone()),
        expires_in_hours,
    );

    match jwt_manager.generate_token(claims) {
        Ok(token) => {
            tracing::info!("Generated token for device: {}", body.device_id);
            Ok(HttpResponse::Ok().json(LoginResponse {
                token,
                expires_in: expires_in_hours * 3600,
                role: "device".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("Failed to generate device token: {}", e);
            Err(AppError::Internal)
        }
    }
}

// Protected endpoints

// Public ingest endpoint (no auth required - for simulator and mock data)
async fn ingest_public(
    state: web::Data<Arc<Mutex<AppState>>>,
    hub: web::Data<WsHub>,
    payload: web::Json<SensorReading>,
) -> Result<HttpResponse, AppError> {
    tracing::debug!("Public ingest request (no auth)");

    // Validate
    let reading = payload.into_inner();
    reading.validate().map_err(AppError::BadRequest)?;

    // Convert to FHIR Observation
    let obs = FhirObservation::from_reading(reading.clone());
    
    // Validate FHIR schema compliance
    obs.validate().map_err(AppError::BadRequest)?;

    // Store reading (now with database support)
    {
        let mut st = state.lock().await;
        st.push(reading, None).await?;
    }

    // Push to WebSocket subscribers
    let _ = hub.tx.send(obs.clone());

    Ok(HttpResponse::Ok().json(obs))
}

// Protected ingest endpoint (JWT required)
async fn ingest(
    req: HttpRequest,
    state: web::Data<Arc<Mutex<AppState>>>,
    hub: web::Data<WsHub>,
    payload: web::Json<SensorReading>,
) -> Result<HttpResponse, AppError> {
    // Get authenticated user from JWT claims
    let claims = get_claims_from_request(&req)
        .ok_or_else(|| AppError::Unauthorized)?;

    tracing::debug!("Ingest request from user: {}, role: {}", claims.sub, claims.role);

    // Validate
    let reading = payload.into_inner();
    reading.validate().map_err(AppError::BadRequest)?;

    // Convert to FHIR Observation
    let obs = FhirObservation::from_reading(reading.clone());
    
    // Validate FHIR schema compliance
    obs.validate().map_err(AppError::BadRequest)?;

    // Store reading (now with database support and audit logging)
    {
        let mut st = state.lock().await;
        st.push(reading.clone(), Some(&claims)).await?;
    }

    // Push to WebSocket subscribers
    let _ = hub.tx.send(obs.clone());

    Ok(HttpResponse::Ok().json(obs))
}

#[derive(serde::Deserialize)]
struct ObsQuery {
    code: Option<String>,
    limit: Option<usize>,
}

async fn get_observations(
    req: HttpRequest,
    state: web::Data<Arc<Mutex<AppState>>>,
    q: web::Query<ObsQuery>,
) -> Result<HttpResponse, AppError> {
    // Verify authentication
    let _claims = get_claims_from_request(&req)
        .ok_or_else(|| AppError::Unauthorized)?;

    let limit = q.limit.unwrap_or(100).min(500);

    let st = state.lock().await;

    let bundle = if let Some(code) = &q.code {
        st.bundle_by_code(limit, code).await?
    } else {
        st.bundle(limit, None).await?
    };

    Ok(HttpResponse::Ok().json(bundle))
}

// ML Endpoints

#[derive(serde::Deserialize)]
struct MlQuery {
    limit: Option<usize>,
    hours_back: Option<u32>,
}

async fn ml_predict(
    req: HttpRequest,
    ml_client: Option<web::Data<Arc<MlClient>>>,
    query: web::Query<MlQuery>,
) -> Result<HttpResponse, AppError> {
    // Verify authentication
    let _claims = get_claims_from_request(&req)
        .ok_or_else(|| AppError::Unauthorized)?;

    let client = ml_client.ok_or_else(|| {
        AppError::BadRequest("ML service not configured".to_string())
    })?;

    let limit = query.limit.unwrap_or(100).min(1000);
    let hours_back = query.hours_back;

    match client.get_predictions(limit, hours_back).await {
        Ok(predictions) => Ok(HttpResponse::Ok().json(predictions)),
        Err(e) => {
            tracing::error!("ML prediction failed: {}", e);
            Err(AppError::Internal)
        }
    }
}

async fn ml_analysis(
    req: HttpRequest,
    ml_client: Option<web::Data<Arc<MlClient>>>,
    query: web::Query<MlQuery>,
) -> Result<HttpResponse, AppError> {
    // Verify authentication
    let _claims = get_claims_from_request(&req)
        .ok_or_else(|| AppError::Unauthorized)?;

    let client = ml_client.ok_or_else(|| {
        AppError::BadRequest("ML service not configured".to_string())
    })?;

    let limit = query.limit.unwrap_or(1000).min(10000);
    let hours_back = query.hours_back;

    match client.get_analysis(limit, hours_back).await {
        Ok(analysis) => Ok(HttpResponse::Ok().json(analysis)),
        Err(e) => {
            tracing::error!("ML analysis failed: {}", e);
            Err(AppError::Internal)
        }
    }
}

#[derive(serde::Deserialize)]
struct TrainRequest {
    min_samples: Option<usize>,
}

async fn ml_train(
    req: HttpRequest,
    ml_client: Option<web::Data<Arc<MlClient>>>,
    body: web::Json<TrainRequest>,
) -> Result<HttpResponse, AppError> {
    // Verify authentication and require admin role
    let claims = get_claims_from_request(&req)
        .ok_or_else(|| AppError::Unauthorized)?;

    if claims.role != "admin" {
        tracing::warn!("Non-admin user {} attempted to train models", claims.sub);
        return Err(AppError::Unauthorized);
    }

    let client = ml_client.ok_or_else(|| {
        AppError::BadRequest("ML service not configured".to_string())
    })?;

    let min_samples = body.min_samples.unwrap_or(100);

    match client.train_models(min_samples).await {
        Ok(message) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": message
        }))),
        Err(e) => {
            tracing::error!("ML training failed: {}", e);
            Err(AppError::Internal)
        }
    }
}

async fn ml_health(
    req: HttpRequest,
    ml_client: Option<web::Data<Arc<MlClient>>>,
) -> Result<HttpResponse, AppError> {
    // Verify authentication
    let _claims = get_claims_from_request(&req)
        .ok_or_else(|| AppError::Unauthorized)?;

    let client = ml_client.ok_or_else(|| {
        AppError::BadRequest("ML service not configured".to_string())
    })?;

    match client.health_check().await {
        Ok(health) => Ok(HttpResponse::Ok().json(health)),
        Err(e) => {
            tracing::error!("ML health check failed: {}", e);
            Err(AppError::Internal)
        }
    }
}
