//! W3b rolls a failed release back to the previous binary. That binary must still
//! start on a database the newer release has already migrated.

use capsnap_server::{AppState, Config};

#[tokio::test]
async fn an_older_binary_starts_on_a_newer_schema() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config::for_dir(dir.path(), "https://guests.example");

    let state = AppState::open(cfg.clone()).await.unwrap();
    // A migration this binary doesn't know about, as a newer release would leave it.
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
         VALUES (99990101000000, 'from a newer release', 1, x'00', 0)",
    )
    .execute(&state.pool)
    .await
    .unwrap();
    state.pool.close().await;

    let reopened = AppState::open(cfg).await;
    assert!(reopened.is_ok(), "{:?}", reopened.err());
}
