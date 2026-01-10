use anyhow::{Context, Result};
use rand::Rng;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

use soundsense_backend::domain::models::{SensorReading, SignalCode};

#[tokio::main]
async fn main() -> Result<()> {
    // In Docker: BASE_URL should be "http://backend:8080"
    // Locally:  "http://127.0.0.1:8080"
    let base = std::env::var("BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    let token = std::env::var("INGEST_TOKEN").ok();

    eprintln!("simulator starting. BASE_URL={base}");

    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .context("failed to build reqwest client")?;

    loop {
        let mut rng = rand::thread_rng();
        let now = chrono::Utc::now();

        let reading = SensorReading {
            patient_id: "demo-patient-1".into(),
            device_id: "simulator-1".into(),
            code: SignalCode::Sound, // keep canonical
            value: rng.gen_range(150.0..260.0),
            unit: "au".into(),
            ts: now,
        };

        let url = format!("{}/ingest", base);

        let mut req = client.post(&url).json(&reading);
        if let Some(t) = token.as_deref() {
            if !t.trim().is_empty() {
                req = req.header("authorization", format!("Bearer {}", t.trim()));
            }
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    eprintln!("sent ok: value={} status={}", reading.value, status);
                } else {
                    let body = resp.text().await.unwrap_or_default();
                    eprintln!("sent failed: status={} body={}", status, body);
                }
            }
            Err(e) => {
                eprintln!("send error: {e:?} (will retry)");
            }
        }

        sleep(Duration::from_millis(300)).await;
    }
}
