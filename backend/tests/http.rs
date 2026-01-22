use actix_web::{test, web, App};
use std::sync::Arc;
use tokio::sync::Mutex;

use soundsense_backend::auth::{Claims, JwtManager};
use soundsense_backend::domain::models::{SensorReading, SignalCode};
use soundsense_backend::domain::store::AppState;
use soundsense_backend::routes;

/// Helper function to generate JWT token for testing
fn generate_test_token(role: &str) -> String {
    let jwt_manager = JwtManager::new("test-secret-key".to_string());
    let claims = Claims::new(
        "test-user".to_string(),
        role.to_string(),
        None,
        24, // 24 hours
    );
    jwt_manager.generate_token(claims).unwrap()
}

#[actix_web::test]
async fn healthz_works() {
    let state = web::Data::new(Arc::new(Mutex::new(AppState::new_demo())));
    let app = test::init_service(App::new().app_data(state).configure(routes::configure)).await;

    let req = test::TestRequest::get().uri("/healthz").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn ingest_and_query_bundle() {
    std::env::set_var("JWT_SECRET", "test-secret-key");

    let state = web::Data::new(Arc::new(Mutex::new(AppState::new_demo())));
    let app = test::init_service(App::new().app_data(state).configure(routes::configure)).await;

    let token = generate_test_token("user");

    let reading = SensorReading {
        patient_id: "p1".into(),
        device_id: "d1".into(),
        code: SignalCode::Sound,
        value: 200.0,
        unit: "raw".into(),
        ts: chrono::Utc::now(),
    };

    let req = test::TestRequest::post()
        .uri("/api/ingest")
        .insert_header(("authorization", format!("Bearer {}", token)))
        .set_json(&reading)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let req = test::TestRequest::get()
        .uri("/api/fhir/Observation?code=sound&limit=10")
        .insert_header(("authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["resourceType"], "Bundle");
}

#[actix_web::test]
async fn ingest_rejects_empty_patient_id() {
    std::env::set_var("JWT_SECRET", "test-secret-key");

    let state = web::Data::new(Arc::new(Mutex::new(AppState::new_demo())));
    let app = test::init_service(App::new().app_data(state).configure(routes::configure)).await;

    let token = generate_test_token("user");

    let reading = SensorReading {
        patient_id: "".into(),
        device_id: "d1".into(),
        code: SignalCode::Sound,
        value: 200.0,
        unit: "raw".into(),
        ts: chrono::Utc::now(),
    };

    let req = test::TestRequest::post()
        .uri("/api/ingest")
        .insert_header(("authorization", format!("Bearer {}", token)))
        .set_json(&reading)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn ingest_rejects_empty_device_id() {
    std::env::set_var("JWT_SECRET", "test-secret-key");

    let state = web::Data::new(Arc::new(Mutex::new(AppState::new_demo())));
    let app = test::init_service(App::new().app_data(state).configure(routes::configure)).await;

    let token = generate_test_token("user");

    let reading = SensorReading {
        patient_id: "p1".into(),
        device_id: "".into(),
        code: SignalCode::Sound,
        value: 200.0,
        unit: "raw".into(),
        ts: chrono::Utc::now(),
    };

    let req = test::TestRequest::post()
        .uri("/api/ingest")
        .insert_header(("authorization", format!("Bearer {}", token)))
        .set_json(&reading)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn ingest_rejects_nan_value() {
    std::env::set_var("JWT_SECRET", "test-secret-key");

    let state = web::Data::new(Arc::new(Mutex::new(AppState::new_demo())));
    let app = test::init_service(App::new().app_data(state).configure(routes::configure)).await;

    let token = generate_test_token("user");

    let reading = SensorReading {
        patient_id: "p1".into(),
        device_id: "d1".into(),
        code: SignalCode::Sound,
        value: f64::NAN,
        unit: "raw".into(),
        ts: chrono::Utc::now(),
    };

    let req = test::TestRequest::post()
        .uri("/api/ingest")
        .insert_header(("authorization", format!("Bearer {}", token)))
        .set_json(&reading)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn ingest_rejects_infinity_value() {
    std::env::set_var("JWT_SECRET", "test-secret-key");

    let state = web::Data::new(Arc::new(Mutex::new(AppState::new_demo())));
    let app = test::init_service(App::new().app_data(state).configure(routes::configure)).await;

    let token = generate_test_token("user");

    let reading = SensorReading {
        patient_id: "p1".into(),
        device_id: "d1".into(),
        code: SignalCode::Sound,
        value: f64::INFINITY,
        unit: "raw".into(),
        ts: chrono::Utc::now(),
    };

    let req = test::TestRequest::post()
        .uri("/api/ingest")
        .insert_header(("authorization", format!("Bearer {}", token)))
        .set_json(&reading)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn ingest_requires_auth_when_token_set() {
    std::env::set_var("JWT_SECRET", "test-secret-key");

    let state = web::Data::new(Arc::new(Mutex::new(AppState::new_demo())));
    let app = test::init_service(App::new().app_data(state).configure(routes::configure)).await;

    let reading = SensorReading {
        patient_id: "p1".into(),
        device_id: "d1".into(),
        code: SignalCode::Sound,
        value: 200.0,
        unit: "raw".into(),
        ts: chrono::Utc::now(),
    };

    // Request without token should fail
    let req = test::TestRequest::post()
        .uri("/api/ingest")
        .set_json(&reading)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn ingest_accepts_valid_token() {
    std::env::set_var("JWT_SECRET", "test-secret-key");

    let state = web::Data::new(Arc::new(Mutex::new(AppState::new_demo())));
    let app = test::init_service(App::new().app_data(state).configure(routes::configure)).await;

    let token = generate_test_token("user");

    let reading = SensorReading {
        patient_id: "p1".into(),
        device_id: "d1".into(),
        code: SignalCode::Sound,
        value: 200.0,
        unit: "raw".into(),
        ts: chrono::Utc::now(),
    };

    // Request with correct JWT token should succeed
    let req = test::TestRequest::post()
        .uri("/api/ingest")
        .insert_header(("authorization", format!("Bearer {}", token)))
        .set_json(&reading)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn query_with_code_filter() {
    std::env::set_var("JWT_SECRET", "test-secret-key");

    let state = web::Data::new(Arc::new(Mutex::new(AppState::new_demo())));
    let app = test::init_service(App::new().app_data(state).configure(routes::configure)).await;

    let token = generate_test_token("user");

    let reading = SensorReading {
        patient_id: "p1".into(),
        device_id: "d1".into(),
        code: SignalCode::Sound,
        value: 200.0,
        unit: "raw".into(),
        ts: chrono::Utc::now(),
    };

    let req = test::TestRequest::post()
        .uri("/api/ingest")
        .insert_header(("authorization", format!("Bearer {}", token)))
        .set_json(&reading)
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::get()
        .uri("/api/fhir/Observation?code=sound")
        .insert_header(("authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["resourceType"], "Bundle");
    assert_eq!(body["total"], 1);
}

#[actix_web::test]
async fn query_with_limit() {
    std::env::set_var("JWT_SECRET", "test-secret-key");

    let state = web::Data::new(Arc::new(Mutex::new(AppState::new_demo())));
    let app = test::init_service(App::new().app_data(state).configure(routes::configure)).await;

    let token = generate_test_token("user");

    // Insert multiple readings
    for i in 0..5 {
        let reading = SensorReading {
            patient_id: format!("p{}", i),
            device_id: "d1".into(),
            code: SignalCode::Sound,
            value: 200.0 + i as f64,
            unit: "raw".into(),
            ts: chrono::Utc::now(),
        };

        let req = test::TestRequest::post()
            .uri("/api/ingest")
            .insert_header(("authorization", format!("Bearer {}", token)))
            .set_json(&reading)
            .to_request();
        test::call_service(&app, req).await;
    }

    let req = test::TestRequest::get()
        .uri("/api/fhir/Observation?limit=2")
        .insert_header(("authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["resourceType"], "Bundle");
    assert_eq!(body["total"], 2);
}
