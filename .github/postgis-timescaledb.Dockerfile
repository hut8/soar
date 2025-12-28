# Custom PostgreSQL + PostGIS + TimescaleDB image for CI
FROM postgis/postgis:17-3.5

# Install dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        curl \
        ca-certificates \
        gnupg \
        lsb-release \
    && rm -rf /var/lib/apt/lists/*

# Add TimescaleDB repository
RUN mkdir -p /usr/share/keyrings && \
    curl -fsSL https://packagecloud.io/timescale/timescaledb/gpgkey | \
    gpg --dearmor -o /usr/share/keyrings/timescaledb.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/timescaledb.gpg] https://packagecloud.io/timescale/timescaledb/debian/ $(lsb_release -cs) main" | \
    tee /etc/apt/sources.list.d/timescaledb.list

# Install TimescaleDB, H3, and pg_partman extensions
# NOTE: pg_partman is kept for CI migration history (old migrations depend on it)
# even though production has migrated to TimescaleDB
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        postgresql-17-h3 \
        postgresql-17-partman \
        timescaledb-2-postgresql-17 \
        timescaledb-toolkit-postgresql-17 \
        timescaledb-tools \
    && rm -rf /var/lib/apt/lists/*

# Configure PostgreSQL to load TimescaleDB
RUN echo "shared_preload_libraries = 'timescaledb'" >> /usr/share/postgresql/postgresql.conf.sample

# postgis/postgis base image already has PostGIS configured
# This image adds TimescaleDB, H3, and pg_partman (for migration history) on top of that
