//! Maintenance-gated restore with a durable rollback journal and safety snapshot.
use crate::{
    attachments::{assets_path, safe_join},
    maintenance::Maintenance,
    snapshots::{self, Manifest},
    sqlite::{MIGRATIONS, Result, Store, check_integrity, checksum},
};
use brickxinator_core::ledger::Id;
use rusqlite::{TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Journal {
    rollback_directory: String,
    had_assets: bool,
    committed: bool,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RestoreReport {
    pub manifest: Manifest,
    pub safety_snapshot: PathBuf,
    pub config_for_review: Option<PathBuf>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct UpgradeReport {
    pub previous_schema: u64,
    pub current_schema: u64,
    pub safety_snapshot: Option<PathBuf>,
}
fn sidecar(database: &Path, suffix: &str) -> PathBuf {
    let mut name = database.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}
pub(crate) fn pending(database: &Path) -> bool {
    sidecar(database, ".restore.json").exists()
}
fn write_journal(database: &Path, journal: &Journal) -> Result<()> {
    let parent = database.parent().ok_or("database parent missing")?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    staged.write_all(&serde_json::to_vec(journal)?)?;
    staged.as_file().sync_all()?;
    staged
        .persist(sidecar(database, ".restore.json"))
        .map_err(|e| e.error)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}
fn remove(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
fn cleanup_sqlite(database: &Path) -> Result<()> {
    remove(&sidecar(database, "-wal"))?;
    remove(&sidecar(database, "-shm"))?;
    Ok(())
}
fn require_no_pending(database: &Path) -> Result<()> {
    if pending(database) {
        return Err(
            "restore was interrupted; run recover before opening or changing this database".into(),
        );
    }
    Ok(())
}

pub fn restore(
    database: impl AsRef<Path>,
    archive: impl AsRef<Path>,
    safety_directory: impl AsRef<Path>,
) -> Result<RestoreReport> {
    let validated = snapshots::validate(archive)?;
    let guard = Maintenance::acquire(database)?;
    restore_validated_locked(&guard, validated, safety_directory.as_ref())
}

pub(crate) fn restore_validated_locked(
    guard: &Maintenance,
    validated: snapshots::Validated,
    safety_directory: &Path,
) -> Result<RestoreReport> {
    require_no_pending(&guard.database)?;
    fs::create_dir_all(safety_directory)?;
    let safety = safety_directory.join(format!("before-restore-{}.zip", Id::new()));
    let config =
        crate::backups::config_path(guard.database.parent().ok_or("database parent missing")?);
    snapshots::snapshot_locked(
        guard,
        &safety,
        "before-restore",
        config.exists().then_some(config.as_path()),
    )?;
    let store = Store::under_maintenance(guard)?;
    store
        .connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
    drop(store);
    cleanup_sqlite(&guard.database)?;
    let parent = guard.database.parent().ok_or("database parent missing")?;
    let rollback_name = format!(".brickxinator-rollback-{}", Id::new());
    let rollback = parent.join(&rollback_name);
    fs::create_dir(&rollback)?;
    let mut journal = Journal {
        rollback_directory: rollback_name,
        had_assets: assets_path(&guard.database).exists(),
        committed: false,
    };
    write_journal(&guard.database, &journal)?;
    let result = (|| -> Result<RestoreReport> {
        fs::rename(&guard.database, rollback.join("database.sqlite"))?;
        if journal.had_assets {
            fs::rename(assets_path(&guard.database), rollback.join("assets"))?;
        }
        File::open(&rollback)?.sync_all()?;
        File::open(parent)?.sync_all()?;
        snapshots::copy_file(&validated.database(), &guard.database)?;
        snapshots::copy_assets(
            &snapshots::staged_assets(&validated),
            &assets_path(&guard.database),
        )?;
        File::open(parent)?.sync_all()?;
        let store = Store::under_maintenance(guard)?;
        if crate::sqlite::has_attachments(&store.connection)? {
            for record in store.attachments()? {
                store.attachment_bytes(record.id)?;
            }
        }
        store
            .connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        drop(store);
        let config_for_review = if let Some(source) = validated.config() {
            let destination =
                parent.join(format!("restored-config-{}.json", validated.manifest.id));
            snapshots::copy_file(&source, &destination)?;
            Some(destination)
        } else {
            None
        };
        journal.committed = true;
        write_journal(&guard.database, &journal)?;
        remove(&rollback)?;
        fs::remove_file(sidecar(&guard.database, ".restore.json"))?;
        File::open(parent)?.sync_all()?;
        Ok(RestoreReport {
            manifest: validated.manifest,
            safety_snapshot: safety,
            config_for_review,
        })
    })();
    if result.is_err() && pending(&guard.database) {
        recover_locked(guard)?;
    }
    result
}

pub fn recover_interrupted(database: impl AsRef<Path>) -> Result<bool> {
    let guard = Maintenance::acquire(database)?;
    recover_locked(&guard)
}
fn recover_locked(guard: &Maintenance) -> Result<bool> {
    if !pending(&guard.database) {
        return Ok(false);
    }
    let journal: Journal =
        serde_json::from_slice(&fs::read(sidecar(&guard.database, ".restore.json"))?)?;
    if journal
        .rollback_directory
        .strip_prefix(".brickxinator-rollback-")
        .and_then(|id| serde_json::from_value::<Id>(serde_json::Value::String(id.to_owned())).ok())
        .is_none()
    {
        return Err("invalid restore journal directory".into());
    }
    let parent = guard.database.parent().ok_or("database parent missing")?;
    let rollback = safe_join(parent, &journal.rollback_directory)?;
    if !journal.committed {
        let old = rollback.join("database.sqlite");
        if old.exists() {
            cleanup_sqlite(&guard.database)?;
            remove(&guard.database)?;
            fs::rename(old, &guard.database)?;
        }
        let old_assets = rollback.join("assets");
        if old_assets.exists() {
            remove(&assets_path(&guard.database))?;
            fs::rename(old_assets, assets_path(&guard.database))?;
        } else if !journal.had_assets {
            remove(&assets_path(&guard.database))?;
        }
    }
    let store = Store::under_maintenance(guard)?;
    if crate::sqlite::has_attachments(&store.connection)? {
        for attachment in store.attachments()? {
            store.attachment_bytes(attachment.id)?;
        }
    }
    drop(store);
    remove(&rollback)?;
    fs::remove_file(sidecar(&guard.database, ".restore.json"))?;
    File::open(parent)?.sync_all()?;
    Ok(true)
}

pub fn upgrade(
    database: impl AsRef<Path>,
    safety_directory: impl AsRef<Path>,
) -> Result<UpgradeReport> {
    let guard = Maintenance::acquire(database)?;
    require_no_pending(&guard.database)?;
    let mut store = Store::under_maintenance(&guard)?;
    let previous: i64 =
        store
            .connection
            .query_row("SELECT max(version) FROM schema_migration", [], |r| {
                r.get(0)
            })?;
    if previous as usize == MIGRATIONS.len() {
        return Ok(UpgradeReport {
            previous_schema: previous as u64,
            current_schema: previous as u64,
            safety_snapshot: None,
        });
    }
    drop(store);
    fs::create_dir_all(&safety_directory)?;
    let safety = safety_directory
        .as_ref()
        .join(format!("before-upgrade-{}.zip", Id::new()));
    let config =
        crate::backups::config_path(guard.database.parent().ok_or("database parent missing")?);
    snapshots::snapshot_locked(
        &guard,
        &safety,
        "before-upgrade",
        config.exists().then_some(config.as_path()),
    )?;
    store = Store::under_maintenance(&guard)?;
    let tx = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Exclusive)?;
    for (index, (name, sql)) in MIGRATIONS.iter().enumerate().skip(previous as usize) {
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO schema_migration(version,name,checksum) VALUES(?1,?2,?3)",
            params![i64::try_from(index + 1)?, name, checksum(sql)],
        )?;
    }
    check_integrity(&tx)?;
    crate::sqlite::load(&tx)?;
    tx.commit()?;
    drop(store);
    Store::under_maintenance(&guard)?;
    Ok(UpgradeReport {
        previous_schema: previous as u64,
        current_schema: MIGRATIONS.len() as u64,
        safety_snapshot: Some(safety),
    })
}
