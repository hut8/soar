ALTER TABLE spurious_flights
    ADD CONSTRAINT fk_spurious_flights_aircraft
    FOREIGN KEY (aircraft_id) REFERENCES aircraft(id);
