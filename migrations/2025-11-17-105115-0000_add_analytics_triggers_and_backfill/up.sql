-- Complete the analytics tables migration by adding trigger functions, triggers, and backfilling data
-- This completes what was started in migration 2025-11-16-015418-0000_create_analytics_tables
-- This migration is idempotent and safe to run multiple times

BEGIN;

-- ============================================================================
-- TRIGGER FUNCTIONS
-- ============================================================================

CREATE OR REPLACE FUNCTION public.update_airport_analytics_daily()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    affected_date DATE;
    old_date DATE;
BEGIN
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

    -- Handle UPDATE
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

CREATE OR REPLACE FUNCTION public.update_club_analytics_daily()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    affected_club UUID;
    affected_date DATE;
    old_club UUID;
    old_date DATE;
    flight_duration INT;
    old_duration INT;
BEGIN
    -- Handle INSERT
    IF TG_OP = 'INSERT' AND NEW.club_id IS NOT NULL THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        affected_club := NEW.club_id;
        affected_date := DATE(NEW.takeoff_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        INSERT INTO club_analytics_daily (club_id, date, club_name, flight_count, total_airtime_seconds, tow_count)
        SELECT
            NEW.club_id,
            affected_date,
            c.name,
            1,
            flight_duration,
            CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END
        FROM clubs c
        WHERE c.id = NEW.club_id
        ON CONFLICT (club_id, date) DO UPDATE SET
            flight_count = club_analytics_daily.flight_count + 1,
            total_airtime_seconds = club_analytics_daily.total_airtime_seconds + flight_duration,
            tow_count = club_analytics_daily.tow_count + CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END,
            updated_at = NOW();

    -- Handle UPDATE
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_club := OLD.club_id;
        old_date := DATE(OLD.takeoff_time);
        affected_club := NEW.club_id;
        affected_date := DATE(NEW.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        -- Remove from old club/date
        IF old_club IS NOT NULL THEN
            UPDATE club_analytics_daily SET
                flight_count = GREATEST(0, flight_count - 1),
                total_airtime_seconds = GREATEST(0, total_airtime_seconds - old_duration),
                tow_count = GREATEST(0, tow_count - CASE WHEN OLD.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END),
                updated_at = NOW()
            WHERE club_id = old_club AND date = old_date;
        END IF;

        -- Add to new club/date
        IF affected_club IS NOT NULL THEN
            INSERT INTO club_analytics_daily (club_id, date, club_name, flight_count, total_airtime_seconds, tow_count)
            SELECT
                NEW.club_id,
                affected_date,
                c.name,
                1,
                flight_duration,
                CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END
            FROM clubs c
            WHERE c.id = NEW.club_id
            ON CONFLICT (club_id, date) DO UPDATE SET
                flight_count = club_analytics_daily.flight_count + 1,
                total_airtime_seconds = club_analytics_daily.total_airtime_seconds + flight_duration,
                tow_count = club_analytics_daily.tow_count + CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END,
                updated_at = NOW();
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' AND OLD.club_id IS NOT NULL THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_club := OLD.club_id;
        old_date := DATE(OLD.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);

        UPDATE club_analytics_daily SET
            flight_count = GREATEST(0, flight_count - 1),
            total_airtime_seconds = GREATEST(0, total_airtime_seconds - old_duration),
            tow_count = GREATEST(0, tow_count - CASE WHEN OLD.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END),
            updated_at = NOW()
        WHERE club_id = old_club AND date = old_date;
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
    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        new_device := NEW.device_id;
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        INSERT INTO device_analytics (device_id, registration, aircraft_model, flight_count_total, last_flight_at, total_distance_meters)
        SELECT
            NEW.device_id,
            d.registration,
            d.aircraft_model,
            1,
            NEW.takeoff_time,
            COALESCE(NEW.total_distance_meters, 0)
        FROM devices d
        WHERE d.id = NEW.device_id
        ON CONFLICT (device_id) DO UPDATE SET
            flight_count_total = device_analytics.flight_count_total + 1,
            last_flight_at = GREATEST(device_analytics.last_flight_at, NEW.takeoff_time),
            total_distance_meters = device_analytics.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            avg_flight_duration_seconds = CASE WHEN device_analytics.flight_count_total + 1 > 0
                THEN ((device_analytics.avg_flight_duration_seconds * device_analytics.flight_count_total) + flight_duration) / (device_analytics.flight_count_total + 1)
                ELSE 0 END,
            updated_at = NOW();

    -- Handle UPDATE
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_device := OLD.device_id;
        new_device := NEW.device_id;
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
                NEW.device_id,
                d.registration,
                d.aircraft_model,
                1,
                NEW.takeoff_time,
                COALESCE(NEW.total_distance_meters, 0)
            FROM devices d
            WHERE d.id = NEW.device_id
            ON CONFLICT (device_id) DO UPDATE SET
                flight_count_total = device_analytics.flight_count_total + 1,
                last_flight_at = GREATEST(device_analytics.last_flight_at, NEW.takeoff_time),
                total_distance_meters = device_analytics.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
                updated_at = NOW();
        ELSE
            -- Same device, just update the difference
            UPDATE device_analytics SET
                total_distance_meters = total_distance_meters - COALESCE(OLD.total_distance_meters, 0) + COALESCE(NEW.total_distance_meters, 0),
                last_flight_at = GREATEST(last_flight_at, NEW.takeoff_time),
                updated_at = NOW()
            WHERE device_id = new_device;
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_device := OLD.device_id;
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);

        UPDATE device_analytics SET
            flight_count_total = GREATEST(0, flight_count_total - 1),
            total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
            avg_flight_duration_seconds = CASE WHEN flight_count_total - 1 > 0
                THEN ((avg_flight_duration_seconds * flight_count_total) - old_duration) / (flight_count_total - 1)
                ELSE 0 END,
            updated_at = NOW()
        WHERE device_id = old_device;
    END IF;

    RETURN NEW;
END;
$function$;

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
            CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END,
            CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END
        )
        ON CONFLICT (date) DO UPDATE SET
            flight_count = flight_analytics_daily.flight_count + 1,
            total_duration_seconds = flight_analytics_daily.total_duration_seconds + flight_duration,
            total_distance_meters = flight_analytics_daily.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            tow_flight_count = flight_analytics_daily.tow_flight_count + CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END,
            cross_country_count = flight_analytics_daily.cross_country_count + CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END,
            avg_duration_seconds = CASE WHEN flight_analytics_daily.flight_count + 1 > 0
                THEN (flight_analytics_daily.total_duration_seconds + flight_duration) / (flight_analytics_daily.flight_count + 1)
                ELSE 0 END,
            updated_at = NOW();

    -- Handle UPDATE
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
            tow_flight_count = GREATEST(0, tow_flight_count - CASE WHEN OLD.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END),
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
            CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END,
            CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END
        )
        ON CONFLICT (date) DO UPDATE SET
            flight_count = flight_analytics_daily.flight_count + 1,
            total_duration_seconds = flight_analytics_daily.total_duration_seconds + flight_duration,
            total_distance_meters = flight_analytics_daily.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            tow_flight_count = flight_analytics_daily.tow_flight_count + CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END,
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
            tow_flight_count = GREATEST(0, tow_flight_count - CASE WHEN OLD.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END),
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

    -- Handle UPDATE
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_hour := DATE_TRUNC('hour', OLD.takeoff_time);
        affected_hour := DATE_TRUNC('hour', NEW.takeoff_time);

        -- Remove from old hour
        UPDATE flight_analytics_hourly SET
            flight_count = GREATEST(0, flight_count - 1),
            updated_at = NOW()
        WHERE hour = old_hour;

        -- Add to new hour
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

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        affected_hour := DATE_TRUNC('hour', OLD.takeoff_time);

        UPDATE flight_analytics_hourly SET
            flight_count = GREATEST(0, flight_count - 1),
            updated_at = NOW()
        WHERE hour = affected_hour;
    END IF;

    RETURN NEW;
