-- Optimize analytics triggers to skip execution when only non-analytics fields change
-- This dramatically improves performance for the common case of updating last_fix_at during active flights
--
-- Analytics-relevant fields that we check:
-- - takeoff_time, landing_time (when the flight started/ended)
-- - aircraft_id (which device/aircraft)
-- - club_id (which club owns the aircraft)
-- - departure_airport_id, arrival_airport_id (where the flight went)
-- - towed_by_aircraft_id (towing relationships)
-- - total_distance_meters (flight distance metrics)
--
-- Non-analytics fields that can change without triggering analytics updates:
-- - last_fix_at (updated on every fix for an active flight)
-- - timed_out_at (timeout tracking)
-- - callsign (doesn't affect aggregations)
-- - runway/location fields (used for display, not analytics)
-- - altitude offset fields (used for display)

CREATE OR REPLACE FUNCTION public.update_flight_analytics_daily()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    affected_date DATE;
    old_date DATE;
    flight_duration INT;
    old_duration INT;
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.landing_time IS NOT DISTINCT FROM NEW.landing_time AND
       OLD.aircraft_id IS NOT DISTINCT FROM NEW.aircraft_id AND
       OLD.club_id IS NOT DISTINCT FROM NEW.club_id AND
       OLD.departure_airport_id IS NOT DISTINCT FROM NEW.departure_airport_id AND
       OLD.arrival_airport_id IS NOT DISTINCT FROM NEW.arrival_airport_id AND
       OLD.towed_by_aircraft_id IS NOT DISTINCT FROM NEW.towed_by_aircraft_id AND
       OLD.total_distance_meters IS NOT DISTINCT FROM NEW.total_distance_meters
    THEN
        RETURN NEW;  -- Skip analytics update - only last_fix_at or other non-analytics fields changed
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        affected_date := DATE(NEW.takeoff_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        INSERT INTO flight_analytics_daily (date, flight_count, total_duration_seconds, total_distance_meters, tow_flight_count, cross_country_count)
        VALUES (
            affected_date,
            1,
            flight_duration,
            COALESCE(NEW.total_distance_meters, 0),
            CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END,
            CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END
        )
        ON CONFLICT (date) DO UPDATE SET
            flight_count = flight_analytics_daily.flight_count + 1,
            total_duration_seconds = flight_analytics_daily.total_duration_seconds + flight_duration,
            total_distance_meters = flight_analytics_daily.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            tow_flight_count = flight_analytics_daily.tow_flight_count + CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END,
            cross_country_count = flight_analytics_daily.cross_country_count + CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END,
            avg_duration_seconds = CASE WHEN flight_analytics_daily.flight_count + 1 > 0
                THEN (flight_analytics_daily.total_duration_seconds + flight_duration) / (flight_analytics_daily.flight_count + 1)
                ELSE 0 END,
            updated_at = NOW();

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_date := DATE(OLD.takeoff_time);
        affected_date := DATE(NEW.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        -- Remove old values
        UPDATE flight_analytics_daily SET
            flight_count = GREATEST(0, flight_count - 1),
            total_duration_seconds = GREATEST(0, total_duration_seconds - old_duration),
            total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
            tow_flight_count = GREATEST(0, tow_flight_count - CASE WHEN OLD.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END),
            cross_country_count = GREATEST(0, cross_country_count - CASE WHEN OLD.departure_airport_id IS DISTINCT FROM OLD.arrival_airport_id THEN 1 ELSE 0 END),
            avg_duration_seconds = CASE WHEN flight_count - 1 > 0
                THEN (total_duration_seconds - old_duration) / (flight_count - 1)
                ELSE 0 END,
            updated_at = NOW()
        WHERE date = old_date;

        -- Add new values
        INSERT INTO flight_analytics_daily (date, flight_count, total_duration_seconds, total_distance_meters, tow_flight_count, cross_country_count)
        VALUES (
            affected_date,
            1,
            flight_duration,
            COALESCE(NEW.total_distance_meters, 0),
            CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END,
            CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END
        )
        ON CONFLICT (date) DO UPDATE SET
            flight_count = flight_analytics_daily.flight_count + 1,
            total_duration_seconds = flight_analytics_daily.total_duration_seconds + flight_duration,
            total_distance_meters = flight_analytics_daily.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            tow_flight_count = flight_analytics_daily.tow_flight_count + CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END,
            cross_country_count = flight_analytics_daily.cross_country_count + CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END,
            avg_duration_seconds = CASE WHEN flight_analytics_daily.flight_count + 1 > 0
                THEN (flight_analytics_daily.total_duration_seconds + flight_duration) / (flight_analytics_daily.flight_count + 1)
                ELSE 0 END,
            updated_at = NOW();

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        affected_date := DATE(OLD.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);

        UPDATE flight_analytics_daily SET
            flight_count = GREATEST(0, flight_count - 1),
            total_duration_seconds = GREATEST(0, total_duration_seconds - old_duration),
            total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
            tow_flight_count = GREATEST(0, tow_flight_count - CASE WHEN OLD.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END),
            cross_country_count = GREATEST(0, cross_country_count - CASE WHEN OLD.departure_airport_id IS DISTINCT FROM OLD.arrival_airport_id THEN 1 ELSE 0 END),
            avg_duration_seconds = CASE WHEN flight_count - 1 > 0
                THEN (total_duration_seconds - old_duration) / (flight_count - 1)
                ELSE 0 END,
            updated_at = NOW()
        WHERE date = affected_date;
    END IF;

    RETURN NEW;
END;
$function$;

CREATE OR REPLACE FUNCTION public.update_flight_analytics_hourly()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    affected_hour TIMESTAMPTZ;
    old_hour TIMESTAMPTZ;
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.landing_time IS NOT DISTINCT FROM NEW.landing_time AND
       OLD.aircraft_id IS NOT DISTINCT FROM NEW.aircraft_id AND
       OLD.club_id IS NOT DISTINCT FROM NEW.club_id
    THEN
        RETURN NEW;  -- Skip analytics update
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        affected_hour := DATE_TRUNC('hour', NEW.takeoff_time);

        INSERT INTO flight_analytics_hourly (hour, flight_count, active_devices, active_clubs)
        VALUES (
            affected_hour,
            1,
            1,
            CASE WHEN NEW.club_id IS NOT NULL THEN 1 ELSE 0 END
        )
        ON CONFLICT (hour) DO UPDATE SET
            flight_count = flight_analytics_hourly.flight_count + 1,
            updated_at = NOW();

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_hour := DATE_TRUNC('hour', OLD.takeoff_time);
        affected_hour := DATE_TRUNC('hour', NEW.takeoff_time);

        -- If hour changed, remove from old and add to new
        IF old_hour != affected_hour THEN
            UPDATE flight_analytics_hourly SET
                flight_count = GREATEST(0, flight_count - 1),
                updated_at = NOW()
            WHERE hour = old_hour;

            INSERT INTO flight_analytics_hourly (hour, flight_count, active_devices, active_clubs)
            VALUES (
                affected_hour,
                1,
                1,
                CASE WHEN NEW.club_id IS NOT NULL THEN 1 ELSE 0 END
            )
            ON CONFLICT (hour) DO UPDATE SET
                flight_count = flight_analytics_hourly.flight_count + 1,
                updated_at = NOW();
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_hour := DATE_TRUNC('hour', OLD.takeoff_time);

        UPDATE flight_analytics_hourly SET
            flight_count = GREATEST(0, flight_count - 1),
            updated_at = NOW()
        WHERE hour = old_hour;
    END IF;

    RETURN NEW;
END;
$function$;

CREATE OR REPLACE FUNCTION public.update_flight_duration_buckets()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    flight_duration INT;
    old_duration INT;
    bucket VARCHAR(20);
    old_bucket VARCHAR(20);
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.landing_time IS NOT DISTINCT FROM NEW.landing_time
    THEN
        RETURN NEW;  -- Skip analytics update
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);
        bucket := get_flight_duration_bucket(flight_duration);

        UPDATE flight_duration_buckets SET
            flight_count = flight_count + 1,
            updated_at = NOW()
        WHERE bucket_name = bucket;

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);
        old_bucket := get_flight_duration_bucket(old_duration);
        bucket := get_flight_duration_bucket(flight_duration);

        -- If bucket changed, remove from old and add to new
        IF old_bucket != bucket THEN
            UPDATE flight_duration_buckets SET
                flight_count = GREATEST(0, flight_count - 1),
                updated_at = NOW()
            WHERE bucket_name = old_bucket;

            UPDATE flight_duration_buckets SET
                flight_count = flight_count + 1,
                updated_at = NOW()
            WHERE bucket_name = bucket;
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        old_bucket := get_flight_duration_bucket(old_duration);

        UPDATE flight_duration_buckets SET
            flight_count = GREATEST(0, flight_count - 1),
            updated_at = NOW()
        WHERE bucket_name = old_bucket;
    END IF;

    RETURN NEW;
END;
$function$;

CREATE OR REPLACE FUNCTION public.update_device_analytics()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    old_device UUID;
    new_device UUID;
    flight_duration INT;
    old_duration INT;
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.landing_time IS NOT DISTINCT FROM NEW.landing_time AND
       OLD.aircraft_id IS NOT DISTINCT FROM NEW.aircraft_id AND
       OLD.total_distance_meters IS NOT DISTINCT FROM NEW.total_distance_meters
    THEN
        RETURN NEW;  -- Skip analytics update
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        new_device := NEW.aircraft_id;
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        INSERT INTO device_analytics (device_id, registration, aircraft_model, flight_count_total, last_flight_at, total_distance_meters)
        SELECT
            NEW.aircraft_id,
            d.registration,
            d.aircraft_model,
            1,
            NEW.takeoff_time,
            COALESCE(NEW.total_distance_meters, 0)
        FROM devices d
        WHERE d.id = NEW.aircraft_id
        ON CONFLICT (device_id) DO UPDATE SET
            flight_count_total = device_analytics.flight_count_total + 1,
            last_flight_at = GREATEST(device_analytics.last_flight_at, NEW.takeoff_time),
            total_distance_meters = device_analytics.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            avg_flight_duration_seconds = CASE WHEN device_analytics.flight_count_total + 1 > 0
                THEN ((device_analytics.avg_flight_duration_seconds * device_analytics.flight_count_total) + flight_duration) / (device_analytics.flight_count_total + 1)
                ELSE 0 END,
            updated_at = NOW();

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_device := OLD.aircraft_id;
        new_device := NEW.aircraft_id;
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        -- If device changed, update both
        IF old_device != new_device THEN
            -- Remove from old device
            UPDATE device_analytics SET
                flight_count_total = GREATEST(0, flight_count_total - 1),
                total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
                updated_at = NOW()
            WHERE device_id = old_device;

            -- Add to new device
            INSERT INTO device_analytics (device_id, registration, aircraft_model, flight_count_total, last_flight_at, total_distance_meters)
            SELECT
                NEW.aircraft_id,
                d.registration,
                d.aircraft_model,
                1,
                NEW.takeoff_time,
                COALESCE(NEW.total_distance_meters, 0)
            FROM devices d
            WHERE d.id = NEW.aircraft_id
            ON CONFLICT (device_id) DO UPDATE SET
                flight_count_total = device_analytics.flight_count_total + 1,
                last_flight_at = GREATEST(device_analytics.last_flight_at, NEW.takeoff_time),
                total_distance_meters = device_analytics.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
                avg_flight_duration_seconds = CASE WHEN device_analytics.flight_count_total + 1 > 0
                    THEN ((device_analytics.avg_flight_duration_seconds * device_analytics.flight_count_total) + flight_duration) / (device_analytics.flight_count_total + 1)
                    ELSE 0 END,
                updated_at = NOW();
        ELSE
            -- Same device, just update distance if changed
            IF OLD.total_distance_meters IS DISTINCT FROM NEW.total_distance_meters THEN
                UPDATE device_analytics SET
                    total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0) + COALESCE(NEW.total_distance_meters, 0)),
                    updated_at = NOW()
                WHERE device_id = new_device;
            END IF;
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_device := OLD.aircraft_id;

        UPDATE device_analytics SET
            flight_count_total = GREATEST(0, flight_count_total - 1),
            total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
            updated_at = NOW()
        WHERE device_id = old_device;
    END IF;

    RETURN NEW;
