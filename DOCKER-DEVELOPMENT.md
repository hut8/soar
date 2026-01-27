# Docker Development Environment

This guide explains how to use Docker Compose to set up a complete SOAR development environment with a single command.

## Quick Start

```bash
# 1. Copy the Docker environment file
cp .env.docker.example .env.docker

# 2. Edit .env.docker and add your Google Maps API key (required)
# GOOGLE_MAPS_API_KEY=your_actual_key_here

# 3. Build the custom PostgreSQL image (first time only, includes H3 extension)
docker-compose -f docker-compose.dev.yml build db

# 4. Start the development environment
docker-compose -f docker-compose.dev.yml --env-file .env.docker up

# That's it! The application is now running:
# - Frontend: http://localhost:5173
# - Backend API: http://localhost:3000
# - Database: localhost:5432
# - NATS: localhost:4222
```

## What's Included

The Docker development environment includes:

- **PostgreSQL 17 + PostGIS 3.5 + TimescaleDB + H3**: Custom-built database with all required extensions
- **NATS**: Message broker for real-time aircraft tracking
- **Rust Backend**: API server with hot reload via `cargo-watch`
- **SvelteKit Frontend**: Web UI with Vite hot module replacement

All services are configured to work together out of the box. The database image is custom-built with H3 (geospatial indexing), TimescaleDB (time-series data), and PostGIS (spatial operations).

## Services

### Database (PostgreSQL + PostGIS)
- **Port**: 5432
- **Database**: `soar_dev`
- **User**: `postgres`
- **Password**: `postgres`
- **Connection**: `postgresql://postgres:postgres@localhost:5432/soar_dev`

Data is persisted in a Docker volume (`soar_dev_postgres_data`).

### NATS
- **Client Port**: 4222
- **Monitoring**: http://localhost:8222
- **Connection**: `nats://localhost:4222`

### Backend (Rust)
- **Port**: 3000
- **API**: http://localhost:3000
- **Hot Reload**: Enabled via `cargo-watch`
- **Logs**: `docker-compose logs -f backend`

Changes to Rust files will automatically trigger a rebuild and restart.

### Frontend (SvelteKit)
- **Port**: 5173
- **URL**: http://localhost:5173
- **Hot Reload**: Enabled via Vite HMR
- **Logs**: `docker-compose logs -f frontend`

Changes to frontend files will automatically update in the browser.

## Common Commands

### Start All Services
```bash
docker-compose -f docker-compose.dev.yml --env-file .env.docker up
```

### Start in Background (Detached Mode)
```bash
docker-compose -f docker-compose.dev.yml --env-file .env.docker up -d
```

### View Logs
```bash
# All services
docker-compose -f docker-compose.dev.yml logs -f

# Specific service
docker-compose -f docker-compose.dev.yml logs -f backend
docker-compose -f docker-compose.dev.yml logs -f frontend
```

### Stop All Services
```bash
docker-compose -f docker-compose.dev.yml down
```

### Stop and Remove Volumes (Clean Slate)
```bash
docker-compose -f docker-compose.dev.yml down -v
```

### Restart a Single Service
```bash
docker-compose -f docker-compose.dev.yml restart backend
```

### Rebuild After Dependency Changes
```bash
# Rebuild backend (if Cargo.toml changed)
docker-compose -f docker-compose.dev.yml build backend

# Rebuild and restart
docker-compose -f docker-compose.dev.yml up --build backend
```

## Database Management

### Run Migrations
```bash
# Inside the backend container
docker-compose -f docker-compose.dev.yml exec backend diesel migration run

# Or run directly
docker-compose -f docker-compose.dev.yml exec backend sh -c "diesel migration run --database-url postgresql://postgres:postgres@db:5432/soar_dev"
```

### Access Database CLI
```bash
docker-compose -f docker-compose.dev.yml exec db psql -U postgres -d soar_dev
```

### Create Test Database
```bash
docker-compose -f docker-compose.dev.yml exec db createdb -U postgres soar_test
```

## Testing a Branch

To test a specific branch:

```bash
# 1. Switch to the branch
git checkout feature/my-branch

# 2. Rebuild (if backend changed)
docker-compose -f docker-compose.dev.yml build backend

# 3. Start services
docker-compose -f docker-compose.dev.yml --env-file .env.docker up

# Frontend changes don't need rebuild - just refresh browser
```

