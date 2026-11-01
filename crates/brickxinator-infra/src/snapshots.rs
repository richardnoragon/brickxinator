//! Consistent, self-contained ZIP snapshots with bounded, validated extraction.
use crate::{
    attachments::{assets_path, safe_join},
    maintenance::Maintenance,
    sqlite::{Result, Store, has_attachments},
};
use brickxinator_core::ledger::Id;
use rusqlite::MAIN_DB;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub sha256: String,
    pub bytes: u64,
}
// Hash large databases with bounded memory, including during archive validation.
fn hash_file(path: &Path) -> Result<String> {
    let mut input = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    pub id: Id,
    pub created_at: String,
    pub kind: String,
    pub app_version: String,
    pub schema_version: u64,
    pub last_operation_sequence: u64,
    pub review_config_before_activation: bool,
    pub files: BTreeMap<String, Entry>,
}
pub struct Validated {
    pub manifest: Manifest,
    pub(crate) staging: tempfile::TempDir,
}
impl Validated {
    pub fn database(&self) -> PathBuf {
        self.staging.path().join("database.sqlite")
    }
    pub fn config(&self) -> Option<PathBuf> {
        let path = self.staging.path().join("config.json");
        path.exists().then_some(path)
    }
}

pub fn snapshot(
    database: impl AsRef<Path>,
    output: impl AsRef<Path>,
    kind: &str,
    config: Option<&Path>,
) -> Result<Manifest> {
    let guard = Maintenance::acquire(database)?;
    snapshot_locked(&guard, output.as_ref(), kind, config)
}
pub(crate) fn snapshot_locked(
    guard: &Maintenance,
    output: &Path,
    kind: &str,
    config: Option<&Path>,
) -> Result<Manifest> {
    if output.exists() {
        return Err("snapshot destination already exists".into());
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let staging = tempfile::tempdir_in(parent)?;
    let copy = staging.path().join("database.sqlite");
    let store = Store::under_maintenance(guard)?;
    store
        .connection
        .backup(MAIN_DB, &copy, rusqlite::backup::NO_PROGRESS)?;
    let mut payloads = BTreeMap::<String, PathBuf>::from([("database.sqlite".into(), copy)]);
    if has_attachments(&store.connection)? {
        for record in store.attachments()? {
            let (_, bytes) = store.attachment_bytes(record.id)?;
            let destination = safe_join(
                staging.path(),
                &format!("attachments/{}", record.relative_path),
            )?;
            fs::create_dir_all(destination.parent().ok_or("missing asset parent")?)?;
            fs::write(&destination, bytes)?;
            payloads.insert(format!("attachments/{}", record.relative_path), destination);
        }
    }
    if let Some(config) = config {
        let bytes = fs::read(config)?;
        let _: serde_json::Value = serde_json::from_slice(&bytes)?;
        let path = staging.path().join("config.json");
        fs::write(&path, bytes)?;
        payloads.insert("config.json".into(), path);
    }
    let schema: i64 =
        store
            .connection
            .query_row("SELECT max(version) FROM schema_migration", [], |r| {
                r.get(0)
            })?;
    let sequence: i64 = store.connection.query_row(
        "SELECT coalesce(max(sequence),0) FROM inventory_operation",
        [],
        |r| r.get(0),
    )?;
    let mut files = BTreeMap::new();
    for (name, path) in &payloads {
        files.insert(
            name.clone(),
            Entry {
                sha256: hash_file(path)?,
                bytes: fs::metadata(path)?.len(),
            },
        );
    }
    let manifest = Manifest {
        format: "brickxinator-snapshot".into(),
        version: 1,
        id: Id::new(),
        created_at: chrono::Utc::now().to_rfc3339(),
        kind: kind.into(),
        app_version: env!("CARGO_PKG_VERSION").into(),
        schema_version: u64::try_from(schema)?,
        last_operation_sequence: u64::try_from(sequence)?,
        review_config_before_activation: true,
        files,
    };
    drop(store);
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut zip = ZipWriter::new(temporary.as_file_mut());
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o600);
        zip.start_file("metadata.json", options)?;
        zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
        for (name, path) in payloads {
            zip.start_file(name, options)?;
            std::io::copy(&mut File::open(path)?, &mut zip)?;
        }
        zip.finish()?;
    }
    temporary.as_file().sync_all()?;
    validate(temporary.path())?;
    temporary.persist_noclobber(output).map_err(|e| e.error)?;
    File::open(parent)?.sync_all()?;
    Ok(manifest)
}

