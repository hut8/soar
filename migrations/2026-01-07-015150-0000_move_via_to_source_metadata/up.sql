-- Move the 'via' column data into the source_metadata JSONB column
-- The 'via' column contains APRS routing information (digipeater path) which is
-- specific to OGN/APRS and should be stored in source_metadata

-- Update existing rows to add 'via' to source_metadata in chunk-sized operations
-- Handle both NULL and non-NULL source_metadata cases
DO $$
DECLARE
    chunk REGCLASS;
    was_compressed BOOLEAN;
    chunk_schema_name TEXT;
    chunk_table_name TEXT;
BEGIN
    FOR chunk IN SELECT show_chunks('fixes') LOOP
        chunk_schema_name := split_part(chunk::text, '.', 1);
        chunk_table_name := split_part(chunk::text, '.', 2);

        SELECT is_compressed
        INTO was_compressed
        FROM timescaledb_information.chunks
        WHERE hypertable_name = 'fixes'
          AND chunk_schema = chunk_schema_name
          AND chunk_name = chunk_table_name;

        IF COALESCE(was_compressed, FALSE) THEN
            EXECUTE format('SELECT decompress_chunk(%L::regclass)', chunk::text);
        END IF;

        EXECUTE format($sql$
            UPDATE %s
            SET source_metadata = CASE
                -- If source_metadata is NULL, create new object with just 'via'
                WHEN source_metadata IS NULL THEN
                    jsonb_build_object('via', to_jsonb(via))
                -- If source_metadata exists, merge 'via' into it
                ELSE
                    source_metadata || jsonb_build_object('via', to_jsonb(via))
            END
        $sql$, chunk);

        IF COALESCE(was_compressed, FALSE) THEN
            EXECUTE format('SELECT compress_chunk(%L::regclass)', chunk::text);
        END IF;
    END LOOP;
END;
$$;

-- Drop the 'via' column as it's now in source_metadata
ALTER TABLE fixes DROP COLUMN via;
