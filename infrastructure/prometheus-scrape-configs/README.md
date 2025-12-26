# Prometheus Scrape Configs (Legacy)

**⚠️ DEPRECATED:** This directory provides backward compatibility for older Prometheus configurations using `file_sd_configs`.

## Current Status

The server is currently using the OLD Prometheus configuration structure with `file_sd_configs`. These files are deployed to `/etc/prometheus/scrape-configs/` to support the existing setup.

## Migration Path

The repository has already been updated to use the NEW configuration structure with `scrape_config_files`. To complete the migration:

1. **Update Prometheus Configuration** (one-time manual step):
   ```bash
   sudo cp infrastructure/prometheus.yml /etc/prometheus/prometheus.yml
   sudo systemctl reload prometheus
   ```

2. **Verify the new configuration works**:
   ```bash
   # Check that prometheus is using /etc/prometheus/jobs/
   curl -s http://localhost:9090/api/v1/targets | jq -r '.data.activeTargets[] | select(.labels.job | contains("soar")) | .labels.job'
   ```

3. **After verification, remove legacy files**:
   ```bash
   sudo rm -rf /etc/prometheus/scrape-configs/
   ```

4. **Remove this directory from the repository** once all servers are migrated

## Why Two Directories?

- **`prometheus-jobs/`**: NEW structure (recommended) - uses `scrape_config_files` directive
- **`prometheus-scrape-configs/`**: OLD structure (legacy) - uses `file_sd_configs` directive

Both directories are deployed to maintain backward compatibility until all servers are migrated.

## Component Label Fix

These scrape-configs files have been updated to use the correct `component` labels:
- **APRS/OGN ingest**: `component: 'ingest-ogn'` (was incorrectly `aprs-ingest`)
- **Run service**: `component: 'run'`
- **Web service**: `component: 'web'`

This ensures the Grafana dashboards can properly query the metrics.

## See Also

- `prometheus-jobs/README.md` - Documentation for the NEW configuration structure
- `DEPLOYMENT.md` - General deployment documentation
