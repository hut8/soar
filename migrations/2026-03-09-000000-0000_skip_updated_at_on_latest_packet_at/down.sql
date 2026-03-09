-- Restore the generic trigger
DROP TRIGGER IF EXISTS update_receivers_updated_at ON receivers;
CREATE TRIGGER update_receivers_updated_at
    BEFORE UPDATE ON receivers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Remove the receivers-specific function
DROP FUNCTION IF EXISTS update_receivers_updated_at_column();
