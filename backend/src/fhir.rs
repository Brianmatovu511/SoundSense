use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::models::{SensorReading, SignalCode};

#[derive(Debug, Serialize, Clone)]
pub struct FhirCoding {
    pub system: &'static str,
    pub code: &'static str,
    pub display: &'static str,
}

#[derive(Debug, Serialize, Clone)]
pub struct FhirCode {
    pub coding: Vec<FhirCoding>,
    pub text: &'static str,
}

#[derive(Debug, Serialize, Clone)]
pub struct FhirQuantity {
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct FhirReference {
    pub reference: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct FhirObservation {
    #[serde(rename = "resourceType")]
    pub resource_type: &'static str,
    pub id: String,
    pub status: &'static str,
    pub code: FhirCode,
    pub subject: FhirReference,
    #[serde(rename = "effectiveDateTime")]
    pub effective_date_time: DateTime<Utc>,
    #[serde(rename = "valueQuantity")]
    pub value_quantity: FhirQuantity,
}

impl FhirObservation {
    pub fn from_reading(r: SensorReading) -> Self {
        let (code, display) = match r.code {
            SignalCode::Sound => ("sound", "Sound Level"),
        };

        Self {
            resource_type: "Observation",
            id: Uuid::new_v4().to_string(),
            status: "final",
            code: FhirCode {
                coding: vec![FhirCoding {
                    system: "http://loinc.org",
                    code,
                    display,
                }],
                text: display,
            },
            subject: FhirReference {
                reference: format!("Patient/{}", r.patient_id),
            },
            effective_date_time: r.ts,
            value_quantity: FhirQuantity {
                value: r.value,
                unit: r.unit,
            },
        }
    }

    /// Validate FHIR Observation against FHIR R4 schema
    pub fn validate(&self) -> Result<(), String> {
        // Resource type must be "Observation"
        if self.resource_type != "Observation" {
            return Err("resourceType must be 'Observation'".into());
        }

        // ID must be present and valid UUID format
        if self.id.is_empty() {
            return Err("Observation ID is required".into());
        }
        Uuid::parse_str(&self.id).map_err(|_| "ID must be a valid UUID")?;

        // Status must be one of: registered, preliminary, final, amended, corrected, cancelled, entered-in-error, unknown
        let valid_statuses = ["registered", "preliminary", "final", "amended", "corrected", "cancelled", "entered-in-error", "unknown"];
        if !valid_statuses.contains(&self.status) {
            return Err(format!("Invalid status '{}'. Must be one of: {}", self.status, valid_statuses.join(", ")));
        }

        // Code must have at least one coding
        if self.code.coding.is_empty() {
            return Err("Observation must have at least one coding".into());
        }

        // Validate coding system (must be a valid URI)
        for coding in &self.code.coding {
            if coding.system.is_empty() {
                return Err("Coding system is required".into());
            }
            if !coding.system.starts_with("http://") && !coding.system.starts_with("https://") {
                return Err(format!("Coding system must be a valid URI: {}", coding.system));
            }
            if coding.code.is_empty() {
                return Err("Coding code is required".into());
            }
        }

        // Subject reference must be present and follow pattern
        if self.subject.reference.is_empty() {
            return Err("Subject reference is required".into());
        }
        if !self.subject.reference.contains('/') {
            return Err("Subject reference must follow format: ResourceType/id".into());
        }

        // Value must be finite
        if !self.value_quantity.value.is_finite() {
            return Err("Value must be a finite number".into());
        }

        // Unit must be present
        if self.value_quantity.unit.is_empty() {
            return Err("Value unit is required".into());
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct FhirBundleEntry {
    pub resource: FhirObservation,
}

#[derive(Debug, Serialize, Clone)]
pub struct FhirBundle {
    #[serde(rename = "resourceType")]
    pub resource_type: &'static str,
    pub r#type: &'static str,
    pub total: usize,
    pub entry: Vec<FhirBundleEntry>,
}

impl FhirBundle {
    pub fn from_obs(obs: Vec<FhirObservation>) -> Self {
        let total = obs.len();
        Self {
            resource_type: "Bundle",
            r#type: "collection",
            total,
            entry: obs
                .into_iter()
                .map(|o| FhirBundleEntry { resource: o })
                .collect(),
        }
    }

    /// Validate FHIR Bundle against FHIR R4 schema
    pub fn validate(&self) -> Result<(), String> {
        // Resource type must be "Bundle"
        if self.resource_type != "Bundle" {
            return Err("resourceType must be 'Bundle'".into());
        }

        // Type must be one of: document, message, transaction, transaction-response, batch, batch-response, history, searchset, collection
        let valid_types = ["document", "message", "transaction", "transaction-response", "batch", "batch-response", "history", "searchset", "collection"];
        if !valid_types.contains(&self.r#type) {
            return Err(format!("Invalid Bundle type '{}'. Must be one of: {}", self.r#type, valid_types.join(", ")));
        }

        // Total must match entry count
        if self.total != self.entry.len() {
            return Err(format!("Bundle total ({}) does not match entry count ({})", self.total, self.entry.len()));
        }

        // Validate all observations in the bundle
        for (idx, entry) in self.entry.iter().enumerate() {
            entry.resource.validate()
                .map_err(|e| format!("Observation at index {} is invalid: {}", idx, e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_valid_observation() {
        let obs = FhirObservation {
            resource_type: "Observation",
            id: Uuid::new_v4().to_string(),
            status: "final",
            code: FhirCode {
                coding: vec![FhirCoding {
                    system: "http://loinc.org",
                    code: "sound",
                    display: "Sound Level",
                }],
                text: "Sound Level",
            },
            subject: FhirReference {
                reference: "Patient/p1".into(),
            },
            effective_date_time: Utc::now(),
            value_quantity: FhirQuantity {
                value: 200.0,
                unit: "raw".into(),
            },
        };

        assert!(obs.validate().is_ok());
    }

    #[test]
    fn test_invalid_status() {
        let obs = FhirObservation {
            resource_type: "Observation",
            id: Uuid::new_v4().to_string(),
            status: "invalid_status",
            code: FhirCode {
                coding: vec![FhirCoding {
                    system: "http://loinc.org",
                    code: "sound",
                    display: "Sound Level",
                }],
                text: "Sound Level",
            },
            subject: FhirReference {
                reference: "Patient/p1".into(),
            },
            effective_date_time: Utc::now(),
            value_quantity: FhirQuantity {
                value: 200.0,
                unit: "raw".into(),
            },
        };

        assert!(obs.validate().is_err());
    }

    #[test]
    fn test_invalid_value() {
        let obs = FhirObservation {
            resource_type: "Observation",
            id: Uuid::new_v4().to_string(),
            status: "final",
            code: FhirCode {
                coding: vec![FhirCoding {
                    system: "http://loinc.org",
                    code: "sound",
                    display: "Sound Level",
                }],
                text: "Sound Level",
            },
            subject: FhirReference {
                reference: "Patient/p1".into(),
            },
            effective_date_time: Utc::now(),
            value_quantity: FhirQuantity {
                value: f64::NAN,
                unit: "raw".into(),
            },
        };

        assert!(obs.validate().is_err());
    }
}