## Troubleshooting

### Database Connection Errors

If you see "could not connect to database":

```bash
# Check database health
docker-compose -f docker-compose.dev.yml ps db

# View database logs
docker-compose -f docker-compose.dev.yml logs db

# Restart database
docker-compose -f docker-compose.dev.yml restart db
```

### Backend Won't Start

```bash
# Check backend logs
docker-compose -f docker-compose.dev.yml logs backend

# Common fix: rebuild after dependency changes
docker-compose -f docker-compose.dev.yml build --no-cache backend
```

### Port Already in Use

If ports 3000, 5173, 5432, or 4222 are already in use:

```bash
# Find what's using the port
sudo lsof -i :5432

# Either stop the conflicting service or change ports in docker-compose.dev.yml
```

### Frontend Module Errors

If you see "Cannot find module" errors:

```bash
# Recreate node_modules volume
docker-compose -f docker-compose.dev.yml down -v
docker-compose -f docker-compose.dev.yml up frontend
```

### Slow Initial Build

The first build takes time because it:
- Builds custom PostgreSQL image with H3, TimescaleDB extensions (5-10 minutes)
- Downloads Rust dependencies (10-20 minutes)

Subsequent builds are faster due to Docker layer caching.

### Missing Database Extensions (H3, TimescaleDB)

If you see "extension 'h3' is not available" or similar errors:

```bash
# Rebuild the database image
docker-compose -f docker-compose.dev.yml build --no-cache db

# Recreate the database
docker-compose -f docker-compose.dev.yml down -v
docker-compose -f docker-compose.dev.yml up db
```

## Performance Tips

### Use Docker Volumes for Better Performance

The setup already uses named volumes for:
- `cargo_cache`: Rust dependency cache
- `target_cache`: Rust build artifacts
- `node_modules`: Node.js dependencies
- `postgres_data`: Database data

These persist between restarts for faster startup.

### Limit Resource Usage

If Docker is using too much CPU/memory, edit your Docker Desktop settings:
- **Memory**: 4-8 GB recommended
- **CPUs**: 2-4 cores recommended

## Development Workflow

### Typical Day

```bash
# Morning: Start environment
docker-compose -f docker-compose.dev.yml --env-file .env.docker up -d

# Work on code (hot reload handles changes)
# Backend: Edit .rs files → automatic rebuild
# Frontend: Edit .svelte files → instant HMR

# View logs when debugging
docker-compose -f docker-compose.dev.yml logs -f backend

# Evening: Stop environment
docker-compose -f docker-compose.dev.yml down
```

### Running Tests

```bash
# Rust tests
docker-compose -f docker-compose.dev.yml exec backend cargo test

# Frontend tests
docker-compose -f docker-compose.dev.yml exec frontend npm test

# E2E tests (after starting services)
cd web && npm run test:e2e
```

## Environment Variables

All configuration is in `.env.docker`. Key variables:

- `DATABASE_URL`: PostgreSQL connection string
- `NATS_URL`: NATS connection URL
- `GOOGLE_MAPS_API_KEY`: Required for operations map
- `RUST_LOG`: Logging level (info, debug, trace)

See `.env.docker.example` for all options.

## Comparison: Docker vs Manual Setup

| Task | Docker | Manual |
|------|--------|--------|
| Initial setup | `docker-compose up` | Install Postgres, NATS, Rust, Node.js, run migrations |
| Start dev env | One command | Start Postgres, NATS, backend, frontend separately |
| Switch branches | Automatic rebuild | Manual cargo build, npm install |
| Clean state | `docker-compose down -v` | Drop DB, clear caches manually |
| Share config | .env.docker file | Everyone configures separately |

## Advanced: Production-Like Testing

To test in a production-like environment:

```bash
# Use release builds
docker-compose -f docker-compose.dev.yml build --build-arg RUST_BUILD=release

# Or create a separate docker-compose.prod.yml
```

## Getting Help

- **Logs**: Always check `docker-compose logs` first
- **Status**: Use `docker-compose ps` to see service health
- **Clean Start**: `docker-compose down -v && docker-compose up` fixes most issues

## Next Steps

After the environment is running:

1. Visit http://localhost:5173 for the frontend
2. Check http://localhost:3000/health for backend status
3. Access http://localhost:8222 for NATS monitoring
4. Read the main README.md for application-specific documentation
