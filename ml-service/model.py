"""
Sound Level Classification Model

Classifies sound levels into categories:
- Quiet (0-187)
- Normal (187-300)
- Moderate (300-500)
- Loud (500-700)
- Concerning (700+)

Also detects anomalies and patterns.
"""
import numpy as np
import pandas as pd
import joblib
from sklearn.ensemble import IsolationForest, RandomForestClassifier
from sklearn.preprocessing import StandardScaler
from sklearn.model_selection import train_test_split
from typing import Optional, Tuple
from datetime import datetime, timedelta
from pathlib import Path
from loguru import logger


class SoundClassifier:
    """
    Multi-level classifier for sound sensor data.
    
    Features extracted:
    - Raw sound value
    - Rolling mean (5 readings)
    - Rolling std (5 readings)
    - Rate of change
    - Time of day features (hour, day_of_week)
    """
    
    def __init__(self, model_dir: Path = Path("models")):
        if isinstance(model_dir, str):
            self.model_dir = Path(model_dir)
        else:
            self.model_dir = model_dir
        self.model_dir.mkdir(exist_ok=True)
        
        self.classifier = None
        self.anomaly_detector = None
        self.scaler = StandardScaler()
        
        # Thresholds for rule-based classification
        self.thresholds = {
            'quiet': 187,
            'normal': 300,
            'moderate': 500,
            'loud': 700
        }
        
    def extract_features(self, df: pd.DataFrame) -> np.ndarray:
        """
        Extract features from sound readings.
        
        Args:
            df: DataFrame with columns: value, timestamp
            
        Returns:
            Feature matrix
        """
        features = []
        
        # Ensure timestamp is datetime
        if not pd.api.types.is_datetime64_any_dtype(df['timestamp']):
            df['timestamp'] = pd.to_datetime(df['timestamp'])
        
        # Sort by timestamp
        df = df.sort_values('timestamp').reset_index(drop=True)
        
        for idx in range(len(df)):
            row_features = []
            
            # Raw value
            row_features.append(df.loc[idx, 'value'])
            
            # Rolling statistics (5-reading window)
            window_start = max(0, idx - 4)
            window = df.loc[window_start:idx, 'value']
            row_features.append(window.mean())
            row_features.append(window.std() if len(window) > 1 else 0)
            
            # Rate of change
            if idx > 0:
                rate_of_change = df.loc[idx, 'value'] - df.loc[idx - 1, 'value']
            else:
                rate_of_change = 0
            row_features.append(rate_of_change)
            
            # Time features
            ts = df.loc[idx, 'timestamp']
            row_features.append(ts.hour)
            row_features.append(ts.dayofweek)
            row_features.append(1 if 22 <= ts.hour or ts.hour < 6 else 0)  # Night flag
            
            features.append(row_features)
        
        return np.array(features)
    
    def rule_based_classify(self, value: float) -> str:
        """
        Simple rule-based classification.
        
        Args:
            value: Sound level value
            
        Returns:
            Category string
        """
        if value < self.thresholds['quiet']:
            return 'quiet'
        elif value < self.thresholds['normal']:
            return 'normal'
        elif value < self.thresholds['moderate']:
            return 'moderate'
        elif value < self.thresholds['loud']:
            return 'loud'
        else:
            return 'concerning'
    
    def train_classifier(self, df: pd.DataFrame):
        """
        Train Random Forest classifier on labeled data.
        
        Args:
            df: DataFrame with columns: value, timestamp, label
        """
        logger.info(f"Training classifier on {len(df)} samples")
        
        # Extract features
        X = self.extract_features(df[['value', 'timestamp']])
        y = df['label'].values
        
        # Split data
        X_train, X_test, y_train, y_test = train_test_split(
            X, y, test_size=0.2, random_state=42
        )
        
        # Scale features
        X_train_scaled = self.scaler.fit_transform(X_train)
        X_test_scaled = self.scaler.transform(X_test)
        
        # Train classifier
        self.classifier = RandomForestClassifier(
            n_estimators=100,
            max_depth=10,
            random_state=42
        )
        self.classifier.fit(X_train_scaled, y_train)
        
        # Evaluate
        train_score = self.classifier.score(X_train_scaled, y_train)
        test_score = self.classifier.score(X_test_scaled, y_test)
        
        logger.info(f"Training accuracy: {train_score:.3f}")
        logger.info(f"Testing accuracy: {test_score:.3f}")
        
        # Save model
        self.save_classifier()
    
    def train_anomaly_detector(self, df: pd.DataFrame):
        """
        Train Isolation Forest for anomaly detection.
        
        Args:
            df: DataFrame with columns: value, timestamp
        """
        logger.info(f"Training anomaly detector on {len(df)} samples")
        
        # Extract features
        X = self.extract_features(df[['value', 'timestamp']])
        
        # Scale features
        X_scaled = self.scaler.fit_transform(X)
        
        # Train anomaly detector
        self.anomaly_detector = IsolationForest(
            contamination=0.1,  # Expect 10% anomalies
            random_state=42
        )
        self.anomaly_detector.fit(X_scaled)
        
        logger.info("Anomaly detector trained")
        
        # Save model
        self.save_anomaly_detector()
    
    def predict(self, df: pd.DataFrame) -> pd.DataFrame:
        """
        Predict classifications and detect anomalies.
        
        Args:
            df: DataFrame with columns: value, timestamp
            
        Returns:
            DataFrame with added columns: category, is_anomaly, anomaly_score
        """
        result = df.copy()
        
        # Extract features
        X = self.extract_features(df[['value', 'timestamp']])
        X_scaled = self.scaler.transform(X)
        
        # Rule-based classification (always available)
        result['category_rule'] = df['value'].apply(self.rule_based_classify)
        
        # ML-based classification (if model trained)
        if self.classifier is not None:
            result['category_ml'] = self.classifier.predict(X_scaled)
            result['category_confidence'] = self.classifier.predict_proba(X_scaled).max(axis=1)
        else:
            result['category_ml'] = None
            result['category_confidence'] = 0.0
        
        # Anomaly detection (if model trained)
        if self.anomaly_detector is not None:
            anomaly_predictions = self.anomaly_detector.predict(X_scaled)
            result['is_anomaly'] = anomaly_predictions == -1
            result['anomaly_score'] = self.anomaly_detector.score_samples(X_scaled)
        else:
            result['is_anomaly'] = False
            result['anomaly_score'] = 0.0
        
        return result
    
    def save_classifier(self):
        """Save trained classifier to disk."""
        if self.classifier is not None:
            joblib.dump(self.classifier, self.model_dir / 'classifier.joblib')
            joblib.dump(self.scaler, self.model_dir / 'scaler.joblib')
            logger.info(f"Classifier saved to {self.model_dir}")
    
    def save_anomaly_detector(self):
        """Save trained anomaly detector to disk."""
        if self.anomaly_detector is not None:
            joblib.dump(self.anomaly_detector, self.model_dir / 'anomaly_detector.joblib')
            logger.info(f"Anomaly detector saved to {self.model_dir}")
    
    def load_classifier(self):
        """Load trained classifier from disk."""
        classifier_path = self.model_dir / 'classifier.joblib'
        scaler_path = self.model_dir / 'scaler.joblib'
        
        if classifier_path.exists() and scaler_path.exists():
            self.classifier = joblib.load(classifier_path)
            self.scaler = joblib.load(scaler_path)
            logger.info("Classifier loaded successfully")
        else:
            logger.warning("No pre-trained classifier found")
    
    def load_anomaly_detector(self):
        """Load trained anomaly detector from disk."""
        path = self.model_dir / 'anomaly_detector.joblib'
        
        if path.exists():
            self.anomaly_detector = joblib.load(path)
            logger.info("Anomaly detector loaded successfully")
        else:
            logger.warning("No pre-trained anomaly detector found")
    
    def analyze_patterns(self, df: pd.DataFrame) -> dict:
        """
        Analyze patterns in sound data.
        
        Args:
            df: DataFrame with predictions
            
        Returns:
            Dictionary with pattern analysis
        """
        analysis = {}
        
        # Basic statistics
        analysis['total_readings'] = len(df)
        analysis['avg_level'] = float(df['value'].mean())
        analysis['std_level'] = float(df['value'].std())
        analysis['min_level'] = float(df['value'].min())
        analysis['max_level'] = float(df['value'].max())
        
        # Anomaly statistics
        if 'is_anomaly' in df.columns:
            analysis['anomaly_count'] = int(df['is_anomaly'].sum())
            analysis['anomaly_percentage'] = float(df['is_anomaly'].mean() * 100)
        
        # Time-based patterns
        if 'timestamp' in df.columns:
            df['hour'] = pd.to_datetime(df['timestamp']).dt.hour
            hourly_avg = df.groupby('hour')['value'].mean()
            analysis['peak_hour'] = int(hourly_avg.idxmax())
            analysis['quietest_hour'] = int(hourly_avg.idxmin())
            
        # Category distribution
        if 'category_rule' in df.columns:
            analysis['category_distribution'] = df['category_rule'].value_counts().to_dict()
        
        return analysis
