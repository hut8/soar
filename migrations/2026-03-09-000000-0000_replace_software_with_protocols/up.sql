ALTER TABLE receivers DROP COLUMN software;
ALTER TABLE receivers ADD COLUMN protocols TEXT[];
