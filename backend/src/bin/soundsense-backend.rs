use actix_web::{middleware, web, App, HttpServer};
use actix_cors::Cors;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use soundsense_backend::db::Database;
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
    let ingest_url =
        std::env::var("INGEST_URL").unwrap_or_else(|_| format!("http://127.0.0.1:{}/ingest", port));

    // Initialize database connection if DATABASE_URL is provided
    let state = if let Ok(database_url) = std::env::var("DATABASE_URL") {
        tracing::info!("Connecting to database...");
        
        match sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(3))
            .connect(&database_url)
            .await
        {
            Ok(pool) => {
                tracing::info!("Database connected successfully");
                
                // Run migrations
                match sqlx::migrate!("./migrations").run(&pool).await {
                    Ok(_) => {
                        tracing::info!("Database migrations completed successfully");
                        let db = Database::new(pool);
                        web::Data::new(Arc::new(Mutex::new(AppState::with_database(db))))
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to run database migrations");
                        tracing::warn!("Falling back to in-memory storage");
                        web::Data::new(Arc::new(Mutex::new(AppState::new_demo())))
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to connect to database");
                tracing::warn!("Falling back to in-memory storage");
                web::Data::new(Arc::new(Mutex::new(AppState::new_demo())))
            }
        }
    } else {
        tracing::info!("DATABASE_URL not set, using in-memory storage only");
        web::Data::new(Arc::new(Mutex::new(AppState::new_demo())))
    };

    tracing::info!(%host, %port, "starting backend");

    // Start serial ingest thread (only if serial provided)
    if let Some(serial_port) = serial_port.clone() {
        let token = token.clone();
        let ingest_url = ingest_url.clone();

        std::thread::spawn(move || {
            eprintln!("Serial ingest starting on {serial_port} -> {ingest_url}");
            if let Err(e) = serial_ingest::run_serial_to_ingest(
                &serial_port,
                9600,
                &ingest_url,
                token.as_deref(),
            ) {
                eprintln!("serial ingest error: {e:?}");
            }
        });
    } else {
        eprintln!("No serial port provided. Run with: --serial COM6  (or set SERIAL_PORT=COM6)");
    }

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(state.clone())
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .configure(routes::configure)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
