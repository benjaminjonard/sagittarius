use actix_cors::Cors;
use actix_web::http::header;
use actix_web::{web, App, HttpServer, middleware};
use sqlx::sqlite::SqlitePoolOptions;
use std::env;

mod models;
mod routes;

use models::AppState;
use routes::{index, health, receive_stats, get_stats};

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

    // Crée la table metadata pour stocker la dernière date d'envoi
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(&pool)
    .await
    .expect("Failed to create metadata table");

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
    println!("   GET  /              - Dashboard web");
    println!("   POST /api/stats     - Receive and update stats");
    println!("   GET  /api/stats     - Get all stats");
    println!("   GET  /health        - Health check");

    HttpServer::new(move || {
        let frontend_origin = env::var("CORS_ALLOW_ORIGIN")
            .unwrap_or_else(|_| "*".to_string());

        let cors = Cors::default()
            .allowed_origin(&frontend_origin)
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec![header::CONTENT_TYPE, header::HeaderName::from_static("x-api-secret")])
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .route("/", web::get().to(index))
            .route("/health", web::get().to(health))
            .route("/api/stats", web::post().to(receive_stats))
            .route("/api/stats", web::get().to(get_stats))
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}