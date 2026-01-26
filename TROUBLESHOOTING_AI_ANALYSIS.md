# AI Analysis Feature - Troubleshooting Guide

## Issue: "No data for analysis"

### Root Cause
The frontend was looking for the wrong data structure in the ML service response.

**Before:** Frontend expected `data.summary.total_samples`  
**After:** ML service returns `data.analysis.total_readings`

### Fix Applied
Updated [frontend/index.html](frontend/index.html) to correctly parse the ML service response structure:

```javascript
// Old (incorrect)
if (data.summary) {
  html += `<p><strong>Total Samples:</strong> ${data.summary.total_samples || 0}</p>`;
}

// New (correct)
if (data.analysis) {
  html += `<p><strong>Total Samples:</strong> ${data.analysis.total_readings || 0}</p>`;
}
```

## ML Service Response Structure

The `/analysis` endpoint returns:

```json
{
  "success": true,
  "analysis": {
    "total_readings": 150,
    "avg_level": 245.3,
    "std_level": 87.2,
    "min_level": 45.0,
    "max_level": 512.0,
    "anomaly_count": 3,
    "anomaly_percentage": 2.0,
    "peak_hour": 14,
    "quietest_hour": 3,
    "category_distribution": {
      "Quiet": 45,
      "Normal": 80,
      "Moderate": 20,
      "Loud": 5
    }
  }
}
```

## Testing the Fix

### 1. Ensure Data Exists
Make sure you have sensor readings in the database:

```bash
# Check database has data
docker-compose exec postgres psql -U soundsense -d soundsense -c "SELECT COUNT(*) FROM sensor_readings;"
```

### 2. Test ML Service Directly
```bash
curl http://localhost:8000/health
curl http://localhost:8000/analysis
```

### 3. Generate Test Data
If you have no data, use the simulator:

```bash
# Start with simulator
docker-compose --profile sim up -d

# Or run the sound simulator manually
cd backend
cargo run --bin sound-simulator
```

### 4. Check Frontend
1. Open http://localhost:5173
2. Navigate to "🤖 AI Analysis" tab
3. Click "Refresh Analysis"
4. You should now see:
   - Overall Summary with statistics
   - Time Patterns (peak/quietest hours)
   - Sound Classifications by category
   - Anomaly detection results

## Additional Features

The updated frontend now displays:

✅ **Total readings** analyzed  
✅ **Average, min, max** sound levels  
✅ **Standard deviation** for variability  
✅ **Peak and quietest hours** for time-based patterns  
✅ **Category distribution** (Quiet, Normal, Moderate, Loud, Concerning)  
✅ **Anomaly detection** with percentage  

## Common Issues

### "ML service unavailable"
- Ensure ML service is running: `docker-compose ps ml-service`
- Check logs: `docker-compose logs ml-service`

### "No readings found" (404)
- Database is empty - start simulator or connect Arduino
- Check: `curl http://localhost:8000/stats`

### "Error: fetch failed"
- ML service not accessible
- CORS issue (check browser console)
- Port 8000 not exposed

### Empty Categories
- Not enough data variety
- All readings in same category
- Need more diverse sensor data

## Implementation Details

### Files Modified
- `frontend/index.html` - Fixed data structure parsing in `loadMLAnalysis()` function

### Key Changes
1. Changed from `data.summary` to `data.analysis`
2. Added better error handling with helpful messages
3. Display peak/quietest hours from time analysis
4. Show all category distributions from ML predictions
5. Improved anomaly display with percentage

### Code Location
See `loadMLAnalysis()` function around line 945 in [frontend/index.html](frontend/index.html)

## Verification

After the fix, the AI Analysis tab should display rich insights when you have data:

```
📈 Overall Summary
Total Samples: 150
Average Level: 245.3 AU
Standard Deviation: 87.2 AU
Range: 45.0 - 512.0 AU

⏰ Time Patterns
Peak Activity Hour: 14:00
Quietest Hour: 3:00

🎯 Sound Classifications
Quiet: 45 (30.0%)
Normal: 80 (53.3%)
Moderate: 20 (13.3%)
Loud: 5 (3.3%)

✓ No Anomalies: All readings appear normal
```

## Next Steps

If you still see "no data":
1. Verify database has readings (see Testing section above)
2. Check ML service logs: `docker-compose logs ml-service`
3. Test ML endpoint directly: `curl http://localhost:8000/analysis`
4. Ensure simulator or Arduino is sending data