END;
$function$;

CREATE OR REPLACE FUNCTION public.update_flight_duration_buckets()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    old_bucket VARCHAR;
    new_bucket VARCHAR;
    old_duration INT;
    new_duration INT;
BEGIN
    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        new_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);
        IF new_duration > 0 THEN
            new_bucket := get_duration_bucket(new_duration);
            UPDATE flight_duration_buckets SET
                flight_count = flight_count + 1,
                updated_at = NOW()
            WHERE bucket_name = new_bucket;
        END IF;

    -- Handle UPDATE
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        new_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        IF old_duration > 0 THEN
            old_bucket := get_duration_bucket(old_duration);
            UPDATE flight_duration_buckets SET
                flight_count = GREATEST(0, flight_count - 1),
                updated_at = NOW()
            WHERE bucket_name = old_bucket;
        END IF;

        IF new_duration > 0 THEN
            new_bucket := get_duration_bucket(new_duration);
            UPDATE flight_duration_buckets SET
                flight_count = flight_count + 1,
                updated_at = NOW()
            WHERE bucket_name = new_bucket;
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        IF old_duration > 0 THEN
            old_bucket := get_duration_bucket(old_duration);
            UPDATE flight_duration_buckets SET
                flight_count = GREATEST(0, flight_count - 1),
                updated_at = NOW()
            WHERE bucket_name = old_bucket;
        END IF;
    END IF;

    RETURN NEW;
