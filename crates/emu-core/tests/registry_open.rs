//! `Registry::open` bootstraps and migrates the SQLite registry.

use emu_core::registry::Registry;

#[tokio::test]
async fn registry_open_creates_file_runs_migrations_and_is_idempotent() {
    let dir = tempfile::tempdir().expect("tempdir");

    let registry = Registry::open(dir.path()).await.expect("open registry");
    assert!(dir.path().join("db.sqlite").exists(), "db file was created");

    // A migrated table is queryable and empty.
    let (emulators,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM emulators")
        .fetch_one(registry.pool())
        .await
        .expect("query the emulators table");
    assert_eq!(emulators, 0);

    drop(registry);

    // Re-opening the same directory succeeds and re-runs zero migrations.
    let reopened = Registry::open(dir.path()).await.expect("re-open registry");
    let (jobs,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM jobs")
        .fetch_one(reopened.pool())
        .await
        .expect("query after re-open");
    assert_eq!(jobs, 0);
}
