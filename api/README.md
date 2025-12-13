# Sagittarius Server 🎯

API server to receive and store keyboard/mouse statistics.

## Quick Start with Docker

```bash
cd sagittarius-server

# Configure
cp .env.example .env
nano .env  # Set API_SECRET

# Run
docker-compose up -d

# View logs
docker-compose logs -f
```

Server runs on `http://localhost:3000`

## Configuration (.env)

```bash
API_SECRET=your_secret_key_here
DATABASE_URL=sqlite:///data/sagittarius.db
HOST=0.0.0.0
PORT=3000
```

## API Endpoints

### POST /api/stats
Receive stats from client (requires `X-API-Secret` header)

### GET /api/stats
Get all statistics (requires `X-API-Secret` header)

**Response:**
```json
{
  "total_keys": 15420,
  "total_clicks": 3482,
  "total_wheels": 1273,
  "events": [
    {"name": "KEY_SPACE", "type": "KEY", "count": 1245},
    {"name": "KEY_E", "type": "KEY", "count": 892}
  ]
}
```

### GET /health
Health check (no auth required)

## Database

SQLite database stored in `./data/sagittarius.db`

**Schema:**
```sql
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_name TEXT NOT NULL UNIQUE,
    event_type TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

## Useful SQL Queries

```sql
-- Global stats
SELECT 
    SUM(CASE WHEN event_type = 'KEY' THEN count ELSE 0 END) as keys,
    SUM(CASE WHEN event_type = 'CLICK' THEN count ELSE 0 END) as clicks,
    SUM(CASE WHEN event_type = 'WHEEL' THEN count ELSE 0 END) as wheels
FROM events;

-- Top 10 keys
SELECT event_name, count FROM events 
WHERE event_type = 'KEY' 
ORDER BY count DESC LIMIT 10;

-- Export to CSV
sqlite3 data/sagittarius.db -header -csv \
  "SELECT * FROM events ORDER BY count DESC;" > events.csv
```

## Run without Docker

```bash
cargo build --release
DATABASE_URL=sqlite://sagittarius.db \
API_SECRET=secret \
./target/release/sagittarius-server
```

## Backup

```bash
# Manual backup
cp data/sagittarius.db backup/sagittarius-$(date +%Y%m%d).db

# Automated (crontab)
0 2 * * * cp /path/to/data/sagittarius.db /backup/sagittarius-$(date +\%Y\%m\%d).db
```