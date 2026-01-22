/// ML Service Client
/// 
/// Communicates with Python ML service for predictions and analysis.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MlClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
pub struct PredictionRequest {
    pub limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hours_back: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PredictionResponse {
    pub success: bool,
    pub total_readings: usize,
    pub predictions: Vec<Prediction>,
    pub summary: PredictionSummary,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prediction {
    pub value: f64,
    pub timestamp: String,
    pub category_rule: String,
    #[serde(default)]
    pub category_ml: Option<String>,
    #[serde(default)]
    pub category_confidence: Option<f64>,
    #[serde(default)]
    pub is_anomaly: bool,
    #[serde(default)]
    pub anomaly_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PredictionSummary {
    pub total_readings: usize,
    pub avg_value: f64,
    pub max_value: f64,
    pub min_value: f64,
    pub anomaly_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResponse {
    pub success: bool,
    pub analysis: Analysis,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Analysis {
    pub total_readings: usize,
    pub avg_level: f64,
    pub std_level: f64,
    pub min_level: f64,
    pub max_level: f64,
    #[serde(default)]
    pub anomaly_count: usize,
    #[serde(default)]
    pub anomaly_percentage: f64,
    #[serde(default)]
    pub peak_hour: i32,
    #[serde(default)]
    pub quietest_hour: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub database_connected: bool,
    pub classifier_loaded: bool,
    pub anomaly_detector_loaded: bool,
}

impl MlClient {
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        Self { base_url, client }
    }

    /// Get ML predictions for recent readings
    pub async fn get_predictions(
        &self,
        limit: usize,
        hours_back: Option<u32>,
    ) -> Result<PredictionResponse, String> {
        let url = format!("{}/predict", self.base_url);
        
        let request_body = PredictionRequest { limit, hours_back };

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("ML service request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("ML service returned status: {}", response.status()));
        }

        response
            .json::<PredictionResponse>()
            .await
            .map_err(|e| format!("Failed to parse ML response: {}", e))
    }

    /// Get pattern analysis
    pub async fn get_analysis(
        &self,
        limit: usize,
        hours_back: Option<u32>,
    ) -> Result<AnalysisResponse, String> {
        let mut url = format!("{}/analysis?limit={}", self.base_url, limit);
        
        if let Some(hours) = hours_back {
            url.push_str(&format!("&hours_back={}", hours));
        }

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("ML service request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("ML service returned status: {}", response.status()));
        }

        response
            .json::<AnalysisResponse>()
            .await
            .map_err(|e| format!("Failed to parse ML response: {}", e))
    }

    /// Trigger model training
    pub async fn train_models(&self, min_samples: usize) -> Result<String, String> {
        let url = format!("{}/train", self.base_url);
        
        let request_body = serde_json::json!({
            "min_samples": min_samples
        });

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("ML service request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("ML service returned status: {}", response.status()));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse ML response: {}", e))?;

        Ok(body["message"].as_str().unwrap_or("Training started").to_string())
    }

    /// Check ML service health
    pub async fn health_check(&self) -> Result<HealthResponse, String> {
        let url = format!("{}/health", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("ML service request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("ML service returned status: {}", response.status()));
        }

        response
            .json::<HealthResponse>()
            .await
            .map_err(|e| format!("Failed to parse ML response: {}", e))
    }
}
