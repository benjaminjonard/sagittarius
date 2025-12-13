# Sagittarius 🏹

Un moniteur d'entrées clavier et souris pour Linux qui envoie les statistiques d'utilisation à une API via HTTP.

## Fonctionnalités

- 📊 Capture tous les événements clavier et souris en temps réel
- 🔄 Envoi périodique des statistiques à une API
- 💾 Sauvegarde automatique en cas d'échec d'envoi
- 🔐 Authentification par clé secrète
- 🛡️ Persistance des données en cas de crash

## Prérequis

- Rust 1.70+ (`cargo --version`)
- libinput (`sudo apt install libinput-dev`)
- Droits root (pour accéder aux périphériques d'entrée)

## Installation

### 1. Clone le projet

```bash
git clone <votre-repo>
cd sagittarius
```

### 2. Configure les variables d'environnement

Crée un fichier `.env` à la racine du projet :

```bash
API_URL=http://localhost:3000/api/stats
API_SECRET=change_moi_par_une_cle_secrete
```

**Important** : Ne commite JAMAIS le fichier `.env` ! Il est déjà dans `.gitignore`.

### 3. Compile le projet

```bash
cargo build --release
```

### 4. Lance le programme

```bash
sudo -E cargo run --release
# Ou directement le binaire
sudo -E ./target/release/sagittarius
```

Le flag `-E` préserve les variables d'environnement.

## Format des données envoyées

```json
{
  "total_keys": 42,
  "total_clicks": 18,
  "total_wheels": 12,
  "events": {
    "KEY_A": 5,
    "KEY_SPACE": 12,
    "KEY_ENTER": 3,
    "CLICK_LEFT": 15,
    "CLICK_RIGHT": 3,
    "WHEEL_VERTICAL": 12
  },
  "timestamp": "2024-01-15T10:30:45+01:00",
  "hostname": "mon-ordinateur"
}
```

Le JSON est envoyé en POST avec le header `X-API-Secret` contenant la clé secrète.

## Configuration de l'API (côté serveur)

Exemple avec Node.js/Express :

```javascript
app.post('/api/stats', (req, res) => {
  // Vérifie la clé secrète
  if (req.headers['x-api-secret'] !== process.env.API_SECRET) {
    return res.status(401).json({ error: 'Unauthorized' });
  }
  
  // Traite les données
  console.log('Stats reçues:', req.body);
  
  res.json({ success: true });
});
```

## Installation en tant que service système

Pour lancer Sagittarius automatiquement au démarrage :

### 1. Installe le binaire

```bash
cargo build --release
sudo cp target/release/sagittarius /usr/local/bin/
sudo chmod +x /usr/local/bin/sagittarius
```

### 2. Crée le fichier de configuration

```bash
sudo mkdir -p /etc/sagittarius
sudo nano /etc/sagittarius/.env
```

Ajoute tes variables d'environnement :

```bash
API_URL=https://ton-api.com/api/stats
API_SECRET=ta_cle_secrete_ici
```

Sécurise le fichier :

```bash
sudo chmod 600 /etc/sagittarius/.env
```

### 3. Crée le service systemd

```bash
sudo nano /etc/systemd/system/sagittarius.service
```

Contenu :

```ini
[Unit]
Description=Sagittarius Input Monitor
After=network.target multi-user.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=/var/lib/sagittarius
EnvironmentFile=/etc/sagittarius/.env
ExecStart=/usr/local/bin/sagittarius
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Sécurité
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

### 4. Crée le répertoire de travail

```bash
sudo mkdir -p /var/lib/sagittarius
sudo chown root:root /var/lib/sagittarius
```

### 5. Active et démarre le service

```bash
# Recharge systemd
sudo systemctl daemon-reload

# Active au démarrage
sudo systemctl enable sagittarius

# Démarre maintenant
sudo systemctl start sagittarius

# Vérifie le statut
sudo systemctl status sagittarius
```

## Commandes utiles

```bash
# Voir les logs en temps réel
sudo journalctl -u sagittarius -f

# Voir les logs des dernières 24h
sudo journalctl -u sagittarius --since "24 hours ago"

# Arrêter le service
sudo systemctl stop sagittarius

# Redémarrer le service
sudo systemctl restart sagittarius

# Désactiver au démarrage
sudo systemctl disable sagittarius
```

## Fichiers générés

- `stats_backup.json` : Sauvegarde automatique des stats en cas d'échec d'envoi
    - Créé dans le répertoire de travail (`/var/lib/sagittarius` pour le service)
    - Supprimé automatiquement après envoi réussi
    - Rechargé au démarrage si présent

## Gestion des erreurs

- **Échec d'envoi API** : Les stats sont sauvegardées dans `stats_backup.json` et réessayées au prochain intervalle
- **Retry automatique** : 3 tentatives avec 2s de délai entre chaque
- **Timeout** : 5 secondes par requête HTTP
- **Ctrl+C** : Sauvegarde propre des stats avant arrêt
- **Crash/Redémarrage** : Les stats sont récupérées depuis le backup

## Sécurité

⚠️ **Attention** : Ce programme nécessite les droits root pour accéder aux périphériques d'entrée (`/dev/input/*`).

- Utilise TOUJOURS une connexion HTTPS pour l'API en production
- Ne partage JAMAIS ta clé secrète (`API_SECRET`)
- Le fichier `.env` doit avoir les permissions `600` (lecture/écriture pour le propriétaire uniquement)
- Les clés sont envoyées dans le header `X-API-Secret`, pas dans l'URL

## Développement

```bash
# Compile en mode debug
cargo build

# Lance avec les logs
RUST_LOG=debug sudo -E cargo run

# Teste sans envoyer à l'API
# (modifie temporairement API_URL vers un serveur local de test)
```

## Dépendances principales

- `input` - Interface avec libinput
- `evdev` - Conversion des keycodes
- `ureq` - Client HTTP
- `serde/serde_json` - Sérialisation JSON
- `chrono` - Gestion des timestamps
- `ctrlc` - Gestion du signal d'interruption

## Licence

À définir

## Auteur

À définir