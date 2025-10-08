-- Revert GNSS resolution fields back to satellite fields

ALTER TABLE fixes RENAME COLUMN gnss_horizontal_resolution TO satellites_used;
ALTER TABLE fixes RENAME COLUMN gnss_vertical_resolution TO satellites_visible;
