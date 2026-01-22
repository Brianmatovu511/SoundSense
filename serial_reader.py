"""
Simple serial port reader for Arduino that sends data to the backend API.
This runs natively on Windows to access COM ports directly.
"""
import serial
import requests
import time
import json
from datetime import datetime
import re

# Configuration
SERIAL_PORT = "COM4"
BAUD_RATE = 9600
BACKEND_URL = "http://localhost:8080/ingest"

def read_and_send():
    """Read from serial port and send to backend API"""
    print(f"Opening serial port {SERIAL_PORT} at {BAUD_RATE} baud...")
    
    try:
        ser = serial.Serial(SERIAL_PORT, BAUD_RATE, timeout=1)
        print(f"✓ Connected to {SERIAL_PORT}")
        print(f"Sending data to {BACKEND_URL}")
        print("Reading data... (Press Ctrl+C to stop)\n")
        
        # Pattern: SOUND:123
        pattern = re.compile(r'^SOUND:(\d+)\s*$')
        
        while True:
            try:
                line = ser.readline().decode('utf-8', errors='ignore').strip()
                
                if line:
                    print(f"Received: {line}")
                    
                    match = pattern.match(line)
                    if match:
                        value = int(match.group(1))
                        
                        # Create sensor reading payload
                        payload = {
                            "patient_id": "demo-patient-1",
                            "device_id": f"arduino-{SERIAL_PORT}",
                            "code": "sound",
                            "value": float(value),
                            "unit": "raw",
                            "ts": datetime.utcnow().isoformat() + "Z"
                        }
                        
                        # Send to backend
                        try:
                            response = requests.post(BACKEND_URL, json=payload, timeout=2)
                            if response.status_code == 200:
                                print(f"  ✓ Sent value {value} to backend")
                            else:
                                print(f"  ✗ Backend returned status {response.status_code}")
                        except requests.exceptions.RequestException as e:
                            print(f"  ✗ Failed to send to backend: {e}")
                    else:
                        print(f"  (Ignored - doesn't match SOUND:### pattern)")
                        
            except UnicodeDecodeError:
                pass  # Skip invalid data
                
            time.sleep(0.01)  # Small delay
            
    except serial.SerialException as e:
        print(f"✗ Error opening serial port: {e}")
        print(f"\nMake sure:")
        print(f"  1. Arduino is connected to {SERIAL_PORT}")
        print(f"  2. No other program is using the port")
        print(f"  3. Arduino IDE Serial Monitor is closed")
        return 1
    except KeyboardInterrupt:
        print("\n\nStopped by user")
        return 0
    finally:
        if 'ser' in locals() and ser.is_open:
            ser.close()
            print(f"Closed {SERIAL_PORT}")

if __name__ == "__main__":
    exit(read_and_send())
