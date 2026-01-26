/// HIPAA Compliance Audit Logging Module
/// 
/// Tracks all access to Protected Health Information (PHI) and system actions
/// for compliance with HIPAA Security Rule audit requirements.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditAction {
    Create,
    Read,
    Update,
    Delete,
    Login,
    Logout,
    AccessDenied,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditAction::Create => write!(f, "CREATE"),
            AuditAction::Read => write!(f, "READ"),
            AuditAction::Update => write!(f, "UPDATE"),
            AuditAction::Delete => write!(f, "DELETE"),
            AuditAction::Login => write!(f, "LOGIN"),
            AuditAction::Logout => write!(f, "LOGOUT"),
            AuditAction::AccessDenied => write!(f, "ACCESS_DENIED"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub user_id: Option<String>,
    pub user_role: Option<String>,
    pub action: AuditAction,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub patient_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_path: Option<String>,
    pub status_code: Option<i32>,
    pub error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl AuditLogEntry {
    pub fn new(action: AuditAction, resource_type: String) -> Self {
        Self {
            user_id: None,
            user_role: None,
            action,
            resource_type,
            resource_id: None,
            patient_id: None,
            ip_address: None,
            user_agent: None,
            request_path: None,
            status_code: None,
            error_message: None,
            metadata: None,
        }
    }

    pub fn with_user(mut self, user_id: String, user_role: String) -> Self {
        self.user_id = Some(user_id);
        self.user_role = Some(user_role);
        self
    }

    pub fn with_resource_id(mut self, resource_id: String) -> Self {
        self.resource_id = Some(resource_id);
        self
    }

    pub fn with_patient_id(mut self, patient_id: String) -> Self {
        self.patient_id = Some(patient_id);
        self
    }

    pub fn with_request_context(
        mut self,
        ip: Option<String>,
        user_agent: Option<String>,
        path: Option<String>,
    ) -> Self {
        self.ip_address = ip;
        self.user_agent = user_agent;
        self.request_path = path;
        self
    }

    pub fn with_status_code(mut self, status_code: i32) -> Self {
        self.status_code = Some(status_code);
        self
    }

    pub fn with_error(mut self, error_message: String) -> Self {
        self.error_message = Some(error_message);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Log this audit entry to the database
    pub async fn log(&self, pool: &PgPool) -> Result<Uuid, sqlx::Error> {
        // Convert IP address to string for storage (PostgreSQL INET type)
        let ip_str = self.ip_address.as_ref()
            .and_then(|ip| ip.parse::<std::net::IpAddr>().ok())
            .map(|addr| addr.to_string());

        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO audit_logs (
                user_id,
                user_role,
                action,
                resource_type,
                resource_id,
                patient_id,
                ip_address,
                user_agent,
                request_path,
                status_code,
                error_message,
                metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7::inet, $8, $9, $10, $11, $12)
            RETURNING id
            "#
        )
        .bind(&self.user_id)
        .bind(&self.user_role)
        .bind(self.action.to_string())
        .bind(&self.resource_type)
        .bind(&self.resource_id)
        .bind(&self.patient_id)
        .bind(ip_str)
        .bind(&self.user_agent)
        .bind(&self.request_path)
        .bind(self.status_code)
        .bind(&self.error_message)
        .bind(&self.metadata)
        .fetch_one(pool)
        .await?;

        tracing::debug!(
            audit_id = %id,
            user_id = ?self.user_id,
            action = %self.action,
            resource_type = %self.resource_type,
            patient_id = ?self.patient_id,
            "Audit event logged"
        );

        Ok(id)
    }
}

/// Audit logger for HIPAA compliance
pub struct AuditLogger {
    pool: PgPool,
}

impl AuditLogger {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Log an audit event
    pub async fn log(&self, entry: AuditLogEntry) -> Result<Uuid, sqlx::Error> {
        entry.log(&self.pool).await
    }

    /// Query audit logs for a specific patient (for patient access reports)
    pub async fn get_patient_access_log(
        &self,
        patient_id: &str,
        limit: i64,
    ) -> Result<Vec<AuditLogSummary>, sqlx::Error> {
        let logs = sqlx::query_as::<_, AuditLogSummary>(
            r#"
            SELECT 
                id,
                timestamp,
                user_id,
                user_role,
                action,
                resource_type,
                patient_id,
                status_code,
                CASE 
                    WHEN error_message IS NOT NULL THEN 'Error occurred'
                    ELSE 'Success'
                END as outcome
            FROM audit_log_summary
            WHERE patient_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
            "#
        )
        .bind(patient_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    /// Query audit logs for a specific user
    pub async fn get_user_activity_log(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<AuditLogSummary>, sqlx::Error> {
        let logs = sqlx::query_as::<_, AuditLogSummary>(
            r#"
            SELECT 
                id,
                timestamp,
                user_id,
                user_role,
                action,
                resource_type,
                patient_id,
                status_code,
                CASE 
                    WHEN error_message IS NOT NULL THEN 'Error occurred'
                    ELSE 'Success'
                END as outcome
            FROM audit_log_summary
            WHERE user_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
            "#
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLogSummary {
    pub id: Uuid,
    pub timestamp: chrono::DateTime<Utc>,
    pub user_id: Option<String>,
    pub user_role: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub patient_id: Option<String>,
    pub status_code: Option<i32>,
    pub outcome: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_entry_builder() {
        let entry = AuditLogEntry::new(AuditAction::Read, "Observation".to_string())
            .with_user("user123".to_string(), "admin".to_string())
            .with_patient_id("patient456".to_string())
            .with_status_code(200);

        assert_eq!(entry.action.to_string(), "READ");
        assert_eq!(entry.user_id, Some("user123".to_string()));
        assert_eq!(entry.patient_id, Some("patient456".to_string()));
        assert_eq!(entry.status_code, Some(200));
    }

    #[test]
    fn test_audit_action_display() {
        assert_eq!(AuditAction::Create.to_string(), "CREATE");
        assert_eq!(AuditAction::Read.to_string(), "READ");
        assert_eq!(AuditAction::AccessDenied.to_string(), "ACCESS_DENIED");
    }
}
