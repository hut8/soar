# SOAR Grafana Alerting Configuration

This document describes the Grafana alerting system for monitoring SOAR metrics and sending email notifications.

## Overview

SOAR uses **Grafana Alerting** (built into Grafana 8+) to monitor critical metrics and send email notifications when issues are detected. The alerting system is automatically configured during deployment via the `soar-deploy` script.

### Key Features

- ✅ **Email Notifications** - Alerts sent via SMTP (Mailgun)
- ✅ **Automatic Configuration** - SMTP credentials extracted from `/etc/soar/env` during deployment
- ✅ **Repeat Notifications** - Configurable repeat intervals (critical: 30min, warning: 1hr)
- ✅ **Infrastructure as Code** - Alert rules managed in version control
- ✅ **No Credential Commits** - Credentials never stored in repository

## Alert Rules

### OGN/APRS Message Ingestion Rate Too Low
- **Severity**: Critical
- **Condition**: Message ingestion rate drops below 1 message/minute
- **Duration**: Alert if condition persists for 2 minutes
- **Metric**: `rate(aprs_raw_message_processed_total[1m]) * 60`
- **Repeat Interval**: 12 hours
- **Description**: Monitors the rate of APRS messages being processed. If the rate drops below 1 msg/min, it indicates the OGN ingest service may be disconnected or experiencing issues.

### OGN Ingest Service Disconnected
- **Severity**: Critical
- **Condition**: APRS connection gauge is 0 (disconnected)
- **Duration**: Alert if condition persists for 1 minute
- **Metric**: `aprs_connection_connected < 0.5`
- **Repeat Interval**: 12 hours
- **Description**: Monitors the connection status to APRS-IS. Alerts when the service loses connection.

### OGN NATS Publishing Errors
- **Severity**: Warning
- **Condition**: NATS publish error rate exceeds 0.1 errors/minute
- **Duration**: Alert if condition persists for 3 minutes
- **Metric**: `rate(aprs_nats_publish_error_total[5m]) * 60`
- **Repeat Interval**: 24 hours
- **Description**: Monitors errors when publishing messages to NATS. Indicates potential issues with NATS connectivity or queue depth.

## Configuration Files

### Template Files (Committed to Repository)

These files contain placeholders and are processed during deployment:

```
infrastructure/
├── grafana-provisioning/
│   └── alerting/
│       ├── contact-points.yml.template      # Email contact configuration (has placeholders)
│       ├── notification-policies.yml        # Routing and repeat intervals (no placeholders)
│       ├── alert-rules-production.yml       # Production alert rules (deployed to production only)
│       └── alert-rules-staging.yml          # Staging alert rules (deployed to staging only)
└── systemd/
    └── grafana-server.service.d-smtp.conf.template  # SMTP config for Grafana (has placeholders)
```

**Placeholders in Templates:**
- `{{ALERT_EMAIL}}` - Alert recipient email (from `EMAIL_TO` in env file)
- `{{SMTP_SERVER}}` - SMTP server hostname (from `SMTP_SERVER` in env file)
- `{{SMTP_PORT}}` - SMTP port (from `SMTP_PORT` in env file)
- `{{SMTP_USERNAME}}` - SMTP username (from `SMTP_USERNAME` in env file)
- `{{SMTP_PASSWORD}}` - SMTP password (from `SMTP_PASSWORD` in env file)
- `{{FROM_EMAIL}}` - Sender email address (from `FROM_EMAIL` in env file)
- `{{FROM_NAME}}` - Sender display name (from `FROM_NAME` in env file)

### Generated Files (NOT Committed)

These files are generated during deployment with actual credentials:

```
/etc/grafana/provisioning/alerting/
├── contact-points.yml              # Processed from template
├── notification-policies.yml       # Copied as-is
└── alert-rules-{environment}.yml   # Environment-specific (production or staging)

/etc/systemd/system/grafana-server.service.d/
└── smtp.conf                     # Processed from template
```

**Important**: Generated files are excluded from git via `.gitignore` to prevent committing credentials.

## Deployment

### Automatic Deployment

Alerting configuration is automatically deployed by the `soar-deploy` script:

```bash
# Production deployment
sudo /usr/local/bin/soar-deploy production /tmp/soar/deploy/YYYYMMDDHHMMSS

# Staging deployment
sudo /usr/local/bin/soar-deploy staging /tmp/soar/deploy/YYYYMMDDHHMMSS
```

