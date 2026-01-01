-- Increase partman premake from 3 to 7 days
--
-- Context: December 2025 incident where connection exhaustion prevented partman
-- maintenance from running, causing 3 days of missing partitions and data
-- accumulation in DEFAULT partitions.
--
-- Increasing premake from 3 to 7 days provides additional buffer if partman
-- maintenance fails temporarily, reducing risk of partition gaps.
--
-- premake = 7 means partitions are created 7 days in advance
-- Example: On Dec 21, partitions exist for Dec 21-28

UPDATE partman.part_config
SET premake = 7
WHERE parent_table IN ('public.fixes', 'public.raw_messages');