END;
$function$;

-- ============================================================================
-- CREATE TRIGGERS (idempotent - DROP IF EXISTS first)
-- ============================================================================

DROP TRIGGER IF EXISTS trigger_update_airport_analytics_daily ON flights;
CREATE TRIGGER trigger_update_airport_analytics_daily
    AFTER INSERT OR DELETE OR UPDATE ON flights
    FOR EACH ROW EXECUTE FUNCTION update_airport_analytics_daily();

DROP TRIGGER IF EXISTS trigger_update_club_analytics_daily ON flights;
CREATE TRIGGER trigger_update_club_analytics_daily
    AFTER INSERT OR DELETE OR UPDATE ON flights
    FOR EACH ROW EXECUTE FUNCTION update_club_analytics_daily();

DROP TRIGGER IF EXISTS trigger_update_device_analytics ON flights;
CREATE TRIGGER trigger_update_device_analytics
    AFTER INSERT OR DELETE OR UPDATE ON flights
    FOR EACH ROW EXECUTE FUNCTION update_device_analytics();

DROP TRIGGER IF EXISTS trigger_update_flight_analytics_daily ON flights;
CREATE TRIGGER trigger_update_flight_analytics_daily
    AFTER INSERT OR DELETE OR UPDATE ON flights
    FOR EACH ROW EXECUTE FUNCTION update_flight_analytics_daily();

DROP TRIGGER IF EXISTS trigger_update_flight_analytics_hourly ON flights;
CREATE TRIGGER trigger_update_flight_analytics_hourly
    AFTER INSERT OR DELETE OR UPDATE ON flights
    FOR EACH ROW EXECUTE FUNCTION update_flight_analytics_hourly();

DROP TRIGGER IF EXISTS trigger_update_flight_duration_buckets ON flights;
CREATE TRIGGER trigger_update_flight_duration_buckets
    AFTER INSERT OR DELETE OR UPDATE ON flights
    FOR EACH ROW EXECUTE FUNCTION update_flight_duration_buckets();

-- ============================================================================
-- BACKFILL ANALYTICS DATA (idempotent - uses ON CONFLICT)
-- ============================================================================

-- Temporarily disable triggers for bulk backfill to avoid redundant work
ALTER TABLE flights DISABLE TRIGGER ALL;

-- Backfill flight_analytics_daily
INSERT INTO flight_analytics_daily (date, flight_count, total_duration_seconds, avg_duration_seconds, total_distance_meters, tow_flight_count, cross_country_count)
SELECT
    DATE(takeoff_time) as date,
    COUNT(*) as flight_count,
    SUM(EXTRACT(EPOCH FROM (COALESCE(landing_time, takeoff_time) - takeoff_time))::BIGINT) as total_duration_seconds,
    AVG(EXTRACT(EPOCH FROM (COALESCE(landing_time, takeoff_time) - takeoff_time))::INT) as avg_duration_seconds,
    SUM(COALESCE(total_distance_meters, 0)) as total_distance_meters,
    SUM(CASE WHEN towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END) as tow_flight_count,
    SUM(CASE WHEN departure_airport_id IS DISTINCT FROM arrival_airport_id THEN 1 ELSE 0 END) as cross_country_count
