-- Revert premake back to 3 days

UPDATE partman.part_config
SET premake = 3
WHERE parent_table IN ('public.fixes', 'public.raw_messages');
