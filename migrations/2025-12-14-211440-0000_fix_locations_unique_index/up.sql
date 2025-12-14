-- Fix the locations unique index to properly handle NULL values
-- The old index allows unlimited NULL values, causing massive duplication (6.4M records, only 187k unique)

-- NOTE: This migration CANNOT create the proper unique index because existing duplicates exist
-- The application code has been fixed to use COALESCE in queries to find existing locations
-- A future migration will deduplicate and create the proper unique index

-- For now, keep the existing index to at least provide some level of uniqueness checking
-- The application code changes will prevent NEW duplicates from being created

-- Add a comment to the index explaining the situation
COMMENT ON INDEX locations_address_unique_idx IS 'WARNING: This index does not properly handle NULLs and allows duplicates. Application code uses COALESCE queries. Deduplication pending.';
