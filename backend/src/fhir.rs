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
}