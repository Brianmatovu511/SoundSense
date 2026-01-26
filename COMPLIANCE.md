# CI/CD and HIPAA Compliance Implementation Guide

## Overview
This document describes the CI/CD pipeline enhancements and HIPAA compliance features added to SoundSense.

## CI/CD Pipeline Enhancements

### Pipeline Stages

#### 1. Security Scanning
- **Trivy vulnerability scanner** for filesystem security analysis
- SARIF output uploaded to GitHub Security tab
- Runs on every push and pull request

#### 2. Backend Testing
- **Code formatting** check with rustfmt
- **Linting** with clippy (all warnings treated as errors)
- **Security audit** with cargo-audit
- **Unit and integration tests** with PostgreSQL test database
- **Code coverage** reporting with cargo-llvm-cov
- Coverage reports uploaded to Codecov

#### 3. ML Service Testing
- **Format check** with black
- **Linting** with flake8
- **Type checking** with mypy
- **Security check** with bandit
- **Test coverage** with pytest-cov
- PostgreSQL test database for integration testing

#### 4. Integration Tests
- Full docker-compose stack deployment
- Health check validation for all services
- End-to-end API testing
- Log collection and artifact upload

#### 5. Docker Build & Push
- Multi-service build strategy (backend, ml-service)
- Container registry integration (GitHub Container Registry)
- Automated tagging with semantic versioning
- Layer caching for faster builds

#### 6. Deployment
- **Staging environment** deployment on main branch
- **Production environment** deployment with manual approval
- Environment-specific configurations

### Running CI/CD Locally

```bash
# Run backend tests locally
cd backend
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-features

# Run ML service tests locally
cd ml-service
black --check .
flake8 .
pytest --cov=. --cov-report=xml

# Run integration tests
docker-compose up -d
curl http://localhost:8080/healthz
curl http://localhost:8000/health
docker-compose down
```

## HIPAA Compliance Features

### 1. FHIR Schema Validation

#### Implementation
- Full FHIR R4 schema validation in [`backend/src/fhir.rs`](backend/src/fhir.rs)
- Validates:
  - Resource type compliance
  - Status codes (registered, preliminary, final, amended, etc.)
  - Coding system URIs (LOINC)
  - Subject references
  - Value constraints
  - Bundle structure

#### Usage
```rust
let obs = FhirObservation::from_reading(reading);
obs.validate()?;  // Throws error if non-compliant
```

### 2. Database Encryption at Rest

#### Implementation
- PostgreSQL pgcrypto extension enabled
- Symmetric encryption functions for PHI data
- Key rotation support via `encryption_key_version` field
- Migration: [`backend/migrations/20260126000001_add_encryption_and_audit.sql`](backend/migrations/20260126000001_add_encryption_and_audit.sql)

#### Features
- `encrypted_patient_data` table for sensitive information
- `encrypt_patient_data()` function for storing PHI
- `decrypt_patient_data()` function for authorized access
- Encryption key version tracking

#### Usage
```sql
-- Encrypt patient data
SELECT encrypt_patient_data(
    'patient123',
    '{"name": "John Doe", "ssn": "123-45-6789"}',
    'encryption_key_here',
    'v1'
);

-- Decrypt patient data (requires key)
SELECT decrypt_patient_data('patient123', 'encryption_key_here');
```

### 3. Audit Logging for HIPAA Compliance

#### Implementation
- Comprehensive audit logging module in [`backend/src/audit.rs`](backend/src/audit.rs)
- Tracks all PHI access and modifications
- Integration with JWT authentication for user tracking

#### Audit Log Fields
- **Timestamp**: When the action occurred
- **User ID**: From JWT claims (username or device ID)
- **User Role**: admin, user, or device
- **Action**: CREATE, READ, UPDATE, DELETE, LOGIN, LOGOUT, ACCESS_DENIED
- **Resource Type**: Type of data accessed (Observation, SensorReading, etc.)
- **Resource ID**: Specific record ID
- **Patient ID**: Patient whose data was accessed
- **IP Address**: Request origin
- **User Agent**: Client information
- **Request Path**: API endpoint
- **Status Code**: HTTP response code
- **Error Message**: If action failed
- **Metadata**: Additional context (JSON)

