use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Stats {
    pub total_keys: i64,
    pub total_clicks: i64,
    pub total_wheels: i64,
    pub events: HashMap<String, i64>,
}

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub api_secret: String,
}

// Détermine le type d'événement
pub fn get_event_type(event_name: &str) -> &str {
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