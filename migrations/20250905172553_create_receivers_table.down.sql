-- Drop triggers
DROP TRIGGER IF EXISTS set_updated_at_receivers ON receivers;
DROP TRIGGER IF EXISTS set_updated_at_receivers_photos ON receivers_photos;
DROP TRIGGER IF EXISTS set_updated_at_receivers_links ON receivers_links;

-- Drop tables (in reverse order due to foreign key constraints)
DROP TABLE IF EXISTS receivers_links;
DROP TABLE IF EXISTS receivers_photos;
DROP TABLE IF EXISTS receivers;
