
# SoundSense — Real-Time Sound Sensor Dashboard (FHIR + D3)

SoundSense is a **real-time health-sensor web application** that captures ambient sound levels using an **Arduino sound sensor**, streams the data to a **Rust (Actix Web) backend**, emits **FHIR-compliant Observations**, and visualizes everything live in a **D3.js frontend**.

This project was built to meet **Media Management / Innovation & Complexity (INCO)** course requirements and follows good software engineering practices: testing, containerization, standards compliance, and clear system architecture.

---

## Table of Contents
1. Project Overview  
2. Features  
3. System Architecture  
4. Hardware Setup (Arduino)  
5. Data Flow  
6. Backend (Rust / Actix)  
7. Frontend (D3.js)  
8. FHIR Compliance  
9. Running the Project Locally  
10. Running with Docker (with and without Simulator)  
11. API Endpoints  
12. Testing & Validation  
13. Project Structure  
14. Evaluation Readiness  

---

## 1. Project Overview
**Goal:** Demonstrate a complete end-to-end real-time health data system from a physical sensor to a web dashboard using healthcare interoperability standards (FHIR).

---

## 2. Features

| Feature | Description |
|-------|-------------|
| Real-time sensing | Arduino sound sensor |
| Serial ingestion | USB serial parsing |
| FHIR output | HL7 FHIR Observation |
| WebSocket stream | Live updates |
| REST API | Query data |
| D3 dashboard | Interactive charts |
| Simulator | Synthetic data |
| Docker | Reproducible deployment |

---

## 3. System Architecture

Arduino → Serial → Rust Backend → WebSocket/REST → D3 Frontend

---

## 4. Hardware Setup (Arduino)

### Components
| Component | Purpose |
|----------|--------|
| Arduino Uno | Controller |
| Sound Sensor | Noise measurement |
| USB Cable | Power & data |

### Wiring
| Sensor Pin | Arduino |
|-----------|---------|
| VCC | 5V |
| GND | GND |
| AO | A0 |

---

## 5. Data Flow
1. Arduino reads sound
2. Sends `SOUND:<value>`
3. Backend parses & validates
4. Converts to FHIR Observation
5. Broadcasts via WebSocket
6. Frontend visualizes

---

## 6. Backend
- Rust + Actix Web
- Serial ingest
- FHIR mapping
- WebSocket hub

---

## 7. Frontend
- D3.js
- Live chart
- KPIs (min/max/avg)
- Auto-scaled axis

---

## 8. FHIR Compliance

Uses **FHIR R4 Observation** resource with LOINC-style coding.

### FHIR Schema Validation
- Full FHIR R4 schema validation implemented
- Validates resource types, status codes, coding systems
- Ensures compliance with HL7 FHIR standards
- See [COMPLIANCE.md](COMPLIANCE.md) for details

### HIPAA Compliance
✅ **Encryption at Rest**: PostgreSQL pgcrypto for PHI data  
✅ **Audit Logging**: Comprehensive tracking of all PHI access  
✅ **Access Controls**: JWT-based authentication and authorization  
✅ **Data Validation**: FHIR schema validation prevents invalid data  

For complete HIPAA compliance details, see [COMPLIANCE.md](COMPLIANCE.md)

---

## 9. Run Locally

### Backend
```bash
cd backend
cargo run --bin soundsense-backend
```

With Arduino:
```bash
cargo run --bin soundsense-backend -- --serial COM6
```

### Frontend
```bash
cd frontend
python -m http.server 5173
```
Open: http://localhost:5173

---

## 10. Docker

### Backend + Frontend
```bash
docker compose up --build
```

### With Simulator
```bash
docker compose --profile sim up --build
```

---

## 11. API Endpoints

| Endpoint | Method | Description |
|---------|--------|-------------|
| /healthz | GET | Health check |
| /ingest | POST | Ingest reading |
| /fhir/Observation | GET | FHIR bundle |
| /ws/live | GET | WebSocket |

---

## 12. Testing & CI/CD

### Unit & Integration Tests
```bash
cd backend
cargo test --all --all-features
```

### Code Coverage
```bash
cargo install cargo-llvm-cov
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
```

### CI/CD Pipeline
- **Automated testing** on every push
- **Security scanning** with Trivy
- **Code coverage** reporting with Codecov
- **Docker builds** for all services
- **Staging/Production** deployment automation

See [.github/workflows/ci.yml](.github/workflows/ci.yml) for pipeline details.

All tests pass ✅

---

## 13. Project Structure

```
SoundSense/
├── backend/
├── frontend/
├── arduino/
├── docker-compose.yml
└── README.md
```

---

## 14. Evaluation Readiness

✔ Real sensor  
✔ FHIR compliant  
✔ Real-time streaming  
✔ Dockerized  
✔ Tested  



 **Brian Doctor Matovu
   Aamna Muhammad**