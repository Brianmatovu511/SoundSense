use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

use crate::domain::models::SensorReading;
use crate::domain::store::AppState;
use crate::errors::AppError;
use crate::fhir::{FhirBundle, FhirObservation};
use crate::ws::{ws_live, WsHub};

pub fn configure(cfg: &mut web::ServiceConfig) {
    let (tx, _rx) = broadcast::channel::<FhirObservation>(256);

    cfg.app_data(web::Data::new(WsHub { tx }))
        .route("/healthz", web::get().to(healthz))
        .route("/ingest", web::post().to(ingest))
        .route("/fhir/Observation", web::get().to(get_observations))
        .route("/ws/live", web::get().to(ws_live));
}

async fn healthz() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }))
}

fn is_authorized(req: &HttpRequest) -> Result<(), AppError> {
    let token = std::env::var("INGEST_TOKEN").unwrap_or_default();
    if token.trim().is_empty() {
        return Ok(());
    }

    let hdr = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let expected = format!("Bearer {}", token.trim());

    if hdr == expected {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

async fn ingest(
    req: HttpRequest,
    state: web::Data<Arc<Mutex<AppState>>>,
    hub: web::Data<WsHub>,
    payload: web::Json<SensorReading>,
) -> Result<HttpResponse, AppError> {
    is_authorized(&req)?;

    // ✅ Normalize code + validate
    let  reading = payload.into_inner();
    reading.validate().map_err(AppError::BadRequest)?;

    // ✅ Convert to FHIR Observation
    let obs = FhirObservation::from_reading(reading.clone());

    // ✅ Store reading
    {
        let mut st = state.lock().map_err(|_| AppError::Internal)?;
        st.push(reading);
    }

    // ✅ Push to WebSocket subscribers
    let _ = hub.tx.send(obs.clone());

    Ok(HttpResponse::Ok().json(obs))
}

#[derive(serde::Deserialize)]
struct ObsQuery {
    code: Option<String>,
    limit: Option<usize>,
}

async fn get_observations(
    state: web::Data<Arc<Mutex<AppState>>>,
    q: web::Query<ObsQuery>,
) -> Result<HttpResponse, AppError> {
    let limit = q.limit.unwrap_or(100).min(500);

    let st = state.lock().map_err(|_| AppError::Internal)?;
    let bundle: FhirBundle = st.bundle(limit);

    if let Some(code) = &q.code {
        let code = code.as_str();
        let filtered: Vec<_> = bundle
            .entry
            .into_iter()
            .filter(|e| e.resource.code.coding.iter().any(|c| c.code == code))
            .collect();

        let out = crate::fhir::FhirBundle {
            resource_type: "Bundle",
            r#type: "collection",
            total: filtered.len(),
            entry: filtered,
        };

        return Ok(HttpResponse::Ok().json(out));
    }

    Ok(HttpResponse::Ok().json(bundle))
}