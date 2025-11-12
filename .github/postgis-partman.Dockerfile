# Custom PostgreSQL + PostGIS + pg_partman image for CI
FROM postgis/postgis:17-3.5

# Install pg_partman extension
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        postgresql-17-partman \
    && rm -rf /var/lib/apt/lists/*

# postgis/postgis base image already has PostGIS configured
# This image adds pg_partman on top of that
