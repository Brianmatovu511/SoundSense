"""
Database interface for ML service.
Fetches data from PostgreSQL for training and prediction.
"""
import pandas as pd
from sqlalchemy import create_engine, text
from datetime import datetime, timedelta
from typing import Optional, List
from loguru import logger


class SoundDatabase:
    """Interface to SoundSense PostgreSQL database."""
    
    def __init__(self, database_url: str):
        """
        Initialize database connection.
        
        Args:
            database_url: PostgreSQL connection string
        """
        # SQLAlchemy 2.0+ requires 'postgresql://' instead of 'postgres://'
        if database_url.startswith('postgres://'):
            database_url = database_url.replace('postgres://', 'postgresql://', 1)
        
        self.engine = create_engine(database_url)
        logger.info("Database connection initialized")
    
    def fetch_recent_readings(
        self,
        limit: int = 1000,
        hours_back: Optional[int] = None
    ) -> pd.DataFrame:
        """
        Fetch recent sensor readings.
        
        Args:
            limit: Maximum number of readings to fetch
            hours_back: If set, only fetch readings from last N hours
            
        Returns:
            DataFrame with columns: id, patient_id, device_id, code, value, unit, timestamp
        """
        query = """
            SELECT 
                id, 
                patient_id, 
                device_id, 
                code, 
                value, 
                unit, 
                timestamp,
                created_at
            FROM sensor_readings
        """
        
        if hours_back is not None:
            cutoff = datetime.utcnow() - timedelta(hours=hours_back)
            query += f" WHERE timestamp >= '{cutoff.isoformat()}'"
        
        query += " ORDER BY timestamp DESC"
        query += f" LIMIT {limit}"
        
        with self.engine.connect() as conn:
            df = pd.read_sql(text(query), conn)
        
        logger.info(f"Fetched {len(df)} readings from database")
        return df
    
    def fetch_readings_by_patient(
        self,
        patient_id: str,
        limit: int = 1000
    ) -> pd.DataFrame:
        """
        Fetch readings for a specific patient.
        
        Args:
            patient_id: Patient identifier
            limit: Maximum number of readings
            
        Returns:
            DataFrame with readings
        """
        query = """
            SELECT 
                id, 
                patient_id, 
                device_id, 
                code, 
                value, 
                unit, 
                timestamp,
                created_at
            FROM sensor_readings
            WHERE patient_id = :patient_id
            ORDER BY timestamp DESC
            LIMIT :limit
        """
        
        with self.engine.connect() as conn:
            df = pd.read_sql(
                text(query),
                conn,
                params={'patient_id': patient_id, 'limit': limit}
            )
        
        logger.info(f"Fetched {len(df)} readings for patient {patient_id}")
        return df
    
    def fetch_readings_by_time_range(
        self,
        start_time: datetime,
        end_time: datetime
    ) -> pd.DataFrame:
        """
        Fetch readings within a time range.
        
        Args:
            start_time: Start of time range
            end_time: End of time range
            
        Returns:
            DataFrame with readings
        """
        query = """
            SELECT 
                id, 
                patient_id, 
                device_id, 
                code, 
                value, 
                unit, 
                timestamp,
                created_at
            FROM sensor_readings
            WHERE timestamp BETWEEN :start_time AND :end_time
            ORDER BY timestamp ASC
        """
        
        with self.engine.connect() as conn:
            df = pd.read_sql(
                text(query),
                conn,
                params={'start_time': start_time, 'end_time': end_time}
            )
        
        logger.info(f"Fetched {len(df)} readings between {start_time} and {end_time}")
        return df
    
    def get_statistics(self) -> dict:
        """
        Get database statistics.
        
        Returns:
            Dictionary with stats
        """
        query = """
            SELECT 
                COUNT(*) as total_readings,
                COUNT(DISTINCT patient_id) as unique_patients,
                COUNT(DISTINCT device_id) as unique_devices,
                MIN(timestamp) as earliest_reading,
                MAX(timestamp) as latest_reading,
                AVG(value) as avg_value,
                STDDEV(value) as stddev_value
            FROM sensor_readings
        """
        
        with self.engine.connect() as conn:
            result = conn.execute(text(query)).fetchone()
        
        stats = {
            'total_readings': result[0],
            'unique_patients': result[1],
            'unique_devices': result[2],
            'earliest_reading': result[3],
            'latest_reading': result[4],
            'avg_value': float(result[5]) if result[5] else 0.0,
            'stddev_value': float(result[6]) if result[6] else 0.0,
        }
        
        logger.info(f"Database stats: {stats['total_readings']} total readings")
        return stats
    
    def create_training_dataset(
        self,
        min_readings: int = 100
    ) -> pd.DataFrame:
        """
        Create labeled training dataset.
        
        Uses rule-based labels for supervised learning.
        
        Args:
            min_readings: Minimum number of readings required
            
        Returns:
            DataFrame with columns: value, timestamp, label
        """
        df = self.fetch_recent_readings(limit=10000)
        
        if len(df) < min_readings:
            logger.warning(f"Only {len(df)} readings available, need at least {min_readings}")
            return pd.DataFrame()
        
        # Create labels based on value thresholds
        def label_value(value):
            if value < 187:
                return 'quiet'
            elif value < 300:
                return 'normal'
            elif value < 500:
                return 'moderate'
            elif value < 700:
                return 'loud'
            else:
                return 'concerning'
        
        df['label'] = df['value'].apply(label_value)
        
        logger.info(f"Created training dataset with {len(df)} samples")
        logger.info(f"Label distribution:\n{df['label'].value_counts()}")
        
        return df[['value', 'timestamp', 'label']]
