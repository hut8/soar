# Grafana Datasource Troubleshooting

## Overview

This document explains how Grafana datasource provisioning works and how to troubleshoot issues where provisioned datasources don't appear correctly.

## How Grafana Datasource Provisioning Works

### Configuration File

SOAR provisions a Prometheus datasource via: `infrastructure/grafana-provisioning/datasources/prometheus.yaml`

```yaml
apiVersion: 1

datasources:
  - name: Prometheus          # Display name in UI
    type: prometheus          # Datasource type
    uid: prometheus           # Unique identifier for API/dashboard references
    access: proxy
    url: http://localhost:9090
    isDefault: true
    editable: true
    jsonData:
      httpMethod: POST
      timeInterval: 15s
```

### Key Fields

- **name**: Display name shown in Grafana UI (e.g., "Prometheus")
- **uid**: Unique identifier used in dashboard JSON and API calls (e.g., "prometheus")
- **type**: Datasource plugin type (e.g., "prometheus", "postgres")

### Provisioning Process

1. During deployment, `soar-deploy` copies provisioning files to `/etc/grafana/provisioning/datasources/`
2. Grafana automatically reads these files on startup or periodically
3. Grafana creates/updates datasources in its SQLite database (`/var/lib/grafana/grafana.db`)
4. The datasource is stored with **both** the name and uid from the config file

## Common Issues

### Issue 1: Datasource Has Wrong UID

**Symptom**: Datasource exists with name "Prometheus" but different UID

**Cause**: Datasource was manually created in Grafana UI before provisioning

**Solution**:
```bash
# Check what's actually in the database
sudo sqlite3 /var/lib/grafana/grafana.db "SELECT id, uid, name, type FROM data_source;"

# Option 1: Delete the manually-created datasource via Grafana UI, restart Grafana
sudo systemctl restart grafana-server

# Option 2: Update the UID in the database (not recommended)
sudo sqlite3 /var/lib/grafana/grafana.db "UPDATE data_source SET uid='prometheus' WHERE LOWER(name)='prometheus';"
```

### Issue 2: Datasource Not Created At All

**Symptom**: No datasource with name "Prometheus" or uid "prometheus"

**Possible Causes**:

1. **Provisioning files not copied**: Check `/etc/grafana/provisioning/datasources/prometheus.yaml` exists
2. **Permissions issue**: Grafana user can't read the provisioning file
3. **Grafana not restarted**: Provisioning happens at startup
4. **Syntax error in YAML**: Check Grafana logs for errors

**Solution**:
```bash
# 1. Verify file exists and is readable
ls -la /etc/grafana/provisioning/datasources/
sudo -u grafana cat /etc/grafana/provisioning/datasources/prometheus.yaml

# 2. Fix permissions if needed
sudo chown -R grafana:grafana /etc/grafana/provisioning/datasources
sudo chmod 640 /etc/grafana/provisioning/datasources/*.yaml

# 3. Restart Grafana
sudo systemctl restart grafana-server

# 4. Check Grafana logs for provisioning errors
sudo journalctl -u grafana-server -n 100 --no-pager | grep -i provision
```

### Issue 3: Case Sensitivity Issues

**Note**: The datasource **name** is case-sensitive in the database and UI, but dashboards reference the **uid** which should be lowercase.

- Provisioning file: `name: Prometheus` (capital P), `uid: prometheus` (lowercase)
- Database stores: Both name and uid exactly as specified
- Dashboards reference: The uid (lowercase "prometheus")

## Deployment Validation

The `soar-deploy` script validates datasources after deployment:

1. **First check**: Look for exact UID match (`uid='prometheus'`)
2. **Fallback check**: If not found, search by name (case-insensitive)
3. **Result**:
   - If found by UID: ✅ Success
   - If found by name only: ⚠️ Warning (indicates UID mismatch)
   - If not found at all: ⚠️ Warning (indicates provisioning issue)

**Warnings do not block deployment** - they inform you of potential issues that need manual resolution.

## Debugging Commands

```bash
# List all datasources in Grafana database
sudo sqlite3 /var/lib/grafana/grafana.db "SELECT id, uid, name, type FROM data_source;"

# Check if prometheus datasource exists by UID
sudo sqlite3 /var/lib/grafana/grafana.db "SELECT uid, name FROM data_source WHERE uid='prometheus';"

# Check if prometheus datasource exists by name (case-insensitive)
sudo sqlite3 /var/lib/grafana/grafana.db "SELECT uid, name FROM data_source WHERE LOWER(name)='prometheus';"

# Check provisioning file permissions
ls -la /etc/grafana/provisioning/datasources/

# Check Grafana service status
sudo systemctl status grafana-server

# View recent Grafana logs
sudo journalctl -u grafana-server -n 50 --no-pager

# Test if Prometheus is running (datasource target)
curl http://localhost:9090/api/v1/status/config
```

## Expected State

After successful deployment, you should see:

```bash
$ sudo sqlite3 /var/lib/grafana/grafana.db "SELECT uid, name, type FROM data_source WHERE uid='prometheus';"
prometheus|Prometheus|prometheus
```

This shows:
- UID: `prometheus` (lowercase) - matches provisioning config ✅
- Name: `Prometheus` (capital P) - matches provisioning config ✅
- Type: `prometheus` - matches provisioning config ✅

## Related Files

- **Provisioning Config**: `infrastructure/grafana-provisioning/datasources/prometheus.yaml`
- **Deployment Script**: `infrastructure/soar-deploy` (datasource validation at ~line 647)
- **Grafana Config**: `/etc/grafana/grafana.ini` (on server)
- **Grafana Database**: `/var/lib/grafana/grafana.db` (on server)
- **Provisioned Files**: `/etc/grafana/provisioning/datasources/` (on server)

## References

- [Grafana Provisioning Documentation](https://grafana.com/docs/grafana/latest/administration/provisioning/)
- [Grafana Datasource Configuration](https://grafana.com/docs/grafana/latest/administration/provisioning/#data-sources)
