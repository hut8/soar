-- Create receivers table based on the JSON schema for receivers
CREATE TABLE receivers (
    id SERIAL PRIMARY KEY,
    callsign TEXT NOT NULL UNIQUE,
    description TEXT,
    contact TEXT,
    email TEXT,
    country TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create receivers_photos table for the photos array
CREATE TABLE receivers_photos (
    id SERIAL PRIMARY KEY,
    receiver_id INTEGER NOT NULL REFERENCES receivers(id) ON DELETE CASCADE,
    photo_url TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create receivers_links table for the links array
CREATE TABLE receivers_links (
    id SERIAL PRIMARY KEY,
    receiver_id INTEGER NOT NULL REFERENCES receivers(id) ON DELETE CASCADE,
    rel TEXT NOT NULL,
    href TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for commonly queried fields
CREATE INDEX idx_receivers_callsign ON receivers(callsign);
CREATE INDEX idx_receivers_country ON receivers(country);
CREATE INDEX idx_receivers_photos_receiver_id ON receivers_photos(receiver_id);
CREATE INDEX idx_receivers_links_receiver_id ON receivers_links(receiver_id);

-- Create trigger to automatically update updated_at on row updates
CREATE TRIGGER update_receivers_updated_at
    BEFORE UPDATE ON receivers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
