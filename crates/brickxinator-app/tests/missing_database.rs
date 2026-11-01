use std::process::Command;
#[test]
fn backup_and_maintenance_never_create_a_missing_database() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("missing");
    for command in [
        "backup-run",
        "snapshot",
        "doctor",
        "upgrade",
        "export-native",
        "export-csv",
        "import-native",
        "restore",
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_brickxinator-app"))
            .arg(command)
            .arg("--data-dir")
            .arg(&missing)
            .output()
            .unwrap();
        assert!(
            !result.status.success(),
            "{command} unexpectedly initialized missing data"
        );
        assert!(!missing.exists());
    }
}
#[test]
fn automatic_startup_refuses_to_replace_a_lost_database_in_an_existing_collection() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("backup-config.json"), b"{}").unwrap();
    for command in ["serve", "backup-init"] {
        let result = Command::new(env!("CARGO_BIN_EXE_brickxinator-app"))
            .arg(command)
            .arg("--data-dir")
            .arg(root.path())
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(!root.path().join("database.sqlite").exists());
        assert_eq!(
            std::fs::read(root.path().join("backup-config.json")).unwrap(),
            b"{}"
        );
    }
}
