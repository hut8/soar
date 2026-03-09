ALTER TABLE receivers ADD COLUMN software TEXT;
UPDATE receivers SET software = protocols[1] WHERE protocols IS NOT NULL AND array_length(protocols, 1) > 0;
ALTER TABLE receivers DROP COLUMN protocols;
