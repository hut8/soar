-- Create the new aircraft_approved_operations table
CREATE TABLE aircraft_approved_operations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    aircraft_registration_id VARCHAR(6) NOT NULL,
    operation VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_aircraft_registration
        FOREIGN KEY (aircraft_registration_id)
        REFERENCES aircraft_registrations(registration_number)
        ON DELETE CASCADE
);

-- Create index for faster lookups by registration
CREATE INDEX idx_aircraft_approved_operations_registration_id
    ON aircraft_approved_operations(aircraft_registration_id);

-- Drop all op_* columns from aircraft_registrations
ALTER TABLE aircraft_registrations
    DROP COLUMN op_restricted_other,
    DROP COLUMN op_restricted_ag_pest_control,
    DROP COLUMN op_restricted_aerial_surveying,
    DROP COLUMN op_restricted_aerial_advertising,
    DROP COLUMN op_restricted_forest,
    DROP COLUMN op_restricted_patrolling,
    DROP COLUMN op_restricted_weather_control,
    DROP COLUMN op_restricted_carriage_of_cargo,
    DROP COLUMN op_experimental_show_compliance,
    DROP COLUMN op_experimental_research_development,
    DROP COLUMN op_experimental_amateur_built,
    DROP COLUMN op_experimental_exhibition,
    DROP COLUMN op_experimental_racing,
    DROP COLUMN op_experimental_crew_training,
    DROP COLUMN op_experimental_market_survey,
    DROP COLUMN op_experimental_operating_kit_built,
    DROP COLUMN op_experimental_light_sport_reg_prior_2008,
    DROP COLUMN op_experimental_light_sport_operating_kit_built,
    DROP COLUMN op_experimental_light_sport_prev_21_190,
    DROP COLUMN op_experimental_uas_research_development,
    DROP COLUMN op_experimental_uas_market_survey,
    DROP COLUMN op_experimental_uas_crew_training,
    DROP COLUMN op_experimental_uas_exhibition,
    DROP COLUMN op_experimental_uas_compliance_with_cfr,
    DROP COLUMN op_sfp_ferry_for_repairs_alterations_storage,
    DROP COLUMN op_sfp_evacuate_impending_danger,
    DROP COLUMN op_sfp_excess_of_max_certificated,
    DROP COLUMN op_sfp_delivery_or_export,
    DROP COLUMN op_sfp_production_flight_testing,
    DROP COLUMN op_sfp_customer_demo;
