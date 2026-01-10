use crate::domain::models::SensorReading;
use crate::fhir::{FhirBundle, FhirObservation};
use std::collections::VecDeque;

#[derive(Debug)]
pub struct AppState {
    readings: VecDeque<SensorReading>,
    max: usize,
}

impl AppState {
    pub fn new_demo() -> Self {
        Self {
            readings: VecDeque::new(),
            max: 500,
        }
    }

    pub fn push(&mut self, r: SensorReading) {
        if self.readings.len() >= self.max {
            self.readings.pop_front();
        }
        self.readings.push_back(r);
    }

    pub fn recent_observations(&self, limit: usize) -> Vec<FhirObservation> {
        let n = limit.min(self.readings.len());
        self.readings
            .iter()
            .rev()
            .take(n)
            .cloned()
            .map(FhirObservation::from_reading)
            .collect()
    }

    pub fn bundle(&self, limit: usize) -> FhirBundle {
        FhirBundle::from_obs(self.recent_observations(limit))
    }
}
