# Custom PostgreSQL + PostGIS + pg_partman image for CI
FROM postgis/postgis:15-3.4

# Install pg_partman extension
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        postgresql-15-partman \
    && rm -rf /var/lib/apt/lists/*

# postgis/postgis base image already has PostGIS configured
# This image adds pg_partman on top of that
