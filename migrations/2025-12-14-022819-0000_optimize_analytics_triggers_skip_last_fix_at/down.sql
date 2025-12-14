-- Revert optimization - restore original trigger functions from 2025-11-17-210459-0000_add_analytics_triggers
-- This restores the triggers without the early-exit optimization, so they will execute on all UPDATEs

-- See migrations/2025-11-17-210459-0000_add_analytics_triggers/up.sql for the original versions
-- In practice, you would not want to revert this optimization, but for completeness:

-- Run the original migration to restore unoptimized triggers
\i migrations/2025-11-17-210459-0000_add_analytics_triggers/up.sql
