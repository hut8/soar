-- Delete orphaned spurious_flights whose aircraft was deleted during a merge
-- (merge_pending_registrations reassigned flights but not spurious_flights)
DELETE FROM spurious_flights
WHERE aircraft_id IS NOT NULL
  AND aircraft_id NOT IN (SELECT id FROM aircraft);

ALTER TABLE spurious_flights
    ADD CONSTRAINT fk_spurious_flights_aircraft
    FOREIGN KEY (aircraft_id) REFERENCES aircraft(id);
