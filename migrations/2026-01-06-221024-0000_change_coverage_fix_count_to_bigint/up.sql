-- Change fix_count from INTEGER to BIGINT to support large hex cells
-- Resolution 3 and 4 hexes can accumulate > 2 billion fixes in a day
ALTER TABLE receiver_coverage_h3
ALTER COLUMN fix_count TYPE BIGINT;