pub fn validate(archive: impl AsRef<Path>) -> Result<Validated> {
    let mut zip = ZipArchive::new(File::open(archive)?)?;
    if zip.len() > 100_000 {
        return Err("snapshot contains too many files".into());
    }
    let mut metadata = zip.by_name("metadata.json")?;
    if metadata.size() > 4 * 1024 * 1024 {
        return Err("snapshot manifest exceeds limit".into());
    }
    let mut bytes = Vec::new();
    metadata.read_to_end(&mut bytes)?;
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    drop(metadata);
    if manifest.format != "brickxinator-snapshot"
        || manifest.version != 1
        || manifest.schema_version == 0
        || manifest.schema_version > crate::sqlite::MIGRATIONS.len() as u64
        || !manifest.files.contains_key("database.sqlite")
    {
        return Err("unsupported or incomplete snapshot format/schema".into());
    }
    let staging = tempfile::tempdir()?;
    let mut seen = BTreeSet::new();
    let mut total = 0_u64;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name()?.to_string();
        if !seen.insert(name.clone())
            || entry.is_dir()
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
            || entry.enclosed_name().is_none()
        {
            return Err("duplicate, symlink or unsafe snapshot entry".into());
        }
        if name == "metadata.json" {
            continue;
        }
        if name != "database.sqlite" && name != "config.json" && !name.starts_with("attachments/") {
            return Err("unexpected snapshot entry".into());
        }
        let expected = manifest.files.get(&name).ok_or("unlisted snapshot entry")?;
        if entry.size() != expected.bytes
            || expected.bytes > 4 * 1024 * 1024 * 1024
            || expected.sha256.len() != 64
        {
            return Err("snapshot size/hash metadata invalid".into());
        }
        total = total
            .checked_add(expected.bytes)
            .ok_or("snapshot size overflow")?;
        if total > 16 * 1024 * 1024 * 1024 {
            return Err("snapshot extracted size exceeds limit".into());
        }
        let relative = if let Some(asset) = name.strip_prefix("attachments/") {
            format!("database.sqlite.assets/{asset}")
        } else {
            name.clone()
        };
        let path = safe_join(staging.path(), &relative)?;
        fs::create_dir_all(path.parent().ok_or("entry parent missing")?)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        let copied = std::io::copy(&mut entry.by_ref().take(expected.bytes + 1), &mut file)?;
        file.sync_all()?;
        if copied != expected.bytes || hash_file(&path)? != expected.sha256 {
            return Err("snapshot content hash mismatch".into());
        }
    }
    if seen.len() != manifest.files.len() + 1
        || manifest.files.keys().any(|name| !seen.contains(name))
    {
        return Err("snapshot is missing listed files".into());
    }
    let database = staging.path().join("database.sqlite");
    let guard = Maintenance::acquire(&database)?;
    let store = Store::under_maintenance(&guard)?;
    let schema: i64 =
        store
            .connection
            .query_row("SELECT max(version) FROM schema_migration", [], |r| {
                r.get(0)
            })?;
    let sequence: i64 = store.connection.query_row(
        "SELECT coalesce(max(sequence),0) FROM inventory_operation",
        [],
        |r| r.get(0),
    )?;
    if schema as u64 != manifest.schema_version
        || sequence as u64 != manifest.last_operation_sequence
    {
        return Err("manifest does not match database generation".into());
    }
    let mut referenced = BTreeSet::new();
    if has_attachments(&store.connection)? {
        for record in store.attachments()? {
            store.attachment_bytes(record.id)?;
            referenced.insert(format!("attachments/{}", record.relative_path));
        }
    }
    let archived: BTreeSet<_> = manifest
        .files
        .keys()
        .filter(|name| name.starts_with("attachments/"))
        .cloned()
        .collect();
    if referenced != archived {
        return Err("snapshot files do not exactly cover referenced attachments".into());
    }
    store
        .connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
    drop(store);
    drop(guard);
    Ok(Validated { manifest, staging })
}

pub(crate) fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    fs::copy(source, destination)?;
    File::open(destination)?.sync_all()?;
    Ok(())
}
pub(crate) fn copy_assets(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    if source.exists() {
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let meta = entry.file_type()?;
            if !meta.is_file() || meta.is_symlink() {
                return Err("assets directory contains unsupported nested/symlink entry".into());
            }
            copy_file(&entry.path(), &destination.join(entry.file_name()))?;
        }
    }
    File::open(destination)?.sync_all()?;
    Ok(())
}
pub(crate) fn staged_assets(validated: &Validated) -> PathBuf {
    assets_path(&validated.database())
}
