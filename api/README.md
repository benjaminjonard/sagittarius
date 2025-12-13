# Sagittarius Server 🎯

Serveur API Rust pour recevoir et stocker les statistiques d'utilisation du clavier et de la souris.

## Stack

- **Rust** - Actix-web
- **SQLite** - Base de données légère et performante
- **Docker** - Déploiement simplifié

## Structure du projet

```
sagittarius-server/
├── src/
│   └── main.rs          # Code principal du serveur
├── data/                # Données SQLite (créé automatiquement)
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
├── .env                 # Configuration (ne pas commit)
├── .env.example         # Exemple de configuration
└── README.md
```

## Installation locale

### 1. Prérequis

- Rust 1.75+
- SQLite3

### 2. Configuration

Copie `.env.example` vers `.env` et modifie les valeurs :

```bash
cp .env.example .env
nano .env
```

```bash
API_SECRET=ta_cle_secrete_unique
DATABASE_URL=sqlite://sagittarius.db
HOST=0.0.0.0
PORT=3000
RUST_LOG=info
```

### 3. Lance le serveur

```bash
cargo run --release
```

Le serveur démarre sur `http://0.0.0.0:3000`

## Installation avec Docker

### 1. Build l'image

```bash
docker build -t sagittarius-server .
```

### 2. Lance avec docker-compose

Crée d'abord un fichier `.env` :

```bash
cp .env.example .env
nano .env
```

Puis lance :

```bash
docker-compose up -d
```

### 3. Vérifie les logs

```bash
docker-compose logs -f
```

### 4. Arrête le serveur

```bash
docker-compose down
```

## Endpoints

### POST /api/stats

Reçoit les statistiques du client.

**Headers requis :**
- `Content-Type: application/json`
- `X-API-Secret: ta_cle_secrete`

**Body :**
```json
{
  "total_keys": 42,
  "total_clicks": 18,
  "total_wheels": 12,
  "events": {
    "KEY_A": 5,
    "KEY_SPACE": 12,
    "CLICK_LEFT": 15
  },
  "timestamp": "2024-01-15T10:30:45+01:00",
  "hostname": "mon-ordinateur"
}
```

**Réponse (200) :**
```json
{
  "success": true,
  "message": "Stats saved successfully"
}
```

**Réponse (401) :**
```json
{
  "error": "Unauthorized - Invalid or missing API secret"
}
```

### GET /api/stats

Récupère les 100 dernières entrées (nécessite authentification).

**Headers requis :**
- `X-API-Secret: ta_cle_secrete`

**Réponse (200) :**
```json
[
  {
    "id": 1,
    "hostname": "mon-ordinateur",
    "timestamp": "2024-01-15T10:30:45+01:00",
    "total_keys": 42,
    "total_clicks": 18,
    "total_wheels": 12,
    "events": { "KEY_A": 5, ... },
    "created_at": "2024-01-15T10:30:50"
  }
]
```

### GET /health

Endpoint de santé (sans authentification).

**Réponse (200) :**
```json
{
  "status": "ok",
  "service": "sagittarius-server"
}
```

## Schéma de la base de données

```sql
CREATE TABLE stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hostname TEXT,
    timestamp TEXT NOT NULL,
    total_keys INTEGER NOT NULL,
    total_clicks INTEGER NOT NULL,
    total_wheels INTEGER NOT NULL,
    events TEXT NOT NULL,           -- JSON
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_stats_hostname ON stats(hostname);
CREATE INDEX idx_stats_timestamp ON stats(timestamp);
CREATE INDEX idx_stats_created_at ON stats(created_at);
```

## Configuration du client

Dans le client Sagittarius, configure :

```bash
API_URL=http://localhost:3000/api/stats
API_SECRET=ta_cle_secrete_unique
```

Ou si le serveur est distant :

```bash
API_URL=https://ton-serveur.com/api/stats
API_SECRET=ta_cle_secrete_unique
```

## Déploiement en production

### Option 1 : Docker sur VPS

1. Clone le repo sur ton serveur
2. Configure `.env` avec ta vraie clé secrète
3. Lance `docker-compose up -d`
4. Configure un reverse proxy (nginx/caddy) avec HTTPS

### Option 2 : Systemd

Similaire au client, crée un service systemd :

```bash
sudo cp target/release/sagittarius-server /usr/local/bin/
sudo nano /etc/systemd/system/sagittarius-server.service
```

```ini
[Unit]
Description=Sagittarius API Server
After=network.target

[Service]
Type=simple
User=sagittarius
WorkingDirectory=/opt/sagittarius-server
EnvironmentFile=/etc/sagittarius-server/.env
ExecStart=/usr/local/bin/sagittarius-server
Restart=always

[Install]
WantedBy=multi-user.target
```

## Sauvegarde de la base de données

La base SQLite est dans `./data/sagittarius.db` (ou `/data/sagittarius.db` dans Docker).

### Sauvegarde manuelle

```bash
# Local
cp sagittarius.db sagittarius.db.backup

# Docker
docker cp sagittarius-server:/data/sagittarius.db ./backup/
```

### Sauvegarde automatique (cron)

```bash
# Ajoute dans crontab
0 2 * * * cp /chemin/vers/sagittarius.db /backup/sagittarius-$(date +\%Y\%m\%d).db
```

## Requêtes SQL utiles

```sql
-- Stats totales par hostname
SELECT 
    hostname,
    SUM(total_keys) as total_keys,
    SUM(total_clicks) as total_clicks,
    SUM(total_wheels) as total_wheels,
    COUNT(*) as entries
FROM stats
GROUP BY hostname;

-- Stats des dernières 24h
SELECT * FROM stats
WHERE created_at > datetime('now', '-1 day')
ORDER BY created_at DESC;

-- Touches les plus utilisées (nécessite JSON parsing)
SELECT 
    json_extract(events, '$.KEY_A') as key_a_count,
    json_extract(events, '$.KEY_SPACE') as space_count
FROM stats
WHERE json_extract(events, '$.KEY_A') IS NOT NULL;
```

## Sécurité

⚠️ **Important** :

- Utilise TOUJOURS HTTPS en production
- Change l'API_SECRET par défaut
- N'expose PAS le port directement, utilise un reverse proxy
- Configure un firewall (ufw/iptables)
- Limite le taux de requêtes si nécessaire

## Monitoring

```bash
# Logs en temps réel
docker-compose logs -f

# Taille de la base
ls -lh data/sagittarius.db

# Nombre d'entrées
sqlite3 data/sagittarius.db "SELECT COUNT(*) FROM stats;"
```

## Développement

```bash
# Lance en mode dev avec auto-reload
cargo watch -x run

# Tests
cargo test

# Format du code
cargo fmt

# Lint
cargo clippy
```

## Licence

À définir