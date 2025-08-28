-- Create devices table based on Device struct in ddb.rs
CREATE TABLE devices (
    device_id TEXT PRIMARY KEY,
    device_type TEXT NOT NULL,
    aircraft_model TEXT NOT NULL,
    registration TEXT NOT NULL,
    cn TEXT NOT NULL,
    tracked BOOLEAN NOT NULL,
    identified BOOLEAN NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for commonly queried fields
CREATE INDEX idx_devices_device_type ON devices(device_type);
CREATE INDEX idx_devices_registration ON devices(registration);
CREATE INDEX idx_devices_aircraft_model ON devices(aircraft_model);
CREATE INDEX idx_devices_tracked ON devices(tracked);
CREATE INDEX idx_devices_identified ON devices(identified);

-- Create a function to automatically update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create trigger to automatically update updated_at on row updates
CREATE TRIGGER update_devices_updated_at
    BEFORE UPDATE ON devices
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
