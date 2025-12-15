-- Rollback OpenAIP Airspace Integration

-- Drop table (cascade will remove dependent objects like indexes and triggers)
DROP TABLE IF EXISTS airspaces CASCADE;

-- Drop custom types (in reverse order of dependencies)
DROP TYPE IF EXISTS altitude_reference;
DROP TYPE IF EXISTS airspace_type;
DROP TYPE IF EXISTS airspace_class;
