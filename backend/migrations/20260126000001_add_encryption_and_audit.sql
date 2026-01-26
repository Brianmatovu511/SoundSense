-- Migration: Add encryption at rest and HIPAA compliance audit logging
-- Date: 2026-01-26

-- Enable pgcrypto extension for encryption functions
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Create audit log table for HIPAA compliance
CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id VARCHAR(255),  -- JWT subject (username or device_id)
    user_role VARCHAR(50),  -- JWT role (admin, user, device)
    action VARCHAR(100) NOT NULL,  -- Action performed (CREATE, READ, UPDATE, DELETE)
    resource_type VARCHAR(100) NOT NULL,  -- Type of resource (Observation, SensorReading, etc.)
    resource_id VARCHAR(255),  -- ID of the affected resource
    patient_id VARCHAR(255),  -- Patient ID for PHI access tracking
    ip_address INET,  -- IP address of the request
    user_agent TEXT,  -- Browser/client user agent
    request_path TEXT,  -- API endpoint accessed
    status_code INTEGER,  -- HTTP status code
    error_message TEXT,  -- Error message if action failed
    metadata JSONB,  -- Additional context (e.g., changed fields)
    
    -- Indexes for audit queries
    CONSTRAINT action_valid CHECK (action IN ('CREATE', 'READ', 'UPDATE', 'DELETE', 'LOGIN', 'LOGOUT', 'ACCESS_DENIED'))
);

-- Indexes for efficient audit log queries
CREATE INDEX idx_audit_logs_timestamp ON audit_logs (timestamp DESC);
CREATE INDEX idx_audit_logs_user_id ON audit_logs (user_id);
CREATE INDEX idx_audit_logs_patient_id ON audit_logs (patient_id);
CREATE INDEX idx_audit_logs_action ON audit_logs (action);
CREATE INDEX idx_audit_logs_resource_type ON audit_logs (resource_type);

-- Table for encrypted sensitive data (future use - patient demographics, etc.)
CREATE TABLE IF NOT EXISTS encrypted_patient_data (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id VARCHAR(255) NOT NULL UNIQUE,
    encrypted_data BYTEA NOT NULL,  -- Encrypted patient data
    encryption_key_id VARCHAR(100) NOT NULL,  -- Key version for rotation
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT patient_id_not_empty CHECK (LENGTH(TRIM(patient_id)) > 0)
);

-- Index for patient lookups
CREATE INDEX idx_encrypted_patient_data_patient_id ON encrypted_patient_data (patient_id);

-- Add encryption metadata columns to sensor_readings
ALTER TABLE sensor_readings
ADD COLUMN IF NOT EXISTS encrypted_at_rest BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS encryption_key_version VARCHAR(50);

-- Function to log audit events
CREATE OR REPLACE FUNCTION log_audit_event(
    p_user_id VARCHAR(255),
    p_user_role VARCHAR(50),
    p_action VARCHAR(100),
    p_resource_type VARCHAR(100),
    p_resource_id VARCHAR(255) DEFAULT NULL,
    p_patient_id VARCHAR(255) DEFAULT NULL,
    p_metadata JSONB DEFAULT NULL
) RETURNS UUID AS $$
DECLARE
    audit_id UUID;
BEGIN
    INSERT INTO audit_logs (
        user_id,
        user_role,
        action,
        resource_type,
        resource_id,
        patient_id,
        metadata
    ) VALUES (
        p_user_id,
        p_user_role,
        p_action,
        p_resource_type,
        p_resource_id,
        p_patient_id,
        p_metadata
    ) RETURNING id INTO audit_id;
    
    RETURN audit_id;
END;
$$ LANGUAGE plpgsql;

-- Trigger to audit all sensor reading inserts
CREATE OR REPLACE FUNCTION audit_sensor_reading_insert()
RETURNS TRIGGER AS $$
BEGIN
    -- Log the creation event (user context would be set by application)
    PERFORM log_audit_event(
        'system',  -- This should be set by application from JWT
        'system',
        'CREATE',
        'SensorReading',
        NEW.id::TEXT,
        NEW.patient_id,
        jsonb_build_object(
            'device_id', NEW.device_id,
            'code', NEW.code,
            'value', NEW.value,
            'timestamp', NEW.timestamp
        )
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER sensor_readings_audit_insert
    AFTER INSERT ON sensor_readings
    FOR EACH ROW
    EXECUTE FUNCTION audit_sensor_reading_insert();

-- Function to encrypt sensitive data (example for future use)
CREATE OR REPLACE FUNCTION encrypt_patient_data(
    p_patient_id VARCHAR(255),
    p_data TEXT,
    p_encryption_key TEXT,
    p_key_version VARCHAR(100)
) RETURNS UUID AS $$
DECLARE
    encrypted_id UUID;
BEGIN
    INSERT INTO encrypted_patient_data (
        patient_id,
        encrypted_data,
        encryption_key_id
    ) VALUES (
        p_patient_id,
        pgp_sym_encrypt(p_data, p_encryption_key),
        p_key_version
    )
    ON CONFLICT (patient_id) DO UPDATE
    SET 
        encrypted_data = pgp_sym_encrypt(p_data, p_encryption_key),
        encryption_key_id = p_key_version,
        updated_at = NOW()
    RETURNING id INTO encrypted_id;
    
    RETURN encrypted_id;
END;
$$ LANGUAGE plpgsql;

-- Function to decrypt patient data
CREATE OR REPLACE FUNCTION decrypt_patient_data(
    p_patient_id VARCHAR(255),
    p_decryption_key TEXT
) RETURNS TEXT AS $$
DECLARE
    decrypted_data TEXT;
BEGIN
    SELECT pgp_sym_decrypt(encrypted_data, p_decryption_key)
    INTO decrypted_data
    FROM encrypted_patient_data
    WHERE patient_id = p_patient_id;
    
    RETURN decrypted_data;
END;
$$ LANGUAGE plpgsql;

-- View for audit log reporting (excludes sensitive fields)
CREATE OR REPLACE VIEW audit_log_summary AS
SELECT 
    id,
    timestamp,
    user_id,
    user_role,
    action,
    resource_type,
    patient_id,
    status_code,
    CASE 
        WHEN error_message IS NOT NULL THEN 'Error occurred'
        ELSE 'Success'
    END as outcome
FROM audit_logs;

-- Grant permissions (adjust based on your user setup)
-- GRANT SELECT ON audit_log_summary TO readonly_user;

-- Add comment for documentation
COMMENT ON TABLE audit_logs IS 'HIPAA-compliant audit log for tracking all access to protected health information (PHI)';
COMMENT ON TABLE encrypted_patient_data IS 'Encrypted storage for sensitive patient data with key rotation support';
COMMENT ON FUNCTION log_audit_event IS 'Function to log audit events for HIPAA compliance tracking';
