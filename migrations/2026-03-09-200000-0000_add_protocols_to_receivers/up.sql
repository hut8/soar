-- safety-assured:start
ALTER TABLE receivers ADD COLUMN protocols TEXT[] NOT NULL DEFAULT '{}';
UPDATE receivers SET protocols = ARRAY[software] WHERE software IS NOT NULL;
ALTER TABLE receivers DROP COLUMN software;
-- safety-assured:end
