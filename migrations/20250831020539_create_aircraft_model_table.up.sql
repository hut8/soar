-- Create aircraft_model table based on AircraftModel struct in faa/models.rs
CREATE TABLE aircraft_model (
    manufacturer_code TEXT NOT NULL,
    model_code TEXT NOT NULL,
    series_code TEXT NOT NULL,
    manufacturer_name TEXT NOT NULL,
    model_name TEXT NOT NULL,
    aircraft_type TEXT,
    engine_type TEXT,
    aircraft_category TEXT,
    builder_certification TEXT,
    number_of_engines SMALLINT,
    number_of_seats SMALLINT,
    weight_class TEXT,
    cruising_speed SMALLINT,
    type_certificate_data_sheet TEXT,
    type_certificate_data_holder TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    -- Composite primary key as specified
    PRIMARY KEY (manufacturer_code, model_code, series_code)
);

-- Create indexes for commonly queried fields
CREATE INDEX idx_aircraft_model_manufacturer_name ON aircraft_model(manufacturer_name);
CREATE INDEX idx_aircraft_model_model_name ON aircraft_model(model_name);
CREATE INDEX idx_aircraft_model_aircraft_type ON aircraft_model(aircraft_type);
CREATE INDEX idx_aircraft_model_engine_type ON aircraft_model(engine_type);
CREATE INDEX idx_aircraft_model_manufacturer_code ON aircraft_model(manufacturer_code);

-- Create trigger to automatically update updated_at on row updates
CREATE TRIGGER update_aircraft_model_updated_at
    BEFORE UPDATE ON aircraft_model
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
