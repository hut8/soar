ALTER TABLE receivers ADD COLUMN protocols TEXT[];
UPDATE receivers SET protocols = ARRAY[software] WHERE software IS NOT NULL;
