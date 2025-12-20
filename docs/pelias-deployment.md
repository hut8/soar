# Pelias Geocoding Server Deployment Guide

This guide covers deploying [Pelias](https://github.com/pelias/pelias), an open-source geocoding platform, specifically configured for **reverse geocoding only** using city-level data from Who's on First.

## Overview

Pelias is a modular geocoding platform with multiple services. For SOAR, we're using:
- **PIP Service** (Point in Polygon) for reverse geocoding
- **Who's on First** dataset for city/locality/country-level boundaries
- **OpenSearch** for storage and indexing
- No street-level data (no OpenStreetMap, no OpenAddresses)

## Architecture

```
┌─────────────────┐
│  SOAR Backend   │
│  (Rust)         │
└────────┬────────┘
         │ HTTP
         ▼
┌─────────────────┐
│  Pelias API     │
│  (Node.js)      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐        ┌──────────────────┐
│  Pelias PIP     │◄──────►│  OpenSearch      │
│  (Point in      │        │  (Bare Metal)    │
│   Polygon)      │        │                  │
└─────────────────┘        └──────────────────┘
         │
         ▼
   Who's on First
   Dataset (WOF)
```

## System Requirements

### Hardware Requirements
- **RAM**: 16GB minimum, **32GB recommended** for production
- **Disk**: ~20GB for Who's on First data + OpenSearch index
- **CPU**: 4 cores minimum, 8+ cores recommended
- **OS**: Linux (Ubuntu 22.04 LTS recommended)

### Software Requirements
- **OpenSearch**: 2.11.0 or newer (bare metal installation)
- **Docker**: 24.0+ (for data import only)
- **Node.js**: 18.x or newer (for Pelias services)
- **Git**: For cloning Pelias repositories

## Installation Steps

### 1. Install Elasticsearch or OpenSearch (Bare Metal)

Pelias works with either **Elasticsearch** or **OpenSearch**. Both are essentially the same technology - OpenSearch is an open-source fork of Elasticsearch 7.10.2. Choose based on your preference:

- **Elasticsearch**: Latest features, widely used, proprietary license
- **OpenSearch**: Fully open source (Apache 2.0), AWS-backed

#### Option A: Install Elasticsearch (Recommended for simplicity)

```bash
# Install prerequisites
sudo apt update
sudo apt install apt-transport-https gpg

# Add Elasticsearch GPG key and repository
wget -qO - https://artifacts.elastic.co/GPG-KEY-elasticsearch | sudo gpg --dearmor -o /usr/share/keyrings/elasticsearch-keyring.gpg

echo "deb [signed-by=/usr/share/keyrings/elasticsearch-keyring.gpg] https://artifacts.elastic.co/packages/8.x/apt stable main" | sudo tee /etc/apt/sources.list.d/elastic-8.x.list

# Install Elasticsearch
sudo apt update
sudo apt install elasticsearch

# Install required ICU analysis plugin
sudo /usr/share/elasticsearch/bin/elasticsearch-plugin install analysis-icu

# Configure for Pelias
sudo tee -a /etc/elasticsearch/elasticsearch.yml > /dev/null <<EOF

# Pelias configuration
discovery.type: single-node
indices.query.bool.max_clause_count: 4096

# Disable security for local development (WARNING: Enable for production!)
xpack.security.enabled: false
xpack.security.enrollment.enabled: false
xpack.security.http.ssl.enabled: false
xpack.security.transport.ssl.enabled: false
EOF

# Set JVM heap size (50% of RAM, max 32GB)
# Edit /etc/elasticsearch/jvm.options.d/heap.options
echo "-Xms8g" | sudo tee /etc/elasticsearch/jvm.options.d/heap.options
echo "-Xmx8g" | sudo tee -a /etc/elasticsearch/jvm.options.d/heap.options

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable elasticsearch
sudo systemctl start elasticsearch

# Verify it's running
curl http://localhost:9200
```

#### Option B: Install OpenSearch

```bash
# Install prerequisites
sudo apt update
sudo apt install apt-transport-https gpg

# Add OpenSearch GPG key and repository
curl -o- https://artifacts.opensearch.org/publickeys/opensearch.pgp | sudo gpg --dearmor --batch --yes -o /usr/share/keyrings/opensearch-keyring

echo "deb [signed-by=/usr/share/keyrings/opensearch-keyring] https://artifacts.opensearch.org/releases/bundle/opensearch/2.x/apt stable main" | sudo tee /etc/apt/sources.list.d/opensearch-2.x.list

# Install OpenSearch
sudo apt update
sudo apt install opensearch

# Install required ICU analysis plugin
sudo /usr/share/opensearch/bin/opensearch-plugin install analysis-icu

# Configure for Pelias
sudo tee -a /etc/opensearch/opensearch.yml > /dev/null <<EOF

# Pelias configuration
discovery.type: single-node
indices.query.bool.max_clause_count: 4096

# Disable security for local development (WARNING: Enable for production!)
plugins.security.disabled: true
EOF

# Set JVM heap size (50% of RAM, max 32GB)
# Edit /etc/opensearch/jvm.options
echo "-Xms8g" | sudo tee /etc/opensearch/jvm.options.d/heap.options
echo "-Xmx8g" | sudo tee -a /etc/opensearch/jvm.options.d/heap.options

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable opensearch
sudo systemctl start opensearch

# Verify it's running
curl http://localhost:9200

# Verify OpenSearch is running
curl http://localhost:9200/
```

### 2. Install Docker (for Data Import)

Docker is only used for importing Who's on First data. The Pelias services themselves will run outside Docker.

```bash
# Install Docker
sudo apt update
sudo apt install docker.io docker-compose
sudo systemctl enable docker
sudo systemctl start docker

# Add your user to docker group (optional)
sudo usermod -aG docker $USER
# Log out and back in for this to take effect
```

### 3. Set Up Pelias Project Directory

```bash
# Create pelias directory
sudo mkdir -p /opt/pelias
sudo chown $USER:$USER /opt/pelias
cd /opt/pelias

# Clone Pelias docker repository (for configuration and import scripts)
git clone https://github.com/pelias/docker.git
cd docker
```

### 4. Configure Pelias

Create `/opt/pelias/docker/pelias.json`:

```json
{
  "logger": {
    "level": "info",
    "timestamp": true
  },
  "esclient": {
    "hosts": [
      {
        "host": "host.docker.internal",
        "port": 9200
      }
    ]
  },
  "elasticsearch": {
    "settings": {
      "index": {
        "refresh_interval": "30s",
        "number_of_replicas": "0",
        "number_of_shards": "3"
      }
    }
  },
  "acceptance-tests": {
    "endpoints": {
      "local": "http://api:4000/v1/"
    }
  },
  "api": {
    "version": "1.0",
    "indexName": "pelias",
    "host": "0.0.0.0",
    "port": 4000,
    "textAnalyzer": "libpostal",
    "services": {
      "pip": {
        "url": "http://pip:4200"
      }
    },
    "attributionURL": "https://github.com/pelias/pelias"
  },
  "imports": {
    "adminLookup": {
      "enabled": true,
      "maxConcurrentReqs": 4
    },
    "whosonfirst": {
      "datapath": "/data/whosonfirst",
      "importVenues": false,
      "importPostalcodes": false,
      "importPlace": [
        "locality",
        "county",
        "macrocounty",
        "region",
        "macroregion",
        "country",
        "dependency",
        "disputed"
      ]
    }
  }
}
```

**Key configuration notes:**
- `esclient.hosts`: Points to bare metal OpenSearch via `host.docker.internal`
- `imports.whosonfirst.importPlace`: Only city/locality/region/country level data
- `importVenues`: false (no POI data)
- `importPostalcodes`: false (no postal codes)
- No OpenStreetMap or OpenAddresses importers configured

### 5. Download Who's on First Data

```bash
cd /opt/pelias/docker

# Create data directory
mkdir -p data/whosonfirst

# Download Who's on First data
# This downloads administrative boundaries (cities, counties, regions, countries)
# Excludes venues and street-level data
docker-compose run --rm whosonfirst npm run download
```

**Data size:** Approximately 10-15GB for worldwide Who's on First data.

### 6. Import Data into OpenSearch

```bash
cd /opt/pelias/docker

# Create OpenSearch index
docker-compose run --rm schema npm run create_index

# Import Who's on First data
# This will take 30-60 minutes depending on your hardware
docker-compose run --rm whosonfirst npm start

# Verify import
curl -s "http://localhost:9200/pelias/_stats" | jq '.indices.pelias.total.docs.count'
# Should show thousands of documents (cities, regions, countries)
```

### 7. Set Up Pelias Services (Bare Metal)

After importing data, we run the Pelias API and PIP services on bare metal.

#### Install Node.js 18

```bash
# Install Node.js 18.x
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt install -y nodejs

# Verify installation
node --version  # Should be v18.x
npm --version
```

#### Install Pelias PIP Service

```bash
# Create pelias services directory
sudo mkdir -p /opt/pelias-services
sudo chown $USER:$USER /opt/pelias-services
cd /opt/pelias-services

# Clone PIP service
git clone https://github.com/pelias/pip-service.git
cd pip-service

# Install dependencies
npm install

# Create systemd service
sudo tee /etc/systemd/system/pelias-pip.service > /dev/null <<'EOF'
[Unit]
Description=Pelias PIP Service (Point in Polygon)
Documentation=https://github.com/pelias/pip-service
After=network.target opensearch.service
Requires=opensearch.service

[Service]
Type=simple
User=pelias
Group=pelias
WorkingDirectory=/opt/pelias-services/pip-service
Environment="PORT=4200"
Environment="PELIAS_CONFIG=/opt/pelias/docker/pelias.json"
Environment="ELASTICSEARCH_HOST=http://localhost:9200"
ExecStart=/usr/bin/node index.js
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Resource limits
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
EOF
```

#### Install Pelias API Service

```bash
cd /opt/pelias-services

# Clone API service
git clone https://github.com/pelias/api.git
cd api

# Install dependencies
npm install

# Create systemd service
sudo tee /etc/systemd/system/pelias-api.service > /dev/null <<'EOF'
[Unit]
Description=Pelias API Service
Documentation=https://github.com/pelias/api
After=network.target opensearch.service pelias-pip.service
Requires=opensearch.service pelias-pip.service

[Service]
Type=simple
User=pelias
Group=pelias
WorkingDirectory=/opt/pelias-services/api
Environment="PORT=4000"
Environment="PELIAS_CONFIG=/opt/pelias/docker/pelias.json"
Environment="ELASTICSEARCH_HOST=http://localhost:9200"
Environment="PIP_SERVICE=http://localhost:4200"
ExecStart=/usr/bin/node index.js
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Resource limits
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
EOF
```

#### Create pelias User

```bash
sudo useradd --system --home-dir /opt/pelias-services --shell /bin/false pelias
sudo chown -R pelias:pelias /opt/pelias-services
```

#### Start Pelias Services

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable and start PIP service
sudo systemctl enable pelias-pip
sudo systemctl start pelias-pip
sudo systemctl status pelias-pip

# Enable and start API service
sudo systemctl enable pelias-api
sudo systemctl start pelias-api
sudo systemctl status pelias-api
```

### 8. Configure Reverse Proxy (Optional)

If you want to expose Pelias on a public domain:

```nginx
# /etc/nginx/sites-available/pelias.soar.example.com
server {
    listen 80;
    server_name pelias.soar.example.com;

    location / {
        proxy_pass http://localhost:4000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Enable and reload:

```bash
sudo ln -s /etc/nginx/sites-available/pelias.soar.example.com /etc/nginx/sites-enabled/
sudo systemctl reload nginx
```

## Testing the Installation

### Test PIP Service (Reverse Geocoding)

```bash
# Reverse geocode coordinates (San Francisco)
curl "http://localhost:4000/v1/reverse?point.lat=37.7749&point.lon=-122.4194" | jq

# Expected response: City (San Francisco), State (California), Country (United States)
```

### Test with Various Locations

```bash
# New York City
curl "http://localhost:4000/v1/reverse?point.lat=40.7128&point.lon=-74.0060" | jq

# London, UK
curl "http://localhost:4000/v1/reverse?point.lat=51.5074&point.lon=-0.1278" | jq

# Tokyo, Japan
curl "http://localhost:4000/v1/reverse?point.lat=35.6762&point.lon=139.6503" | jq
```

### Verify Data Levels

The responses should contain:
- ✅ **Locality** (city/town)
- ✅ **Region** (state/province)
- ✅ **Country**
- ❌ **Street addresses** (not included - expected)
- ❌ **POIs/Venues** (not included - expected)

## Environment Variables for SOAR

Add to your `.env` or environment configuration:

```bash
# Pelias API endpoint
PELIAS_BASE_URL=http://localhost:4000
```

If running on a separate server:

```bash
PELIAS_BASE_URL=http://pelias.internal.example.com:4000
```

## Maintenance

### Update Who's on First Data

```bash
cd /opt/pelias/docker

# Stop services
sudo systemctl stop pelias-api pelias-pip

# Download latest data
docker-compose run --rm whosonfirst npm run download

# Re-import
docker-compose run --rm whosonfirst npm start

# Restart services
sudo systemctl start pelias-pip pelias-api
```

### Monitor OpenSearch Health

```bash
# Check cluster health
curl http://localhost:9200/_cluster/health?pretty

# Check index stats
curl http://localhost:9200/pelias/_stats?pretty

# Check disk usage
du -sh /var/lib/opensearch
```

### Check Service Logs

```bash
# OpenSearch logs
sudo journalctl -u opensearch -f

# PIP service logs
sudo journalctl -u pelias-pip -f

# API service logs
sudo journalctl -u pelias-api -f
```

### Backup OpenSearch Data

```bash
# Create snapshot repository
curl -X PUT "localhost:9200/_snapshot/pelias_backup" -H 'Content-Type: application/json' -d'
{
  "type": "fs",
  "settings": {
    "location": "/var/lib/opensearch/snapshots"
  }
}'

# Create snapshot
curl -X PUT "localhost:9200/_snapshot/pelias_backup/snapshot_1?wait_for_completion=true"

# Restore snapshot
curl -X POST "localhost:9200/_snapshot/pelias_backup/snapshot_1/_restore"
```

## Performance Tuning

### OpenSearch Tuning

```bash
# Increase file descriptors
echo "opensearch soft nofile 65535" | sudo tee -a /etc/security/limits.conf
echo "opensearch hard nofile 65535" | sudo tee -a /etc/security/limits.conf

# Disable swapping for better performance
sudo sysctl -w vm.swappiness=1
echo "vm.swappiness=1" | sudo tee -a /etc/sysctl.conf
```

### Pelias API Tuning

For high-traffic scenarios, run multiple API instances behind a load balancer:

```bash
# Run on ports 4000, 4001, 4002
sudo systemctl enable pelias-api@{4000,4001,4002}
sudo systemctl start pelias-api@{4000,4001,4002}
```

## Troubleshooting

### OpenSearch Won't Start

```bash
# Check logs
sudo journalctl -u opensearch -n 100

# Common issues:
# 1. Insufficient memory - increase heap size in jvm.options
# 2. Permissions - check ownership of /var/lib/opensearch
# 3. Port conflict - ensure port 9200 is available
```

### PIP Service Returns Empty Results

```bash
# Verify data was imported
curl "http://localhost:9200/pelias/_count"

# Check PIP service logs
sudo journalctl -u pelias-pip -n 100

# Verify OpenSearch is reachable from PIP
curl http://localhost:9200/_cluster/health
```

### Slow Reverse Geocoding

```bash
# Check OpenSearch query performance
curl "http://localhost:9200/pelias/_search?pretty" -d '{"query": {"match_all": {}}}'

# Increase OpenSearch cache size in opensearch.yml:
# indices.queries.cache.size: 20%
# indices.fielddata.cache.size: 40%
```

## Security Considerations

### Production Recommendations

1. **Enable OpenSearch Security Plugin**
   ```bash
   # Re-enable in opensearch.yml
   plugins.security.disabled: false
   ```

2. **Use TLS/SSL for OpenSearch**
   - Generate certificates
   - Configure in `opensearch.yml`
   - Update Pelias config to use HTTPS

3. **Firewall Rules**
   ```bash
   # Only allow local connections to OpenSearch
   sudo ufw allow from 127.0.0.1 to any port 9200

   # Allow Pelias API from specific IPs only
   sudo ufw allow from <SOAR_SERVER_IP> to any port 4000
   ```

4. **Rate Limiting**
   - Configure nginx rate limiting
   - Monitor API usage
   - Set up alerts for unusual traffic

## Resource Monitoring

### Set Up Prometheus Metrics (Optional)

```bash
# Install OpenSearch Prometheus exporter
# Monitor OpenSearch health, query latency, index size

# Monitor Pelias API response times
# Track reverse geocoding request rates
```

## References

- [Pelias Documentation](https://github.com/pelias/pelias)
- [OpenSearch Documentation](https://opensearch.org/docs/latest/)
- [Who's on First Data](https://whosonfirst.org/)
- [PIP Service](https://github.com/pelias/pip-service)
- [Pelias API](https://github.com/pelias/api)

## Support

For issues specific to Pelias configuration:
- [Pelias GitHub Issues](https://github.com/pelias/pelias/issues)
- [Pelias Gitter Chat](https://gitter.im/pelias/pelias)

For SOAR-specific integration questions:
- Open an issue in the SOAR repository
