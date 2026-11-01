//! Native v1 collection exchange: typed named records and relative asset files.
//! Import into an empty collection only; existing data is never merged implicitly.
use crate::{
    attachments::{assets_path, safe_join},
    maintenance::Maintenance,
    snapshots,
    sqlite::{Result, Store},
};
use rusqlite::{
    params_from_iter,
    types::{Value, ValueRef},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

// Fixed v1 entity registry. Archive input never supplies SQL identifiers/schema.
const TABLES: &[&str] = &[
    "schema_migration",
    "stock_holding",
    "inventory_operation",
    "operation_target",
    "purchase",
    "invoice_line",
    "purchase_command",
    "receipt",
    "stock_provenance",
    "owned_set_copy",
    "settings_record",
    "settings_command",
    "catalog_definition",
    "catalog_command",
    "inventory_profile",
    "profile_command",
    "verification_line_identity",
    "verified_content",
    "verification_operation",
    "stock_operation",
    "shipping_allocation",
    "shipping_command",
    "attachment",
    "attachment_association",
    "attachment_command",
    "purchase_deletion",
    "deleted_request_key",
    "purchase_reconciliation",
    "verification_attribute_revision",
];
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Cell {
    Null,
    Text(String),
    Integer(String),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Records {
    pub fields: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Native {
    pub format: String,
    pub version: u32,
    pub schema_version: usize,
    pub records: BTreeMap<String, Records>,
    pub assets: BTreeMap<String, snapshots::Entry>,
}
fn records(store: &Store) -> Result<BTreeMap<String, Records>> {
    let mut result = BTreeMap::new();
    for table in TABLES {
        let mut query = store
            .connection
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))?;
        let fields = query.column_names().map(str::to_owned).collect::<Vec<_>>();
        let count = fields.len();
        let rows = query
            .query_map([], |row| {
                (0..count)
                    .map(|i| match row.get_ref(i)? {
                        ValueRef::Null => Ok(Cell::Null),
                        ValueRef::Text(text) => Ok(Cell::Text(
                            std::str::from_utf8(text)
                                .map_err(|_| rusqlite::Error::InvalidQuery)?
                                .to_owned(),
                        )),
                        ValueRef::Integer(value) => Ok(Cell::Integer(value.to_string())),
                        _ => Err(rusqlite::Error::InvalidQuery),
                    })
                    .collect::<rusqlite::Result<Vec<_>>>()
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        result.insert((*table).into(), Records { fields, rows });
    }
    Ok(result)
}
pub fn export(database: impl AsRef<Path>, directory: impl AsRef<Path>) -> Result<Native> {
    let directory = directory.as_ref();
    if directory.exists() {
        return Err("exchange destination already exists".into());
    }
    let parent = directory.parent().ok_or("exchange parent missing")?;
    fs::create_dir_all(parent)?;
    let temporary = tempfile::tempdir_in(parent)?;
    let guard = Maintenance::acquire(database)?;
    let store = Store::under_maintenance(&guard)?;
    let schema_version: usize =
        store
            .connection
            .query_row("SELECT max(version) FROM schema_migration", [], |r| {
                r.get::<_, u32>(0).map(|version| version as usize)
            })?;
    if schema_version != crate::sqlite::MIGRATIONS.len() {
        return Err("upgrade before native export".into());
    }
    let mut assets = BTreeMap::new();
    for attachment in store.attachments()? {
        let (_, bytes) = store.attachment_bytes(attachment.id)?;
        let path = format!("assets/{}", attachment.relative_path);
        let target = safe_join(temporary.path(), &path)?;
        fs::create_dir_all(target.parent().ok_or("asset parent missing")?)?;
        fs::write(target, bytes)?;
        assets.insert(
            path,
            snapshots::Entry {
                sha256: attachment.sha256,
                bytes: attachment.byte_size,
            },
        );
    }
    let native = Native {
        format: "brickxinator-native".into(),
        version: 1,
        schema_version,
        records: records(&store)?,
        assets,
    };
    fs::write(
        temporary.path().join("collection.json"),
        serde_json::to_vec_pretty(&native)?,
    )?;
    fs::rename(temporary.path(), directory)?;
    Ok(native)
}
pub fn export_archive(database: impl AsRef<Path>, output: impl AsRef<Path>) -> Result<()> {
    use std::io::Write;
    let temporary = tempfile::tempdir()?;
    let directory = temporary.path().join("bundle");
    let native = export(database, &directory)?;
    let output = output.as_ref();
    let parent = output.parent().ok_or("export parent missing")?;
    fs::create_dir_all(parent)?;
    let staged = tempfile::NamedTempFile::new_in(parent)?;
    let mut writer = zip::ZipWriter::new(staged.as_file().try_clone()?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o600);
    for name in std::iter::once("collection.json").chain(native.assets.keys().map(String::as_str)) {
        writer.start_file(name, options)?;
        std::io::copy(
            &mut fs::File::open(safe_join(&directory, name)?)?,
            &mut writer,
        )?;
    }
    writer.finish()?.flush()?;
    staged.as_file().sync_all()?;
    staged.persist_noclobber(output)?;
    Ok(())
}
pub fn import_archive(
    database: impl AsRef<Path>,
    archive: impl AsRef<Path>,
    safety: impl AsRef<Path>,
) -> Result<bool> {
    use std::io::Read;
    let directory = tempfile::tempdir()?;
    let mut zip = zip::ZipArchive::new(fs::File::open(archive)?)?;
    if zip.len() > 100_002 {
        return Err("native archive contains too many entries".into());
    }
    let mut entry = zip.by_name("collection.json")?;
    if entry.size() > 256 * 1024 * 1024 {
        return Err("native JSON exceeds 256 MiB limit".into());
    }
    let mut bytes = vec![];
    entry
        .by_ref()
        .take(256 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    drop(entry);
    if bytes.len() > 256 * 1024 * 1024 {
        return Err("native JSON exceeds limit".into());
    }
    let native: Native = serde_json::from_slice(&bytes)?;
    if native.format != "brickxinator-native"
        || native.version != 1
        || native.schema_version != crate::sqlite::MIGRATIONS.len()
    {
        return Err("unsupported native format or schema".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0u64;
    for index in 0..zip.len() {
        let entry = zip.by_index(index)?;
        let name = entry.name()?.into_owned();
        if !seen.insert(name.clone())
            || entry.is_dir()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            || entry.enclosed_name().is_none()
        {
            return Err("unsafe or duplicate native archive entry".into());
        }
        let size = if name == "collection.json" {
            bytes.len() as u64
        } else {
            let expected = native
                .assets
                .get(&name)
                .ok_or("native archive contains an unlisted asset")?;
            if expected.bytes > crate::attachments::MAX_BYTES as u64 {
                return Err("native asset exceeds size limit".into());
            }
            expected.bytes
        };
        total = total.checked_add(size).ok_or("native size overflow")?;
        if total > 16 * 1024 * 1024 * 1024 || entry.size() != size {
            return Err("native archive size limit or manifest mismatch".into());
        }
        let destination = safe_join(directory.path(), &name)?;
        fs::create_dir_all(destination.parent().ok_or("native asset parent missing")?)?;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(destination)?;
        if std::io::copy(&mut entry.take(size + 1), &mut file)? != size {
            return Err("native archive expanded size mismatch".into());
        }
    }
    if seen.len() != native.assets.len() + 1 {
        return Err("native archive is missing referenced assets".into());
    }
    import(database, directory.path(), safety)
}
pub fn import(
    database: impl AsRef<Path>,
    directory: impl AsRef<Path>,
    safety: impl AsRef<Path>,
) -> Result<bool> {
    let directory = directory.as_ref();
    let json = safe_join(directory, "collection.json")?;
    if fs::metadata(&json)?.len() > 256 * 1024 * 1024 {
        return Err("native JSON exceeds 256 MiB limit".into());
    }
    let native: Native = serde_json::from_slice(&fs::read(json)?)?;
    if native.format != "brickxinator-native"
        || native.version != 1
        || native.schema_version != crate::sqlite::MIGRATIONS.len()
        || native
            .records
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            != TABLES.iter().copied().collect()
    {
        return Err("unsupported native format, schema or record family".into());
    }
    let temporary = tempfile::tempdir()?;
    let staged = temporary.path().join("database.sqlite");
    let mut store = Store::create(&staged)?;
    let empty = records(&store)?;
    // Rebuild a private staging database. Triggers come exclusively from our own
    // immutable migrations, never from imported JSON, and are restored before validation.
    let triggers = store
        .connection
        .prepare("SELECT name,sql FROM sqlite_master WHERE type='trigger'")?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    store
        .connection
        .pragma_update(None, "foreign_keys", false)?;
    let tx = store.connection.transaction()?;
    for (name, _) in &triggers {
        tx.execute_batch(&format!("DROP TRIGGER \"{name}\""))?;
    }
    for table in TABLES {
        tx.execute(&format!("DELETE FROM {table}"), [])?;
    }
    for table in TABLES {
        let imported = &native.records[*table];
        if imported.fields != empty[*table].fields || imported.rows.len() > 1_000_000 {
            return Err("native record fields or row limit invalid".into());
        }
        let placeholders = (0..imported.fields.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let mut insert = tx.prepare(&format!("INSERT INTO {table} VALUES({placeholders})"))?;
        for row in &imported.rows {
            if row.len() != imported.fields.len() {
                return Err("native record width mismatch".into());
            }
            let values = row
                .iter()
                .map(|cell| {
                    Ok(match cell {
                        Cell::Null => Value::Null,
                        Cell::Text(text) => Value::Text(text.clone()),
                        Cell::Integer(value) => Value::Integer(value.parse()?),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            insert.execute(params_from_iter(values))?;
        }
    }
    for (_, sql) in triggers {
        tx.execute_batch(&sql)?;
    }
    tx.commit()?;
    store.connection.pragma_update(None, "foreign_keys", true)?;
    drop(store);
    let store = Store::open(&staged)?;
    let mut assets = BTreeMap::new();
    for record in store.attachments()? {
        let path = format!("assets/{}", record.relative_path);
        let expected = snapshots::Entry {
            sha256: record.sha256.clone(),
            bytes: record.byte_size,
        };
        if native.assets.get(&path) != Some(&expected) {
            return Err("native asset manifest differs from attachment metadata".into());
        }
        let source = safe_join(directory, &path)?;
        if fs::metadata(&source)?.len() != record.byte_size
            || record.byte_size > crate::attachments::MAX_BYTES as u64
        {
            return Err("native asset size mismatch".into());
        }
        let target = safe_join(&assets_path(&staged), &record.relative_path)?;
        fs::create_dir_all(target.parent().ok_or("asset parent missing")?)?;
        fs::copy(source, target)?;
        store.attachment_bytes(record.id)?;
        assets.insert(path, expected);
    }
    if assets != native.assets {
        return Err("native asset manifest contains unreferenced files".into());
    }
    drop(store);
    // Reusing restore gives native imports the same snapshot/rollback protocol.
    // Current v1 imports are whole-collection initialization, not implicit merge.
    let guard = Maintenance::acquire(&database)?;
    let current = Store::under_maintenance(&guard)?;
    let current_records = records(&current)?;
    if current_records == native.records {
        for attachment in current.attachments()? {
            current.attachment_bytes(attachment.id)?;
        }
        return Ok(false);
    }
    let unchanged_seed = TABLES
        .iter()
        .filter(|name| **name != "schema_migration")
        .all(|name| current_records[*name] == empty[*name]);
    if !unchanged_seed {
        return Err("native import conflicts with existing collection; use a new empty data directory (implicit merge is unsupported)".into());
    }
    drop(current);
    let archive = temporary.path().join("import.zip");
    snapshots::snapshot(&staged, &archive, "native-import", None)?;
    let validated = snapshots::validate(archive)?;
    crate::recovery::restore_validated_locked(&guard, validated, safety.as_ref())?;
    Ok(true)
}

fn measurement(value: brickxinator_core::inventory::Measure) -> String {
    match value {
        brickxinator_core::inventory::Measure::Known(value) => value.to_string(),
        brickxinator_core::inventory::Measure::Unknown => String::new(),
    }
}

fn csv(value: &str) -> String {
    let safe = if value.trim_start().starts_with(['=', '+', '-', '@'])
        || value.starts_with(['\t', '\r'])
    {
        format!("'{value}")
    } else {
        value.to_owned()
    };
    format!("\"{}\"", safe.replace('"', "\"\""))
}
pub fn inventory_csv(database: impl AsRef<Path>, output: impl AsRef<Path>) -> Result<PathBuf> {
    let mut store = Store::open(database)?;
    let mut query = brickxinator_core::views::Search {
        limit: 200,
        ..Default::default()
    };
    let mut text = String::from("id,kind,label,number,pieces,weight_mg\r\n");
    loop {
        let result = store.search_inventory(&query)?;
        for row in &result.rows {
            let id = row.target.id().to_string();
            let values = [
                id,
                format!("{:?}", row.kind),
                row.label.clone(),
                row.number.clone().unwrap_or_default(),
                row.holding
                    .as_ref()
                    .map(|h| measurement(h.pieces.owned()))
                    .unwrap_or_default(),
                row.holding
                    .as_ref()
                    .map(|h| measurement(h.weight_mg.owned()))
                    .unwrap_or_default(),
            ];
            text.push_str(&values.iter().map(|s| csv(s)).collect::<Vec<_>>().join(","));
            text.push_str("\r\n");
        }
        query.offset = query
            .offset
            .checked_add(u32::try_from(result.rows.len())?)
            .ok_or("CSV row offset exceeds limit")?;
        if u64::from(query.offset) >= result.total_matches {
            break;
        }
    }
    let output = output.as_ref();
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(output.to_owned())
}