#### Usage
```rust
use crate::audit::{AuditAction, AuditLogEntry};

// Log a PHI access event
let audit_entry = AuditLogEntry::new(
    AuditAction::Read,
    "Observation".to_string(),
)
.with_user(user_id, role)
.with_patient_id(patient_id)
.with_request_context(ip_addr, user_agent, path)
.with_status_code(200);

audit_entry.log(&pool).await?;
```

#### Audit Queries
```rust
// Get all access to a patient's records
let logs = audit_logger.get_patient_access_log("patient123", 100).await?;

// Get all actions by a specific user
let logs = audit_logger.get_user_activity_log("user123", 100).await?;
```

### 4. HIPAA Compliance Checklist

✅ **Administrative Safeguards**
- User authentication with JWT tokens
- Role-based access control (admin, user, device)
- Audit logging of all PHI access

✅ **Physical Safeguards**
- Containerized deployment with Docker
- Environment variable management for secrets

✅ **Technical Safeguards**
- Encryption at rest (PostgreSQL pgcrypto)
- TLS/HTTPS for data in transit (via Nginx/reverse proxy)
- Access controls via JWT authentication
- Audit trails for all PHI access
- Automatic log-off (JWT expiration)
- FHIR schema validation

✅ **Breach Notification**
- Audit logs provide breach detection capabilities
- Timestamped access records for investigation

### 5. Configuration

#### Environment Variables
Add to your `.env` file:

```bash
# Encryption Key (MUST be changed in production)
ENCRYPTION_KEY=your-strong-encryption-key-min-32-chars-change-this

# Database encryption at rest (PostgreSQL)
# Enable transparent data encryption in PostgreSQL config:
# ssl = on
# ssl_cert_file = 'server.crt'
# ssl_key_file = 'server.key'

# JWT secret for authentication
JWT_SECRET=your_super_secret_jwt_key_change_this_in_production_min_32_chars
```

#### Database Setup
Run migrations to enable encryption and audit logging:

```bash
# Using SQLx CLI
sqlx migrate run

# Or in your application startup
# Migrations are automatically applied
```

## Testing

### FHIR Validation Tests
```bash
cd backend
cargo test fhir::tests
```

### Audit Logging Tests
```bash
cd backend
cargo test audit::tests
```

## Monitoring and Compliance

### Audit Log Retention
- Audit logs should be retained for at least 6 years (HIPAA requirement)
- Implement log rotation and archival strategies
- Regular backup of audit logs to secure, immutable storage

### Regular Compliance Reviews
1. Review audit logs monthly for unusual access patterns
2. Quarterly security assessments
3. Annual HIPAA compliance audits
4. Penetration testing as needed

### Incident Response
1. Monitor audit logs for unauthorized access
2. Alert on failed authentication attempts
3. Investigate anomalies immediately
4. Document all security incidents

## Production Deployment Recommendations

### Required Steps
1. **Change all default secrets** in production
2. **Enable PostgreSQL SSL/TLS** for connections
3. **Configure backup encryption** for database backups
4. **Set up log forwarding** to SIEM system
5. **Enable database audit logging** at PostgreSQL level
6. **Implement key management system** (AWS KMS, Azure Key Vault, etc.)
7. **Configure HTTPS** for all API endpoints
8. **Set up monitoring** for audit log anomalies
9. **Implement disaster recovery** procedures
10. **Document security policies** and procedures

### Security Hardening
```yaml
# docker-compose.yml (production additions)
services:
  postgres:
    environment:
      # Enable SSL
      POSTGRES_SSL_MODE: require
    volumes:
      # Mount SSL certificates
      - ./certs:/var/lib/postgresql/certs:ro
      
  backend:
    environment:
      # Enable strict security
      DATABASE_URL: postgres://user:pass@postgres:5432/db?sslmode=require
      RUST_LOG: info  # Don't log sensitive data in production
```

## Compliance Contacts
- **Security Officer**: [security@yourorg.com]
- **Privacy Officer**: [privacy@yourorg.com]
- **Compliance Team**: [compliance@yourorg.com]

## Additional Resources
- [HIPAA Security Rule](https://www.hhs.gov/hipaa/for-professionals/security/index.html)
- [FHIR R4 Specification](https://www.hl7.org/fhir/)
- [PostgreSQL Encryption](https://www.postgresql.org/docs/current/encryption-options.html)
