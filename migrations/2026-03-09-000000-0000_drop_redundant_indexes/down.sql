-- safety-assured:start
CREATE INDEX idx_airspaces_source ON airspaces (source);
CREATE INDEX idx_raw_messages_receiver_id ON raw_messages (receiver_id);
-- safety-assured:end
