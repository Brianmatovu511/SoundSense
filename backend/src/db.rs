use crate::domain::models::{SensorReading, SignalCode};
use crate::errors::AppError;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgPool;
use sqlx::Row;
use uuid::Uuid;

/// Database wrapper for PostgreSQL operations
#[derive(Debug, Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database instance from a connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a reference to the connection pool (for audit logging)
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Insert a sensor reading into the database
    pub async fn insert_reading(&self, reading: &SensorReading) -> Result<Uuid, AppError> {
        tracing::debug!(
            patient_id = %reading.patient_id,
            device_id = %reading.device_id,
            value = reading.value,
            "Inserting sensor reading"
        );

        let code_str = match reading.code {
            SignalCode::Sound => "sound",
        };

        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO sensor_readings (patient_id, device_id, code, value, unit, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
        )
        .bind(&reading.patient_id)
        .bind(&reading.device_id)
        .bind(code_str)
        .bind(reading.value)
        .bind(&reading.unit)
        .bind(reading.ts)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to insert sensor reading");
            AppError::Internal
        })?;

        tracing::debug!(id = %id, "Successfully inserted sensor reading");
        Ok(id)
    }

    /// Get recent sensor readings with optional code filter
    pub async fn get_recent_readings(
        &self,
        limit: usize,
        code_filter: Option<&str>,
    ) -> Result<Vec<SensorReading>, AppError> {
        tracing::debug!(limit = limit, code_filter = ?code_filter, "Fetching recent readings");

        let rows = if let Some(code) = code_filter {
            sqlx::query(
                r#"
                SELECT patient_id, device_id, code, value, unit, timestamp
                FROM sensor_readings
                WHERE code = $1
                ORDER BY timestamp DESC
                LIMIT $2
                "#,
            )
            .bind(code)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                r#"
                SELECT patient_id, device_id, code, value, unit, timestamp
                FROM sensor_readings
                ORDER BY timestamp DESC
                LIMIT $1
                "#,
            )
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch sensor readings");
            AppError::Internal
        })?;

        let readings: Vec<SensorReading> = rows
            .into_iter()
            .filter_map(|row| {
                let patient_id: String = row.get("patient_id");
                let device_id: String = row.get("device_id");
                let code_str: String = row.get("code");
                let value: f64 = row.get("value");
                let unit: String = row.get("unit");
                let ts: DateTime<Utc> = row.get("timestamp");

                // Convert string back to enum
                let code = match code_str.as_str() {
                    "sound" => SignalCode::Sound,
                    _ => {
                        tracing::warn!(code = %code_str, "Unknown code in database");
                        return None;
                    }
                };

                Some(SensorReading {
                    patient_id,
                    device_id,
                    code,
                    value,
                    unit,
                    ts,
                })
            })
            .collect();

        tracing::debug!(count = readings.len(), "Successfully fetched sensor readings");
        Ok(readings)
    }

    /// Health check - verify database connection is alive
    pub async fn health_check(&self) -> Result<(), AppError> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Database health check failed");
                AppError::Internal
            })?;

        Ok(())
    }
}
