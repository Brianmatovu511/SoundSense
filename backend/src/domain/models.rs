use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalCode {
    // Canonical serialized value
    #[serde(rename = "sound")]
    // Accept these incoming values too
    #[serde(alias = "SoundLevel")]
    #[serde(alias = "sound_level")]
    #[serde(alias = "SOUND_LEVEL")]
    #[serde(alias = "Sound")]
    Sound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub patient_id: String,
    pub device_id: String,
    pub code: SignalCode,
    pub value: f64,
    pub unit: String,
    pub ts: DateTime<Utc>,
}

impl SensorReading {
    pub fn validate(&self) -> Result<(), String> {
        if self.patient_id.trim().is_empty() {
            return Err("patient_id required".into());
        }
        if self.device_id.trim().is_empty() {
            return Err("device_id required".into());
        }
        if !self.value.is_finite() {
            return Err("value must be finite".into());
        }
        Ok(())
    }
}
