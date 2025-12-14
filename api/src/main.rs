use actix_cors::Cors;
use actix_web::http::header;
use actix_web::{web, App, HttpServer, HttpRequest, HttpResponse, middleware};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions, Row};
use std::collections::HashMap;
use std::env;

#[derive(Debug, Deserialize, Serialize)]
struct Stats {
    total_keys: i64,
    total_clicks: i64,
    total_wheels: i64,
    events: HashMap<String, i64>,
    timestamp: Option<String>,
    hostname: Option<String>,
}

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    api_secret: String,
}

// Détermine le type d'événement
fn get_event_type(event_name: &str) -> &str {
    if event_name.starts_with("KEY_") {
        "KEY"
    } else if event_name.starts_with("CLICK_") {
        "CLICK"
    } else if event_name.starts_with("WHEEL_") {
        "WHEEL"
    } else {
        "OTHER"
    }
}

// Endpoint pour recevoir les stats
async fn receive_stats(
    req: HttpRequest,
    stats: web::Json<Stats>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Vérifie l'authentification
    let api_secret = req.headers()
        .get("X-API-Secret")
        .and_then(|h| h.to_str().ok());

    if api_secret != Some(&data.api_secret) {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Unauthorized - Invalid or missing API secret"
        })));
    }

    // Commence une transaction
    let mut tx = data.db.begin().await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Upsert chaque événement (INSERT ou UPDATE si existe)
    for (event_name, count) in &stats.events {
        let event_type = get_event_type(event_name);
        
        sqlx::query(
            r#"
            INSERT INTO events (event_name, event_type, count)
            VALUES (?, ?, ?)
            ON CONFLICT(event_name) DO UPDATE SET
                count = count + excluded.count,
                updated_at = CURRENT_TIMESTAMP
            "#
        )
        .bind(event_name)
        .bind(event_type)
        .bind(count)
        .execute(&mut *tx)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    }

    // Commit la transaction
    tx.commit().await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    println!("✅ Stats mises à jour: {} touches, {} clics, {} scrolls ({} événements)", 
        stats.total_keys, 
        stats.total_clicks, 
        stats.total_wheels,
        stats.events.len()
    );
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Stats updated successfully",
        "events_processed": stats.events.len()
    })))
}

// Endpoint de santé
async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "sagittarius-server"
    }))
}

// Endpoint pour récupérer les stats globales
async fn get_stats(
    req: HttpRequest,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Vérifie l'authentification
    let api_secret = req.headers()
        .get("X-API-Secret")
        .and_then(|h| h.to_str().ok());

    if api_secret != Some(&data.api_secret) {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Unauthorized - Invalid or missing API secret"
        })));
    }

    // Récupère tous les événements triés par count
    let rows = sqlx::query(
        r#"
        SELECT event_name, event_type, count
        FROM events
        ORDER BY count DESC
        "#
    )
    .fetch_all(&data.db)
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let mut total_keys: i64 = 0;
    let mut total_clicks: i64 = 0;
    let mut total_wheels: i64 = 0;
    let mut events = Vec::new();

    for row in rows {
        let event_name: String = row.get("event_name");
        let event_type: String = row.get("event_type");
        let count: i64 = row.get("count");

        match event_type.as_str() {
            "KEY" => total_keys += count,
            "CLICK" => total_clicks += count,
            "WHEEL" => total_wheels += count,
            _ => {}
        }

        events.push(serde_json::json!({
            "name": event_name,
            "type": event_type,
            "count": count
        }));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "total_keys": total_keys,
        "total_clicks": total_clicks,
        "total_wheels": total_wheels,
        "events": events
    })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Charge les variables d'environnement
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://sagittarius.db".to_string());
    
    let api_secret = env::var("API_SECRET")
        .expect("API_SECRET must be set");
    
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    println!("🔧 Initialisation du serveur...");
    println!("📂 Database: {}", database_url);

    // Connexion à la base de données
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Crée la table events avec contrainte unique sur event_name
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_name TEXT NOT NULL UNIQUE,
            event_type TEXT NOT NULL CHECK(event_type IN ('KEY', 'CLICK', 'WHEEL', 'OTHER')),
            count INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(&pool)
    .await
    .expect("Failed to create events table");

    // Crée les index pour les performances
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type)")
        .execute(&pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_count ON events(count)")
        .execute(&pool)
        .await
        .ok();

    println!("✅ Base de données initialisée");

    let app_state = AppState {
        db: pool,
        api_secret: api_secret.clone(),
    };

    println!("🚀 Serveur démarré sur http://{}:{}", host, port);
    println!("📡 Endpoints:");
    println!("   POST /api/stats  - Receive and update stats");
    println!("   GET  /api/stats  - Get all stats");
    println!("   GET  /health     - Health check");

    HttpServer::new(move || {
        let frontend_origin = env::var("CORS_ALLOW_ORIGIN")
            .unwrap_or_else(|_| "*".to_string()); // fallback to "*" if not set

        let cors = Cors::default()
            .allowed_origin(&frontend_origin)
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec![header::CONTENT_TYPE, header::HeaderName::from_static("x-api-secret")])
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .route("/health", web::get().to(health))
            .route("/api/stats", web::post().to(receive_stats))
            .route("/api/stats", web::get().to(get_stats))
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}