use actix_web::{test, web, App};
use std::sync::{Arc, Mutex};

use soundsense_backend::domain::models::{SensorReading, SignalCode};
use soundsense_backend::domain::store::AppState;
use soundsense_backend::routes;

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
    std::env::set_var("INGEST_TOKEN", "");

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

    let req = test::TestRequest::post().uri("/ingest").set_json(&reading).to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let req = test::TestRequest::get()
        .uri("/fhir/Observation?code=sound&limit=10")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["resourceType"], "Bundle");
}