FROM flights
WHERE takeoff_time IS NOT NULL
GROUP BY DATE(takeoff_time)
ON CONFLICT (date) DO UPDATE SET
    flight_count = EXCLUDED.flight_count,
    total_duration_seconds = EXCLUDED.total_duration_seconds,
    avg_duration_seconds = EXCLUDED.avg_duration_seconds,
    total_distance_meters = EXCLUDED.total_distance_meters,
    tow_flight_count = EXCLUDED.tow_flight_count,
    cross_country_count = EXCLUDED.cross_country_count,
    updated_at = NOW();

-- Backfill flight_duration_buckets
WITH duration_data AS (
    SELECT
        CASE
            WHEN duration_minutes < 5 THEN '0-5min'
            WHEN duration_minutes < 10 THEN '5-10min'
            WHEN duration_minutes < 15 THEN '10-15min'
            WHEN duration_minutes < 30 THEN '15-30min'
            WHEN duration_minutes < 60 THEN '30-60min'
            WHEN duration_minutes < 90 THEN '60-90min'
            WHEN duration_minutes < 120 THEN '90-120min'
            WHEN duration_minutes < 150 THEN '120-150min'
            WHEN duration_minutes < 180 THEN '150-180min'
            WHEN duration_minutes < 210 THEN '180-210min'
            WHEN duration_minutes < 240 THEN '210-240min'
            WHEN duration_minutes < 270 THEN '240-270min'
            WHEN duration_minutes < 300 THEN '270-300min'
            WHEN duration_minutes < 330 THEN '300-330min'
            WHEN duration_minutes < 360 THEN '330-360min'
            ELSE '360+min'
        END as bucket_name,
        COUNT(*) as count
    FROM (
        SELECT EXTRACT(EPOCH FROM (COALESCE(landing_time, takeoff_time) - takeoff_time))::INT / 60 as duration_minutes
        FROM flights
        WHERE takeoff_time IS NOT NULL
        AND EXTRACT(EPOCH FROM (COALESCE(landing_time, takeoff_time) - takeoff_time)) > 0
    ) durations
    GROUP BY bucket_name
)
UPDATE flight_duration_buckets fdb
SET flight_count = dd.count,
    updated_at = NOW()
FROM duration_data dd
WHERE fdb.bucket_name = dd.bucket_name;

-- Backfill flight_analytics_hourly (last 7 days only)
INSERT INTO flight_analytics_hourly (hour, flight_count, active_devices, active_clubs)
SELECT
    DATE_TRUNC('hour', takeoff_time) as hour,
    COUNT(*) as flight_count,
    COUNT(DISTINCT device_id) as active_devices,
    COUNT(DISTINCT club_id) FILTER (WHERE club_id IS NOT NULL) as active_clubs
FROM flights
WHERE takeoff_time >= NOW() - INTERVAL '7 days'
    AND takeoff_time IS NOT NULL
GROUP BY DATE_TRUNC('hour', takeoff_time)
ON CONFLICT (hour) DO UPDATE SET
    flight_count = EXCLUDED.flight_count,
    active_devices = EXCLUDED.active_devices,
    active_clubs = EXCLUDED.active_clubs,
    updated_at = NOW();

-- Backfill device_analytics
INSERT INTO device_analytics (device_id, registration, aircraft_model, flight_count_total,
                              flight_count_30d, flight_count_7d, last_flight_at,
                              avg_flight_duration_seconds, total_distance_meters)
SELECT
    f.device_id,
    d.registration,
    d.aircraft_model,
    COUNT(*) as flight_count_total,
    COUNT(*) FILTER (WHERE f.takeoff_time >= CURRENT_DATE - 30) as flight_count_30d,
    COUNT(*) FILTER (WHERE f.takeoff_time >= CURRENT_DATE - 7) as flight_count_7d,
    MAX(f.takeoff_time) as last_flight_at,
    AVG(EXTRACT(EPOCH FROM (COALESCE(f.landing_time, f.takeoff_time) - f.takeoff_time))::INT) as avg_flight_duration_seconds,
    SUM(COALESCE(f.total_distance_meters, 0)) as total_distance_meters
