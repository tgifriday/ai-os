mod config;
mod logger;
mod service;

use config::AiosConfig;
use service::{ServiceConfig, ServiceManager};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::info;

fn find_config() -> AiosConfig {
    let paths = [
        PathBuf::from("config/aios.toml"),
        PathBuf::from("/etc/aios/aios.toml"),
    ];

    paths
        .iter()
        .find_map(|p| AiosConfig::load(p).ok())
        .unwrap_or_default()
}

fn find_binary(name: &str) -> String {
    let candidates = [
        format!("./target/release/{}", name),
        format!("./target/debug/{}", name),
        format!("/usr/local/bin/{}", name),
        format!("/bin/{}", name),
    ];

    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return path.clone();
        }
    }

    name.to_string()
}

fn register_services(manager: &mut ServiceManager, config: &AiosConfig) {
    let shell_bin = find_binary("aios-shell");

    if config.services.shell_sessions {
        manager.register(ServiceConfig {
            name: "aios-shell".to_string(),
            command: shell_bin,
            args: vec![],
            auto_start: false,
            restart_on_failure: false,
            max_restarts: 0,
            depends_on: vec![],
            environment: HashMap::new(),
        });
    }
}

fn main() {
    let config = find_config();
    logger::init_logging(&config.system.log_level);

    info!("AIOS Init System starting");
    info!(hostname = %config.system.hostname, "system configuration loaded");

    let mut manager = ServiceManager::new();
    register_services(&mut manager, &config);

    manager.start_all_auto();

    let status = manager.status();
    for (name, state, pid) in &status {
        info!(service = %name, state = %state, pid = ?pid, "service status");
    }

    info!("AIOS Init: all services started, entering monitoring loop");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    let _ = std::thread::spawn(move || {
        let mut buf = String::new();
        loop {
            if std::io::stdin().read_line(&mut buf).is_err() {
                r.store(false, Ordering::SeqCst);
                break;
            }
            let trimmed = buf.trim();
            if trimmed == "quit" || trimmed == "shutdown" {
                r.store(false, Ordering::SeqCst);
                break;
            }
            if trimmed == "status" {
                println!("(check logs for service status)");
            }
            buf.clear();
        }
    });

    while running.load(Ordering::SeqCst) {
        manager.check_services();
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    info!("AIOS Init: shutting down");
    manager.stop_all();
    info!("AIOS Init: goodbye");
}
