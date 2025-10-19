#!/bin/bash
# Photon Geocoding Server Installation Script
# This script automates the installation of Photon on Ubuntu/Debian systems

set -e  # Exit on error

# Configuration
PHOTON_VERSION="${PHOTON_VERSION:-0.5.0}"
PHOTON_JAR="photon-${PHOTON_VERSION}.jar"
PHOTON_URL="https://github.com/komoot/photon/releases/download/${PHOTON_VERSION}/${PHOTON_JAR}"
JAVA_HEAP="${JAVA_HEAP:-16G}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Print functions
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    print_error "Please run this script as root or with sudo"
    exit 1
fi

print_info "Starting Photon installation..."

# Step 1: Install Java
print_info "Installing Java..."
apt update
apt install -y openjdk-17-jre-headless wget

# Verify Java installation
if ! command -v java &> /dev/null; then
    print_error "Java installation failed"
    exit 1
fi
print_info "Java installed: $(java -version 2>&1 | head -n 1)"

# Step 2: Create photon user
print_info "Creating photon system user..."
if ! id -u photon &> /dev/null; then
    useradd --system --home-dir /opt/photon --shell /bin/false photon
    print_info "User 'photon' created"
else
    print_warn "User 'photon' already exists"
fi

# Step 3: Create directory structure
print_info "Creating directory structure..."
mkdir -p /opt/photon
mkdir -p /var/lib/photon
mkdir -p /var/log/photon

# Set ownership
chown -R photon:photon /opt/photon
chown -R photon:photon /var/lib/photon
chown -R photon:photon /var/log/photon

print_info "Directories created and permissions set"

# Step 4: Download Photon JAR
print_info "Downloading Photon ${PHOTON_VERSION}..."
cd /opt/photon

if [ -f "${PHOTON_JAR}" ]; then
    print_warn "Photon JAR already exists, skipping download"
else
    sudo -u photon wget "${PHOTON_URL}"
    if [ $? -eq 0 ]; then
        print_info "Photon JAR downloaded successfully"
    else
        print_error "Failed to download Photon JAR"
        exit 1
    fi
fi

# Create symlink
sudo -u photon ln -sf "${PHOTON_JAR}" photon.jar
print_info "Created symlink photon.jar -> ${PHOTON_JAR}"

# Step 5: Install systemd service
print_info "Installing systemd service..."

cat > /etc/systemd/system/photon.service << EOF
[Unit]
Description=Photon Geocoding Server
Documentation=https://github.com/komoot/photon
After=network.target

[Service]
Type=simple
User=photon
Group=photon

WorkingDirectory=/opt/photon

Environment="JAVA_OPTS=-Xmx${JAVA_HEAP} -Xms${JAVA_HEAP}"

ExecStart=/usr/bin/java \${JAVA_OPTS} -jar /opt/photon/photon.jar \\
    -data-dir /var/lib/photon \\
    -listen-ip 127.0.0.1 \\
    -listen-port 2322 \\
    -cors-any

StandardOutput=append:/var/log/photon/photon.log
StandardError=append:/var/log/photon/photon-error.log

Restart=on-failure
RestartSec=10s

LimitNOFILE=65535

NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/photon /var/log/photon

[Install]
WantedBy=multi-user.target
EOF

print_info "Systemd service file created"

# Step 6: Setup log rotation
print_info "Setting up log rotation..."
cat > /etc/logrotate.d/photon << EOF
/var/log/photon/*.log {
    daily
    rotate 14
    compress
    delaycompress
    missingok
    notifempty
    create 0640 photon photon
    sharedscripts
    postrotate
        systemctl reload photon > /dev/null 2>&1 || true
    endscript
}
EOF

print_info "Log rotation configured"

# Step 7: Reload systemd
print_info "Reloading systemd daemon..."
systemctl daemon-reload

# Enable service
systemctl enable photon
print_info "Photon service enabled"

print_info ""
print_info "=========================================="
print_info "Photon installation completed successfully!"
print_info "=========================================="
print_info ""
print_info "Next steps:"
print_info "1. Download and install Photon data:"
print_info "   cd /var/lib/photon"
print_info "   sudo -u photon wget https://download1.graphhopper.com/public/photon-db-latest.tar.bz2"
print_info "   sudo -u photon tar xjf photon-db-latest.tar.bz2"
print_info "   sudo -u photon rm photon-db-latest.tar.bz2"
print_info ""
print_info "2. Start the service:"
print_info "   sudo systemctl start photon"
print_info ""
print_info "3. Check status:"
print_info "   sudo systemctl status photon"
print_info ""
print_info "4. Test the service:"
print_info "   curl 'http://localhost:2322/api?q=Berlin'"
print_info ""
print_info "For more information, see: docs/photon-deployment.md"