FROM flights f
JOIN devices d ON d.id = f.device_id
WHERE f.takeoff_time IS NOT NULL
GROUP BY f.device_id, d.registration, d.aircraft_model
ON CONFLICT (device_id) DO UPDATE SET
    registration = EXCLUDED.registration,
    aircraft_model = EXCLUDED.aircraft_model,
    flight_count_total = EXCLUDED.flight_count_total,
    flight_count_30d = EXCLUDED.flight_count_30d,
    flight_count_7d = EXCLUDED.flight_count_7d,
    last_flight_at = EXCLUDED.last_flight_at,
    avg_flight_duration_seconds = EXCLUDED.avg_flight_duration_seconds,
    total_distance_meters = EXCLUDED.total_distance_meters,
    updated_at = NOW();

-- Calculate z-scores for device analytics
WITH stats AS (
    SELECT
        AVG(flight_count_30d) as mean,
        STDDEV(flight_count_30d) as stddev
    FROM device_analytics
    WHERE flight_count_30d > 0
)
UPDATE device_analytics
SET z_score_30d = CASE
    WHEN (SELECT stddev FROM stats) > 0
    THEN (flight_count_30d - (SELECT mean FROM stats)) / (SELECT stddev FROM stats)
    ELSE 0
END,
    updated_at = NOW()
WHERE flight_count_30d > 0;

-- Backfill airport_analytics_daily
INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
SELECT
    airport_id,
    date,
    MAX(airport_ident) as airport_ident,
    MAX(airport_name) as airport_name,
    SUM(departure_count) as departure_count,
    SUM(arrival_count) as arrival_count
FROM (
    SELECT
        departure_airport_id as airport_id,
        DATE(takeoff_time) as date,
        a.ident as airport_ident,
        a.name as airport_name,
        COUNT(*) as departure_count,
        0 as arrival_count
    FROM flights f
    JOIN airports a ON a.id = f.departure_airport_id
    WHERE f.departure_airport_id IS NOT NULL
        AND f.takeoff_time IS NOT NULL
    GROUP BY f.departure_airport_id, DATE(f.takeoff_time), a.ident, a.name

    UNION ALL

    SELECT
        arrival_airport_id as airport_id,
        DATE(takeoff_time) as date,
        a.ident as airport_ident,
        a.name as airport_name,
        0 as departure_count,
        COUNT(*) as arrival_count
    FROM flights f
    JOIN airports a ON a.id = f.arrival_airport_id
    WHERE f.arrival_airport_id IS NOT NULL
        AND f.takeoff_time IS NOT NULL
    GROUP BY f.arrival_airport_id, DATE(f.takeoff_time), a.ident, a.name
) combined
GROUP BY airport_id, date
ON CONFLICT (airport_id, date) DO UPDATE SET
    airport_ident = EXCLUDED.airport_ident,
    airport_name = EXCLUDED.airport_name,
    departure_count = EXCLUDED.departure_count,
    arrival_count = EXCLUDED.arrival_count,
    updated_at = NOW();

-- Backfill club_analytics_daily
INSERT INTO club_analytics_daily (club_id, date, club_name, flight_count, active_devices, total_airtime_seconds, tow_count)
SELECT
    f.club_id,
    DATE(f.takeoff_time) as date,
    c.name as club_name,
    COUNT(*) as flight_count,
    COUNT(DISTINCT f.device_id) as active_devices,
    SUM(EXTRACT(EPOCH FROM (COALESCE(f.landing_time, f.takeoff_time) - f.takeoff_time))::BIGINT) as total_airtime_seconds,
    SUM(CASE WHEN f.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END) as tow_count
FROM flights f
JOIN clubs c ON c.id = f.club_id
WHERE f.club_id IS NOT NULL
    AND f.takeoff_time IS NOT NULL
GROUP BY f.club_id, DATE(f.takeoff_time), c.name
ON CONFLICT (club_id, date) DO UPDATE SET
    club_name = EXCLUDED.club_name,
    flight_count = EXCLUDED.flight_count,
    active_devices = EXCLUDED.active_devices,
    total_airtime_seconds = EXCLUDED.total_airtime_seconds,
    tow_count = EXCLUDED.tow_count,
    updated_at = NOW();

-- Re-enable triggers
ALTER TABLE flights ENABLE TRIGGER ALL;

COMMIT;
