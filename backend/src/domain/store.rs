use crate::audit::{AuditAction, AuditLogEntry};
use crate::auth::Claims;
use crate::db::Database;
use crate::domain::models::SensorReading;
use crate::errors::AppError;
use crate::fhir::{FhirBundle, FhirObservation};
use std::collections::VecDeque;

#[derive(Debug)]
pub struct AppState {
    readings: VecDeque<SensorReading>,
    max: usize,
    db: Option<Database>,
}

impl AppState {
    pub fn new_demo() -> Self {
        Self {
            readings: VecDeque::new(),
            max: 500,
            db: None,
        }
    }

    pub fn with_database(db: Database) -> Self {
        Self {
            readings: VecDeque::new(),
            max: 500,
            db: Some(db),
        }
    }

    /// Push a sensor reading to both database (if available) and in-memory storage
    /// Logs audit trail if user claims provided
    pub async fn push(&mut self, r: SensorReading, claims: Option<&Claims>) -> Result<(), AppError> {
        // Store in database if available
        if let Some(db) = &self.db {
            match db.insert_reading(&r).await {
                Ok(id) => {
                    tracing::debug!(id = %id, "Stored reading in database");
                    
                    // Log audit event for HIPAA compliance
                    if let Some(user_claims) = claims {
                        let audit_entry = AuditLogEntry::new(
                            AuditAction::Create,
                            "SensorReading".to_string(),
                        )
                        .with_user(user_claims.sub.clone(), user_claims.role.clone())
                        .with_resource_id(id.to_string())
                        .with_patient_id(r.patient_id.clone())
                        .with_status_code(200);

                        if let Err(e) = audit_entry.log(db.pool()).await {
                            tracing::warn!(error = ?e, "Failed to log audit event");
                            // Don't fail the request if audit logging fails
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(error = ?e, "Failed to store reading in database, continuing with in-memory only");
                    // Continue execution - fallback to in-memory
                }
            }
        }

        // Always store in memory for WebSocket streaming
        if self.readings.len() >= self.max {
            self.readings.pop_front();
        }
        self.readings.push_back(r);

        Ok(())
    }

    /// Get recent observations, preferring database if available, fallback to in-memory
    pub async fn recent_observations(
        &self,
        limit: usize,
        code_filter: Option<&str>,
    ) -> Result<Vec<FhirObservation>, AppError> {
        // Try database first
        if let Some(db) = &self.db {
            match db.get_recent_readings(limit, code_filter).await {
                Ok(readings) => {
                    return Ok(readings
                        .into_iter()
                        .map(FhirObservation::from_reading)
                        .collect());
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "Failed to query database, falling back to in-memory");
                    // Fall through to in-memory fallback
                }
            }
        }

        // Fallback to in-memory
        let n = limit.min(self.readings.len());
        let observations: Vec<_> = self
            .readings
            .iter()
            .rev()
            .take(n)
            .cloned()
            .map(FhirObservation::from_reading)
            .collect();

        Ok(observations)
    }

    pub async fn bundle(&self, limit: usize, code_filter: Option<&str>) -> Result<FhirBundle, AppError> {
        let observations = self.recent_observations(limit, code_filter).await?;
        Ok(FhirBundle::from_obs(observations))
    }

    pub async fn health_check(&self) -> Result<bool, AppError> {
        if let Some(db) = &self.db {
            db.health_check().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn has_database(&self) -> bool {
        self.db.is_some()
    }

    pub async fn bundle_by_code(&self, limit: usize, code: &str) -> Result<FhirBundle, AppError> {
        let observations = self.recent_observations(limit, Some(code)).await?;
        Ok(FhirBundle::from_obs(observations))
    }
}