### What Happens During Deployment

1. **Extract SMTP Config**: Script sources `/etc/soar/env` (or `/etc/soar/env-staging`) to get SMTP credentials
2. **Process Templates**: Substitutes placeholders in template files with actual values
3. **Install Config Files**: Copies processed files to `/etc/grafana/provisioning/alerting/`
4. **Install Environment-Specific Alerts**: Only installs alert rules matching the deployment environment (production or staging)
5. **Configure SMTP**: Creates systemd drop-in file with Grafana SMTP environment variables
6. **Reload Services**: Runs `systemctl daemon-reload` and restarts Grafana

### Environment File Variables

Required variables in `/etc/soar/env` or `/etc/soar/env-staging`:

```bash
# Alert recipient
EMAIL_TO=liam@supervillains.io

# SMTP configuration
SMTP_SERVER=smtp.mailgun.org
SMTP_PORT=465
SMTP_USERNAME=glider.flights@mail.glider.flights
SMTP_PASSWORD=your-smtp-password-here
FROM_EMAIL=noreply@glider.flights
FROM_NAME="SOAR System (Production)"
```

## Testing Alerts

### Test Email Configuration

After deployment, you can test that Grafana can send emails:

1. **Open Grafana**: Navigate to http://grafana.glider.flights (or staging URL)
2. **Go to Alerting**: Click "Alerting" in the left sidebar
3. **Contact Points**: Click "Contact points"
4. **Find Contact**: Look for "soar-ops-email"
5. **Send Test**: Click "Edit" → "Test" → Send test notification

### Trigger Test Alert

To test the OGN message rate alert:

1. **Stop OGN Ingest Service**:
   ```bash
   sudo systemctl stop soar-ingest-ogn.service
   ```

2. **Wait 2-3 Minutes**: Alert should fire after the message rate drops for 2 minutes

3. **Check Email**: You should receive an alert email at the configured address

4. **Restart Service**:
   ```bash
   sudo systemctl start soar-ingest-ogn.service
   ```

5. **Check Email**: You should receive a "resolved" email once the service recovers

## Environment-Specific Alert Deployment

Alert rules are separated by environment to ensure that staging alerts are not deployed to production Grafana instances and vice versa:

- **Production**: Only `alert-rules-production.yml` is deployed when running `soar-deploy production`
- **Staging**: Only `alert-rules-staging.yml` is deployed when running `soar-deploy staging`

This separation is important because:
1. Staging and production use different Grafana instances
2. Production alerts query metrics with `environment="production"` labels
3. Staging alerts query metrics with `environment="staging"` labels
4. Deploying both sets of alerts to the same Grafana instance would cause confusion

To add alerts for both environments, you must update both `alert-rules-production.yml` and `alert-rules-staging.yml`.

## Managing Alerts

### Adding New Alert Rules

1. **Choose Environment**: Determine if the alert is for production, staging, or both
2. **Edit Alert Rules**:
   - For production: Modify `infrastructure/grafana-provisioning/alerting/alert-rules-production.yml`
   - For staging: Modify `infrastructure/grafana-provisioning/alerting/alert-rules-staging.yml`
   - For both: Update both files
3. **Add New Rule**: Add a new rule under the `rules:` section with appropriate `environment` label filter
4. **Test Locally**: Use Grafana UI to test the query
5. **Deploy**: Run `soar-deploy` to deploy the new alert
6. **Verify**: Check Grafana UI to ensure the alert appears

Example alert rule structure:

```yaml
- uid: unique_alert_id
  title: Alert Title Here
  condition: B
  data:
    - refId: A
      relativeTimeRange:
        from: 300
        to: 0
      datasourceUid: Prometheus
      model:
        expr: your_prometheus_query_here
        intervalMs: 1000
        maxDataPoints: 43200
        refId: A

    - refId: B
      datasourceUid: __expr__
      model:
        conditions:
          - evaluator:
              params: [threshold_value]
              type: gt  # or lt, eq, etc.
            operator:
              type: and
            query:
              params: [A]
            reducer:
              params: []
              type: last
            type: query
        expression: A
        refId: B
        type: threshold

  for: 2m  # Alert if condition persists for 2 minutes
  noDataState: Alerting
  execErrState: Alerting
  annotations:
    summary: Short summary of the alert
    description: 'Detailed description with template: {{ $values.A.Value }}'
  labels:
    severity: critical  # or warning
    service: service-name
    team: ops
```

