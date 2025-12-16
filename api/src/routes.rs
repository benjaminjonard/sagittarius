use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::Row;
use crate::models::{Stats, AppState, get_event_type};

// Page web d'accueil avec dashboard
pub async fn index() -> HttpResponse {
    let html = include_str!("templates/dashboard.html");
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

// Endpoint de santé
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "sagittarius-server"
    }))
}

// Endpoint pour recevoir les stats
pub async fn receive_stats(
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

    // Met à jour la date du dernier envoi
    sqlx::query(
        r#"
        INSERT INTO metadata (key, value, updated_at)
        VALUES ('last_sync', datetime('now'), datetime('now'))
        ON CONFLICT(key) DO UPDATE SET
            value = datetime('now'),
            updated_at = datetime('now')
        "#
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Initialise la date de début si c'est le premier envoi
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO metadata (key, value, updated_at)
        VALUES ('first_sync', datetime('now'), datetime('now'))
        "#
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

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

// Endpoint pour récupérer les stats globales
pub async fn get_stats(
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

    // Récupère la date du dernier envoi
    let last_sync = sqlx::query(
        r#"
        SELECT value FROM metadata WHERE key = 'last_sync'
        "#
    )
    .fetch_optional(&data.db)
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
    .and_then(|row| row.get::<Option<String>, _>("value"));

    // Récupère la date de début de collecte
    let first_sync = sqlx::query(
        r#"
        SELECT value FROM metadata WHERE key = 'first_sync'
        "#
    )
    .fetch_optional(&data.db)
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
    .and_then(|row| row.get::<Option<String>, _>("value"));

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "total_keys": total_keys,
        "total_clicks": total_clicks,
        "total_wheels": total_wheels,
        "last_sync": last_sync,
        "first_sync": first_sync,
        "events": events
    })))
}