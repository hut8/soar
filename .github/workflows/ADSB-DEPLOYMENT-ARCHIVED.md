# ADS-B Deployment Workflow - ARCHIVED

## Overview

The `deploy-adsb.yml` workflow has been archived (renamed to `deploy-adsb.yml.archive`) because the deployment architecture has changed.

## Previous Architecture

- **ADSB Ingester**: Deployed to a separate ARM64 server via Tailscale
- **Deployment Method**: GitHub Actions workflow connected via Tailscale SSH
- **Beast Server**: `out.adsb.lol:1365`

## New Architecture

- **ADSB Ingester**: Deployed directly on production/staging servers (same as other services)
- **Deployment Method**: Regular production/staging deployment process
- **Beast Server**: `radar:41365`

## Changes Made

1. **Service Files Updated**:
   - `infrastructure/systemd/soar-beast-ingest.service` - Now connects to `radar:41365`
   - `infrastructure/systemd/soar-beast-ingest-staging.service` - Now connects to `radar:41365`

2. **NATS Connection**:
   - Previous: Connected to main server via Tailscale (`nats://<tailscale-ip>:4222`)
   - New: Connects to localhost (`nats://localhost:4222`) since it runs on the same server

3. **Deployment Process**:
   - Previous: Separate workflow deployed to separate ADSB server
   - New: Deployed via regular production/staging deployment workflows

## Migration Notes

- No need to connect to or deploy to the "radar" ADSB server anymore
- The beast ingest services are now part of the main production/staging infrastructure
- All deployment is handled through the standard CI/CD pipeline
