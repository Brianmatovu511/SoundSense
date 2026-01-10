use actix_web::{middleware, web, App, HttpServer};
use std::sync::{Arc, Mutex};

use soundsense_backend::domain::store::AppState;
use soundsense_backend::{routes, serial_ingest, telemetry::init_tracing};

fn get_arg_value(flag: &str) -> Option<String> {
    let mut args = std::env::args();
    while let Some(a) = args.next() {
        if a == flag {
            return args.next();
        }
    }
    None
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_tracing();

    // host/port for the HTTP server binding
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(8080);

    // Accept serial from either:
    // 1) CLI: --serial COM6
    // 2) ENV: SERIAL_PORT=COM6
    let serial_port = get_arg_value("--serial").or_else(|| std::env::var("SERIAL_PORT").ok());

    // token optional
    let token = std::env::var("INGEST_TOKEN").ok();

    // Where the serial thread should POST readings to.
    // Default is localhost (best for local dev).
    // In Docker you can set: INGEST_URL=http://backend:8080/ingest
    let ingest_url = std::env::var("INGEST_URL")
        .unwrap_or_else(|_| format!("http://127.0.0.1:{}/ingest", port));

    // shared state
    let state = web::Data::new(Arc::new(Mutex::new(AppState::new_demo())));

    tracing::info!(%host, %port, "starting backend");

    // Start serial ingest thread (only if serial provided)
    if let Some(serial_port) = serial_port.clone() {
        let token = token.clone();
        let ingest_url = ingest_url.clone();

        std::thread::spawn(move || {
            eprintln!("Serial ingest starting on {serial_port} -> {ingest_url}");
            if let Err(e) =
                serial_ingest::run_serial_to_ingest(&serial_port, 9600, &ingest_url, token.as_deref())
            {
                eprintln!("serial ingest error: {e:?}");
            }
        });
    } else {
        eprintln!("No serial port provided. Run with: --serial COM6  (or set SERIAL_PORT=COM6)");
    }

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(middleware::Logger::default())
            .configure(routes::configure)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}