-- This migration only updates function definitions, no rollback needed
-- The previous versions of the functions didn't handle NULL properly and would error
-- Rolling back would restore the broken behavior

-- No-op down migration
