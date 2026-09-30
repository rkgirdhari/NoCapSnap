//! `capsnap-server` — run the server, or administer it from the host shell.

use std::io::Read;
use std::process::ExitCode;

use capsnap_server::{AppState, Config, admin, retention};

const USAGE: &str = "usage:
  capsnap-server serve
  capsnap-server create-org <slug> <name>
  capsnap-server create-location <org-slug> <name> <iana-timezone>
  capsnap-server create-staff <org-slug> <login> <display-name> <admin|manager|chef|server> [location-id ...]
      (reads the password from standard input)
  capsnap-server import-menu <location-id> <menu.json>   ([{\"name\":…,\"category\":…}, …])
  capsnap-server revoke-sessions <login>
  capsnap-server retention

environment: CAPSNAP_PUBLIC_BASE_URL (required), CAPSNAP_DATA_DIR (./data), CAPSNAP_BIND (127.0.0.1:8080)";

#[tokio::main]
async fn main() -> ExitCode {
    // Plain text when not on a terminal (journald), so logs carry no colour codes.
    let ansi = std::io::IsTerminal::is_terminal(&std::io::stdout());
    tracing_subscriber::fmt()
        .with_target(false)
        .with_ansi(ansi)
        .init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args).await {
        Ok(msg) => {
            if !msg.is_empty() {
                println!("{msg}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: &[String]) -> Result<String, String> {
    let cfg = Config::from_env()?;
    let state = AppState::open(cfg)
        .await
        .map_err(|e| format!("could not open the database: {e}"))?;
    let pool = &state.pool;
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    match a.as_slice() {
        ["serve"] => capsnap_server::serve(state)
            .await
            .map(|_| String::new())
            .map_err(|e| e.to_string()),
        ["create-org", slug, name] => admin::create_org(pool, slug, name).await,
        ["create-location", org, name, tz] => admin::create_location(pool, org, name, tz).await,
        ["create-staff", org, login, name, role, locations @ ..] => {
            let mut password = String::new();
            std::io::stdin()
                .read_to_string(&mut password)
                .map_err(|e| e.to_string())?;
            let password = password.lines().next().unwrap_or("").to_owned();
            let locations: Vec<String> = locations.iter().map(|s| s.to_string()).collect();
            admin::create_staff(pool, org, login, name, role, &password, &locations).await
        }
        ["import-menu", location, file] => {
            let json = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
            let items: Vec<admin::MenuEntry> =
                serde_json::from_str(&json).map_err(|e| format!("{file}: {e}"))?;
            admin::import_menu(pool, location, &items)
                .await
                .map(|n| format!("{n} dishes"))
        }
        ["revoke-sessions", login] => admin::revoke_sessions(pool, login)
            .await
            .map(|n| format!("{n} sessions revoked")),
        ["retention"] => retention::run(pool, &state.cfg.media_dir, chrono::Utc::now())
            .await
            .map(|r| format!("{r:?}"))
            .map_err(|e| e.to_string()),
        _ => Err(USAGE.into()),
    }
}