END;
$function$;

CREATE OR REPLACE FUNCTION public.update_club_analytics_daily()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    affected_date DATE;
    old_date DATE;
    old_club UUID;
    new_club UUID;
    flight_duration INT;
    old_duration INT;
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.landing_time IS NOT DISTINCT FROM NEW.landing_time AND
       OLD.club_id IS NOT DISTINCT FROM NEW.club_id AND
       OLD.towed_by_aircraft_id IS NOT DISTINCT FROM NEW.towed_by_aircraft_id
    THEN
        RETURN NEW;  -- Skip analytics update
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time or club
        IF NEW.takeoff_time IS NULL OR NEW.club_id IS NULL THEN
            RETURN NEW;
        END IF;

        affected_date := DATE(NEW.takeoff_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        INSERT INTO club_analytics_daily (club_id, date, club_name, flight_count, total_airtime_seconds, tow_count)
        SELECT
            NEW.club_id,
            affected_date,
            c.name,
            1,
            flight_duration,
            CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END
        FROM clubs c
        WHERE c.id = NEW.club_id
        ON CONFLICT (club_id, date) DO UPDATE SET
            flight_count = club_analytics_daily.flight_count + 1,
            total_airtime_seconds = club_analytics_daily.total_airtime_seconds + flight_duration,
            tow_count = club_analytics_daily.tow_count + CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END,
            updated_at = NOW();

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new are missing required fields
        IF (OLD.takeoff_time IS NULL OR OLD.club_id IS NULL) AND (NEW.takeoff_time IS NULL OR NEW.club_id IS NULL) THEN
            RETURN NEW;
        END IF;

        old_club := OLD.club_id;
        new_club := NEW.club_id;
        old_date := DATE(OLD.takeoff_time);
        affected_date := DATE(NEW.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        -- Remove old values if club was set
        IF OLD.club_id IS NOT NULL THEN
            UPDATE club_analytics_daily SET
                flight_count = GREATEST(0, flight_count - 1),
                total_airtime_seconds = GREATEST(0, total_airtime_seconds - old_duration),
                tow_count = GREATEST(0, tow_count - CASE WHEN OLD.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END),
                updated_at = NOW()
            WHERE club_id = old_club AND date = old_date;
        END IF;

        -- Add new values if club is set
        IF NEW.club_id IS NOT NULL THEN
            INSERT INTO club_analytics_daily (club_id, date, club_name, flight_count, total_airtime_seconds, tow_count)
            SELECT
                NEW.club_id,
                affected_date,
                c.name,
                1,
                flight_duration,
                CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END
            FROM clubs c
            WHERE c.id = NEW.club_id
            ON CONFLICT (club_id, date) DO UPDATE SET
                flight_count = club_analytics_daily.flight_count + 1,
                total_airtime_seconds = club_analytics_daily.total_airtime_seconds + flight_duration,
                tow_count = club_analytics_daily.tow_count + CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END,
                updated_at = NOW();
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time or club
        IF OLD.takeoff_time IS NULL OR OLD.club_id IS NULL THEN
            RETURN OLD;
        END IF;

        old_club := OLD.club_id;
        old_date := DATE(OLD.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);

        UPDATE club_analytics_daily SET
            flight_count = GREATEST(0, flight_count - 1),
            total_airtime_seconds = GREATEST(0, total_airtime_seconds - old_duration),
            tow_count = GREATEST(0, tow_count - CASE WHEN OLD.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END),
            updated_at = NOW()
        WHERE club_id = old_club AND date = old_date;
    END IF;

    RETURN NEW;
END;
$function$;

CREATE OR REPLACE FUNCTION public.update_airport_analytics_daily()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    affected_date DATE;
    old_date DATE;
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.departure_airport_id IS NOT DISTINCT FROM NEW.departure_airport_id AND
       OLD.arrival_airport_id IS NOT DISTINCT FROM NEW.arrival_airport_id
    THEN
        RETURN NEW;  -- Skip analytics update
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        affected_date := DATE(NEW.takeoff_time);

        -- Update departure airport
        IF NEW.departure_airport_id IS NOT NULL THEN
            INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
            SELECT
                NEW.departure_airport_id,
                affected_date,
                a.ident,
                a.name,
                1,
                0
            FROM airports a
            WHERE a.id = NEW.departure_airport_id
            ON CONFLICT (airport_id, date) DO UPDATE SET
                departure_count = airport_analytics_daily.departure_count + 1,
                updated_at = NOW();
        END IF;

        -- Update arrival airport
        IF NEW.arrival_airport_id IS NOT NULL THEN
            INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
            SELECT
                NEW.arrival_airport_id,
                affected_date,
                a.ident,
                a.name,
                0,
                1
            FROM airports a
            WHERE a.id = NEW.arrival_airport_id
            ON CONFLICT (airport_id, date) DO UPDATE SET
                arrival_count = airport_analytics_daily.arrival_count + 1,
                updated_at = NOW();
        END IF;

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_date := DATE(OLD.takeoff_time);
        affected_date := DATE(NEW.takeoff_time);

        -- Remove old departure
        IF OLD.departure_airport_id IS NOT NULL THEN
            UPDATE airport_analytics_daily SET
                departure_count = GREATEST(0, departure_count - 1),
                updated_at = NOW()
            WHERE airport_id = OLD.departure_airport_id AND date = old_date;
        END IF;

        -- Remove old arrival
        IF OLD.arrival_airport_id IS NOT NULL THEN
            UPDATE airport_analytics_daily SET
                arrival_count = GREATEST(0, arrival_count - 1),
                updated_at = NOW()
            WHERE airport_id = OLD.arrival_airport_id AND date = old_date;
        END IF;

        -- Add new departure
        IF NEW.departure_airport_id IS NOT NULL THEN
            INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
            SELECT
                NEW.departure_airport_id,
                affected_date,
                a.ident,
                a.name,
                1,
                0
            FROM airports a
            WHERE a.id = NEW.departure_airport_id
            ON CONFLICT (airport_id, date) DO UPDATE SET
                departure_count = airport_analytics_daily.departure_count + 1,
                updated_at = NOW();
        END IF;

        -- Add new arrival
        IF NEW.arrival_airport_id IS NOT NULL THEN
            INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
            SELECT
                NEW.arrival_airport_id,
                affected_date,
                a.ident,
                a.name,
                0,
                1
            FROM airports a
            WHERE a.id = NEW.arrival_airport_id
            ON CONFLICT (airport_id, date) DO UPDATE SET
                arrival_count = airport_analytics_daily.arrival_count + 1,
                updated_at = NOW();
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_date := DATE(OLD.takeoff_time);

        -- Remove departure
        IF OLD.departure_airport_id IS NOT NULL THEN
            UPDATE airport_analytics_daily SET
                departure_count = GREATEST(0, departure_count - 1),
                updated_at = NOW()
            WHERE airport_id = OLD.departure_airport_id AND date = old_date;
        END IF;

        -- Remove arrival
        IF OLD.arrival_airport_id IS NOT NULL THEN
            UPDATE airport_analytics_daily SET
                arrival_count = GREATEST(0, arrival_count - 1),
                updated_at = NOW()
            WHERE airport_id = OLD.arrival_airport_id AND date = old_date;
        END IF;
    END IF;

    RETURN NEW;
END;
$function$;