### Modifying Notification Policies

Edit `infrastructure/grafana-provisioning/alerting/notification-policies.yml`:

```yaml
policies:
  - orgId: 1
    receiver: soar-ops-email
    group_by: ['alertname', 'service']
    group_wait: 30s        # Wait before sending first notification
    group_interval: 5m     # Wait before sending next batch
    repeat_interval: 1h    # Repeat interval for unresolved alerts
```

### Changing Alert Email Address

1. **Update Environment File**: Edit `/etc/soar/env` or `/etc/soar/env-staging`
   ```bash
   EMAIL_TO=new-email@example.com
   ```

2. **Redeploy**: Run `soar-deploy` to regenerate config files with new email

3. **Verify**: Check `/etc/grafana/provisioning/alerting/contact-points.yml` contains new email

### Changing SMTP Configuration

1. **Update Environment File**: Edit SMTP variables in `/etc/soar/env`
2. **Redeploy**: Run `soar-deploy` to regenerate SMTP config
3. **Verify**: Check `/etc/systemd/system/grafana-server.service.d/smtp.conf`
4. **Restart Grafana**: `sudo systemctl restart grafana-server`

## Troubleshooting

### Alerts Not Sending

1. **Check Grafana Service**: Ensure Grafana is running
   ```bash
   sudo systemctl status grafana-server
   ```

2. **Check SMTP Config**: View Grafana environment variables
   ```bash
   sudo systemctl cat grafana-server | grep -A 20 "service.d"
   ```

3. **Check Grafana Logs**: Look for SMTP errors
   ```bash
   sudo journalctl -u grafana-server -n 100 --no-pager
   ```

4. **Test Contact Point**: Use Grafana UI to send test email (see "Testing Alerts" above)

### Alert Not Triggering

1. **Check Query**: Verify the metric exists in Prometheus
   ```bash
   curl 'http://localhost:9090/api/v1/query?query=aprs_raw_message_processed_total'
   ```

2. **Check Alert State**: View alert in Grafana UI under "Alerting" → "Alert rules"

3. **Check Evaluation**: Look for evaluation errors in Grafana logs

4. **Adjust Threshold**: If metric exists but alert doesn't fire, adjust the threshold

### Emails Going to Spam

1. **Check SPF/DKIM**: Ensure your domain has proper SPF and DKIM records
2. **Check FROM_EMAIL**: Use a verified sender address
3. **Test with Different Email**: Try sending to a different email provider
4. **Check Mailgun Logs**: Review Mailgun dashboard for delivery issues

## Monitoring Alert Health

### Check Alert Rule Status

```bash
# View alert rules in Grafana
# Navigate to: Alerting → Alert rules

# Check for evaluation errors
sudo journalctl -u grafana-server | grep -i "alert\|error"
```

### Check SMTP Configuration

```bash
# View SMTP environment variables
sudo systemctl show grafana-server | grep GF_SMTP

# Test SMTP connection manually (if needed)
# Use telnet or openssl s_client to test SMTP server
```

## Metrics Used for Alerting

| Metric | Description | Alert Usage |
|--------|-------------|-------------|
| `aprs_raw_message_processed_total` | Counter of processed APRS messages | Message rate monitoring |
| `aprs_nats_published_total` | Counter of messages published to NATS | NATS publishing health |
| `aprs_connection_connected` | Gauge (0 or 1) indicating connection status | Connection monitoring |
| `aprs_nats_publish_error_total` | Counter of NATS publish errors | Error rate monitoring |

## Security Notes

- **No Credentials in Git**: Template files use placeholders, actual credentials extracted at deploy time
- **File Permissions**: Generated config files have restricted permissions (640 for sensitive files)
- **SMTP Password**: Stored in environment file and systemd drop-in, not in Grafana database
- **Access Control**: Only root and grafana user can read SMTP configuration files

## Further Reading

- [Grafana Alerting Documentation](https://grafana.com/docs/grafana/latest/alerting/)
- [Prometheus Query Language](https://prometheus.io/docs/prometheus/latest/querying/basics/)
- [SOAR Metrics Documentation](grafana-README.md)
- [SOAR Deployment Guide](DEPLOYMENT.md)
- [Grafana Datasource Troubleshooting](GRAFANA-DATASOURCE-TROUBLESHOOTING.md)
