# Sagittarius 🏹

Monitor keyboard and mouse usage on Linux and send stats to an API.

## Quick Start

### Prerequisites

```bash
sudo apt install libinput-dev  # Debian/Ubuntu
```

### Installation

1. **Clone and configure**

```bash
git clone <repo>
cd sagittarius
cp .env.example .env
nano .env  # Set API_URL and API_SECRET
```

2. **Build and run**

```bash
cargo build --release
sudo -E cargo run --release
```

## Run as systemd service (auto-start on boot)

```bash
# Install binary
sudo cp target/release/sagittarius /usr/local/bin/
sudo chmod +x /usr/local/bin/sagittarius

# Configure
sudo mkdir -p /etc/sagittarius
sudo cp .env /etc/sagittarius/.env
sudo chmod 600 /etc/sagittarius/.env

# Create service
sudo tee /etc/systemd/system/sagittarius.service > /dev/null <<EOF
[Unit]
Description=Sagittarius Input Monitor
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=/var/lib/sagittarius
EnvironmentFile=/etc/sagittarius/.env
ExecStart=/usr/local/bin/sagittarius
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Create work directory
sudo mkdir -p /var/lib/sagittarius

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable sagittarius
sudo systemctl start sagittarius

# Check status
sudo systemctl status sagittarius

# View logs
sudo journalctl -u sagittarius -f
```

## Configuration (.env)

```bash
API_URL=http://localhost:3000/api/stats
API_SECRET=your_secret_key_here
INTERVAL_SECS=10
```

## Data sent to API

```json
{
  "total_keys": 42,
  "total_clicks": 18,
  "total_wheels": 12,
  "events": {
    "KEY_A": 5,
    "KEY_SPACE": 12,
    "CLICK_LEFT": 15
  }
}
```

## Server

See [sagittarius-server](./sagittarius-server/README.md) for the API server.