use input::{Libinput, LibinputInterface};
use input::event::keyboard::KeyboardEventTrait;
use input::event::pointer::PointerScrollEvent;
use evdev::Key;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::os::unix::{fs::OpenOptionsExt, io::OwnedFd};
use std::path::Path;
use std::time::{Duration, Instant};
use std::env;
use std::io::Write;

const BACKUP_FILE: &str = "stats_backup.json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct Stats {
    total_keys: u64,
    total_clicks: u64,
    total_wheels: u64,

    events: HashMap<String, u64>,
}

// Charge les stats depuis le fichier de backup
fn load_backup() -> Option<Stats> {
    match std::fs::read_to_string(BACKUP_FILE) {
        Ok(content) => {
            match serde_json::from_str(&content) {
                Ok(stats) => {
                    println!("📂 Backup trouvé et chargé avec succès");
                    Some(stats)
                }
                Err(e) => {
                    eprintln!("⚠️  Erreur de lecture du backup: {}", e);
                    None
                }
            }
        }
        Err(_) => {
            println!("ℹ️  Aucun backup trouvé, démarrage à zéro");
            None
        }
    }
}

// Sauvegarde les stats dans le fichier de backup
fn save_backup(stats: &Stats) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(stats)?;
    let mut file = File::create(BACKUP_FILE)?;
    file.write_all(json.as_bytes())?;
    println!("💾 Stats sauvegardées dans {}", BACKUP_FILE);
    Ok(())
}

// Supprime le fichier de backup
fn delete_backup() -> Result<(), Box<dyn std::error::Error>> {
    if Path::new(BACKUP_FILE).exists() {
        std::fs::remove_file(BACKUP_FILE)?;
        println!("🗑️  Backup supprimé");
    }
    Ok(())
}

struct Interface;

impl LibinputInterface for Interface {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<OwnedFd, i32> {
        OpenOptions::new()
            .custom_flags(flags)
            .read(true)
            .write((flags & libc::O_WRONLY != 0) || (flags & libc::O_RDWR != 0))
            .open(path)
            .map(|file| file.into())
            .map_err(|err| err.raw_os_error().unwrap())
    }

    fn close_restricted(&mut self, fd: OwnedFd) {
        drop(File::from(fd));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Charge les variables d'environnement depuis le fichier .env
    dotenv::dotenv().ok();

    // Récupère les variables d'environnement
    let api_url = env::var("API_URL")
        .unwrap_or_else(|_| {
            eprintln!("⚠️  Variable API_URL non définie, utilisation de la valeur par défaut");
            "http://localhost:3000/api/stats".to_string()
        });

    let api_secret = env::var("API_SECRET")
        .expect("❌ Variable API_SECRET obligatoire ! Créez un fichier .env avec API_SECRET=votre_clé");

    println!("🔎 Initialisation de libinput...");
    println!("📡 API URL: {}", api_url);

    let mut input = Libinput::new_with_udev(Interface);
    input.udev_assign_seat("seat0").map_err(|_| "Impossible d'assigner le seat")?;

    println!("✅ Libinput initialisé");
    println!("👀 En attente d'événements...\n");

    // Charge le backup si il existe
    let mut stats = load_backup().unwrap_or_default();

    if stats.total_keys > 0 || stats.total_clicks > 0 || stats.total_wheels > 0 {
        println!("📊 Stats restaurées: {} touches, {} clics, {} scrolls\n",
            stats.total_keys, stats.total_clicks, stats.total_wheels);
    }

    let mut last_display = Instant::now();

    loop {
        input.dispatch()?;

        for event in &mut input {
            use input::event::*;

            match event {
                // Événements clavier
                Event::Keyboard(KeyboardEvent::Key(e)) => {
                    if e.key_state() == keyboard::KeyState::Pressed {
                        stats.total_keys += 1;
                        let keycode = e.key();

                        // Utilise evdev pour convertir le keycode en nom
                        let key = Key::new(keycode as u16);
                        let key_name = format!("{:?}", key);

                        *stats.events.entry(key_name).or_insert(0) += 1;
                    }
                }

                // Événements souris - boutons
                Event::Pointer(PointerEvent::Button(e)) => {
                    if e.button_state() == pointer::ButtonState::Pressed {
                        stats.total_clicks += 1;
                        let button = e.button();
                        let click_name = match button {
                            0x110 => "CLICK_LEFT",
                            0x111 => "CLICK_RIGHT",
                            0x112 => "CLICK_MIDDLE",
                            _ => "CLICK_OTHER",
                        };
                        *stats.events.entry(click_name.to_string()).or_insert(0) += 1;
                    }
                }

                // Événements souris - scroll
                Event::Pointer(PointerEvent::ScrollWheel(e)) => {
                    if e.has_axis(pointer::Axis::Vertical) {
                        let value = e.scroll_value(pointer::Axis::Vertical);
                        let crans = (value / 15.0).abs().round() as u64;
                        stats.total_wheels += crans;
                        *stats.events.entry("WHEEL_VERTICAL".to_string()).or_insert(0) += crans;
                    }
                    if e.has_axis(pointer::Axis::Horizontal) {
                        let value = e.scroll_value(pointer::Axis::Horizontal);
                        let crans = (value / 15.0).abs().round() as u64;
                        stats.total_wheels += crans;
                        *stats.events.entry("WHEEL_HORIZONTAL".to_string()).or_insert(0) += crans;
                    }
                }

                _ => {}
            }
        }

        // Envoi à l'API toutes les 10 secondes
        if last_display.elapsed() >= Duration::from_secs(10) {
            // Sérialise en JSON
            match serde_json::to_string(&stats) {
                Ok(json) => {
                    println!("📤 Envoi des stats à l'API...");

                    // Envoi POST à l'API
                    match ureq::post(&api_url)
                        .set("Content-Type", "application/json")
                        .set("X-API-Secret", &api_secret)
                        .send_string(&json) {
                        Ok(response) => {
                            println!("✅ Envoyé avec succès ! Status: {}", response.status());
                            println!("   total_keys: {}, total_clicks: {}, total_wheels: {}",
                                stats.total_keys, stats.total_clicks, stats.total_wheels);

                            // Reset les compteurs et supprime le backup en cas de succès
                            stats = Stats::default();
                            if let Err(e) = delete_backup() {
                                eprintln!("⚠️  Erreur lors de la suppression du backup: {}", e);
                            }
                            println!("🔄 Compteurs réinitialisés\n");
                        }
                        Err(e) => {
                            println!("❌ Erreur d'envoi: {}", e);
                            println!("   JSON qui devait être envoyé: {}", json);

                            // Sauvegarde les stats en cas d'échec
                            if let Err(e) = save_backup(&stats) {
                                eprintln!("❌ Erreur lors de la sauvegarde du backup: {}", e);
                            }

                            println!("⚠️  Les compteurs ne sont PAS réinitialisés, réessai dans 10s\n");
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Erreur de sérialisation JSON: {}", e);
                }
            }

            last_display = Instant::now();
        }

        // Petite pause
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}