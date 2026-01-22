"""
FastAPI service for ML predictions and training.

Endpoints:
- POST /predict - Get predictions for recent readings
- POST /train - Trigger model training
- GET /health - Health check
- GET /stats - Get model statistics
- GET /analysis - Get pattern analysis
"""

from fastapi import FastAPI, HTTPException, BackgroundTasks
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, Field
from typing import List, Optional, Dict, Any
from datetime import datetime
import os
import uvicorn
from loguru import logger

from model import SoundClassifier
from database import SoundDatabase

# Configuration
DATABASE_URL = os.getenv(
    "DATABASE_URL",
    "postgres://soundsense:soundsense_dev_password@postgres:5432/soundsense"
)
MODEL_DIR = os.getenv("MODEL_DIR", "models")

# Initialize
app = FastAPI(
    title="SoundSense ML Service",
    description="Machine Learning service for sound pattern classification and anomaly detection",
    version="1.0.0"
)

# CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # In production, specify allowed origins
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Initialize ML model and database
classifier = SoundClassifier(model_dir=MODEL_DIR)
db = SoundDatabase(DATABASE_URL)

# Try to load existing models
classifier.load_classifier()
classifier.load_anomaly_detector()


# Pydantic models
class PredictionRequest(BaseModel):
    limit: int = Field(default=100, ge=1, le=1000, description="Number of readings to analyze")
    hours_back: Optional[int] = Field(default=None, ge=1, description="Only analyze last N hours")


class PredictionResponse(BaseModel):
    success: bool
    total_readings: int
    predictions: List[Dict[str, Any]]
    summary: Dict[str, Any]


class TrainingRequest(BaseModel):
    min_samples: int = Field(default=100, ge=10, description="Minimum samples for training")


class TrainingResponse(BaseModel):
    success: bool
    message: str
    samples_used: int


class AnalysisResponse(BaseModel):
    success: bool
    analysis: Dict[str, Any]


class HealthResponse(BaseModel):
    status: str
    database_connected: bool
    classifier_loaded: bool
    anomaly_detector_loaded: bool


# Endpoints
@app.get("/health", response_model=HealthResponse)
async def health_check():
    """Health check endpoint."""
    try:
        stats = db.get_statistics()
        db_connected = True
    except Exception as e:
        logger.error(f"Database connection failed: {e}")
        db_connected = False
    
    return HealthResponse(
        status="healthy" if db_connected else "degraded",
        database_connected=db_connected,
        classifier_loaded=classifier.classifier is not None,
        anomaly_detector_loaded=classifier.anomaly_detector is not None
    )


@app.post("/predict", response_model=PredictionResponse)
async def predict(request: PredictionRequest):
    """
    Get ML predictions for recent sensor readings.
    
    Returns:
        Predictions with categories, anomaly flags, and confidence scores
    """
    try:
        # Fetch data
        df = db.fetch_recent_readings(
            limit=request.limit,
            hours_back=request.hours_back
        )
        
        if len(df) == 0:
            raise HTTPException(status_code=404, detail="No readings found")
        
        # Get predictions
        predictions_df = classifier.predict(df)
        
        # Convert to list of dicts
        predictions_list = predictions_df.to_dict('records')
        
        # Calculate summary
        summary = {
            'total_readings': len(predictions_df),
            'category_distribution': predictions_df['category_rule'].value_counts().to_dict(),
            'avg_confidence': float(predictions_df['category_confidence'].mean()) if 'category_confidence' in predictions_df.columns else None,
            'anomaly_count': int(predictions_df['is_anomaly'].sum()) if 'is_anomaly' in predictions_df.columns else 0,
            'avg_value': float(predictions_df['value'].mean()),
            'max_value': float(predictions_df['value'].max()),
            'min_value': float(predictions_df['value'].min()),
        }
        
        return PredictionResponse(
            success=True,
            total_readings=len(predictions_df),
            predictions=predictions_list,
            summary=summary
        )
    
    except Exception as e:
        logger.error(f"Prediction failed: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/train", response_model=TrainingResponse)
async def train_models(request: TrainingRequest, background_tasks: BackgroundTasks):
    """
    Trigger model training on historical data.
    
    Training happens in background to avoid blocking.
    """
    try:
        # Check if we have enough data
        stats = db.get_statistics()
        if stats['total_readings'] < request.min_samples:
            raise HTTPException(
                status_code=400,
                detail=f"Insufficient data: {stats['total_readings']} readings available, need {request.min_samples}"
            )
        
        # Create training dataset
        training_df = db.create_training_dataset(min_readings=request.min_samples)
        
        if len(training_df) == 0:
            raise HTTPException(status_code=400, detail="Failed to create training dataset")
        
        # Train models in background
        def train_task():
            logger.info("Starting model training...")
            classifier.train_classifier(training_df)
            classifier.train_anomaly_detector(training_df[['value', 'timestamp']])
            logger.info("Model training completed")
        
        background_tasks.add_task(train_task)
        
        return TrainingResponse(
            success=True,
            message="Training started in background",
            samples_used=len(training_df)
        )
    
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Training failed: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/analysis", response_model=AnalysisResponse)
async def get_analysis(
    limit: int = 1000,
    hours_back: Optional[int] = None
):
    """
    Get comprehensive pattern analysis of sound data.
    
    Args:
        limit: Maximum readings to analyze
        hours_back: Only analyze last N hours
        
    Returns:
        Detailed pattern analysis including trends, peaks, anomalies
    """
    try:
        # Fetch data
        df = db.fetch_recent_readings(limit=limit, hours_back=hours_back)
        
        if len(df) == 0:
            raise HTTPException(status_code=404, detail="No readings found")
        
        # Get predictions
        predictions_df = classifier.predict(df)
        
        # Analyze patterns
        analysis = classifier.analyze_patterns(predictions_df)
        
        return AnalysisResponse(success=True, analysis=analysis)
    
    except Exception as e:
        logger.error(f"Analysis failed: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/stats")
async def get_stats():
    """Get model and database statistics."""
    try:
        db_stats = db.get_statistics()
        
        return {
            "success": True,
            "database": db_stats,
            "models": {
                "classifier_loaded": classifier.classifier is not None,
                "anomaly_detector_loaded": classifier.anomaly_detector is not None
            }
        }
    except Exception as e:
        logger.error(f"Stats failed: {e}")
        raise HTTPException(status_code=500, detail=str(e))


# Startup event
@app.on_event("startup")
async def startup_event():
    """Initialize service on startup."""
    logger.info("SoundSense ML Service starting...")
    logger.info(f"Database URL: {DATABASE_URL}")
    logger.info(f"Model directory: {MODEL_DIR}")
    
    # Check database connection
    try:
        stats = db.get_statistics()
        logger.info(f"Database connected: {stats['total_readings']} readings available")
    except Exception as e:
        logger.error(f"Database connection failed: {e}")
    
    # Check if models exist
    if classifier.classifier is None:
        logger.warning("No pre-trained classifier found. Run POST /train to train models.")
    else:
        logger.info("Pre-trained classifier loaded successfully")
    
    if classifier.anomaly_detector is None:
        logger.warning("No pre-trained anomaly detector found.")
    else:
        logger.info("Pre-trained anomaly detector loaded successfully")


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(
        "api:app",
        host="0.0.0.0",
        port=8000,
        reload=True,
        log_level="info"
    )
