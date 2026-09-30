//! Runs the outbox self-test against a real file. On a device:
//!   adb push selftest /data/local/tmp/ && adb shell /data/local/tmp/selftest /data/local/tmp/w0.db
use std::path::PathBuf;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("capsnap-w0-selftest.db"));
    let _ = std::fs::remove_file(&path);
    match capsnap_store::selftest(&path).await {
        Ok(summary) => println!("W0 selftest {summary} path={}", path.display()),
        Err(err) => {
            eprintln!("W0 selftest FAILED: {err}");
            std::process::exit(1);
        }
    }
}
