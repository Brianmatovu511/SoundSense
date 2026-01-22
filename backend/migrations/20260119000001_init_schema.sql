-- Initial schema for sensor readings
CREATE TABLE IF NOT EXISTS sensor_readings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id VARCHAR(255) NOT NULL,
    device_id VARCHAR(255) NOT NULL,
    code VARCHAR(50) NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    unit VARCHAR(50) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT patient_id_not_empty CHECK (LENGTH(TRIM(patient_id)) > 0),
    CONSTRAINT device_id_not_empty CHECK (LENGTH(TRIM(device_id)) > 0),
    CONSTRAINT value_must_be_finite CHECK (value = value AND value <> 'Infinity'::float AND value <> '-Infinity'::float),
    CONSTRAINT code_must_be_sound CHECK (code = 'sound')
);

-- Indexes for query performance
CREATE INDEX idx_sensor_readings_timestamp ON sensor_readings (timestamp DESC);
CREATE INDEX idx_sensor_readings_code ON sensor_readings (code);
CREATE INDEX idx_sensor_readings_patient_id ON sensor_readings (patient_id);

-- Trigger function to validate sound values (Arduino analog range: 0-1023)
CREATE OR REPLACE FUNCTION validate_sound_value()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.code = 'sound' AND (NEW.value < 0 OR NEW.value > 1023) THEN
        RAISE EXCEPTION 'Sound value must be between 0 and 1023 (Arduino analog range), got %', NEW.value;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Attach trigger to table
CREATE TRIGGER sensor_readings_validate_sound
    BEFORE INSERT OR UPDATE ON sensor_readings
    FOR EACH ROW
    EXECUTE FUNCTION validate_sound_value();
