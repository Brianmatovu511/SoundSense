use anyhow::{Context, Result};
use chrono::Utc;
use regex::Regex;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::domain::models::{SensorReading, SignalCode};

pub fn run_serial_to_ingest(
    port_name: &str,
    baud: u32,
    ingest_url: &str,   // e.g. "http://127.0.0.1:8080/ingest"
    token: Option<&str>,
) -> Result<()> {
    let port = serialport::new(port_name, baud)
        .timeout(Duration::from_millis(1000))
        .open()
        .with_context(|| format!("Failed to open serial port {}", port_name))?;

    let mut reader = BufReader::new(port);

    // Accepts: SOUND:123
    let re = Regex::new(r"^SOUND:(\d+)\s*$")?;

    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            continue;
        }

        let trimmed = line.trim();
        if let Some(caps) = re.captures(trimmed) {
            let v: f64 = caps[1].parse::<u32>().unwrap_or(0) as f64;

            let reading = SensorReading {
                patient_id: "demo-patient-1".into(),
                device_id: format!("arduino-{}", port_name),
                code: SignalCode::Sound,
                value: v,
                unit: "raw".into(),
                ts: Utc::now(),
            };

            // Send to backend /ingest
            if let Err(e) = http_post_json(ingest_url, &reading, token) {
                eprintln!("serial->ingest POST failed: {e:?}");
            }
        }
    }
}

/// Tiny HTTP POST (no reqwest needed)
fn http_post_json(url: &str, reading: &SensorReading, token: Option<&str>) -> Result<()> {
    // Parse very simply: http://host:port/path
    let url = url.strip_prefix("http://").context("Only http:// URLs supported")?;
    let (host_port, path) = url.split_once('/').unwrap_or((url, ""));
    let path = format!("/{}", path);

    let mut host = host_port.to_string();
    let mut port = 80u16;

    if let Some((h, p)) = host_port.split_once(':') {
        host = h.to_string();
        port = p.parse::<u16>().unwrap_or(80);
    }

    let body = serde_json::to_string(reading)?;
    let mut headers = String::new();

    headers.push_str(&format!("POST {} HTTP/1.1\r\n", path));
    headers.push_str(&format!("Host: {}\r\n", host));
    headers.push_str("Content-Type: application/json\r\n");
    headers.push_str(&format!("Content-Length: {}\r\n", body.len()));
    headers.push_str("Connection: close\r\n");

    if let Some(t) = token {
        let t = t.trim();
        if !t.is_empty() {
            headers.push_str(&format!("Authorization: Bearer {}\r\n", t));
        }
    }

    headers.push_str("\r\n");

    let mut stream = TcpStream::connect((host.as_str(), port))
        .with_context(|| format!("TCP connect failed to {}:{}", host, port))?;

    stream.write_all(headers.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()?;

    // Read response just to complete request (optional)
    let mut resp = String::new();
    let mut buf = [0u8; 1024];
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 {
            break;
        }
        resp.push_str(&String::from_utf8_lossy(&buf[..n]));
    }

    // Basic status check
    if !resp.starts_with("HTTP/1.1 200") && !resp.starts_with("HTTP/1.1 201") {
        // Print first line for debugging
        let first_line = resp.lines().next().unwrap_or("<no response>");
        anyhow::bail!("unexpected response: {}", first_line);
    }

    Ok(())
}