use crate::sqlite::{Result, Store};
use brickxinator_core::ledger::Id;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Write,
    path::{Component, Path, PathBuf},
};

pub const MAX_BYTES: usize = 64 * 1024 * 1024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    Purchase(Id),
    InvoiceLine(Id),
    Definition(Id),
    SetCopy(Id),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    pub id: Id,
    pub request_key: String,
    pub actor: String,
    pub target: Target,
    pub original_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Id,
    pub relative_path: String,
    pub original_name: String,
    pub sha256: String,
    pub byte_size: u64,
    pub media_type: String,
    pub target: Target,
}
pub fn assets_path(database: &Path) -> PathBuf {
    let mut path = database.as_os_str().to_os_string();
    path.push(".assets");
    PathBuf::from(path)
}
pub(crate) fn safe_join(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    if path
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
        || relative.contains('\\')
        || relative.is_empty()
    {
        return Err("invalid managed relative path".into());
    }
    let full = root.join(path);
    let mut current = root.to_path_buf();
    if fs::symlink_metadata(root).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("managed assets root cannot be a symlink".into());
    }
    for component in path.components() {
        current.push(component);
        if fs::symlink_metadata(&current).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("managed paths cannot contain symlinks".into());
        }
    }
    Ok(full)
}
pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn media(bytes: &[u8]) -> Result<(&'static str, &'static str)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok(("image/png", "png"))
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Ok(("image/jpeg", "jpg"))
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Ok(("image/gif", "gif"))
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Ok(("image/webp", "webp"))
    } else if bytes.starts_with(b"%PDF-") {
        Ok(("application/pdf", "pdf"))
    } else {
        Err("only PNG, JPEG, GIF, WebP and PDF attachments are supported".into())
    }
}
impl Store {
    pub fn attach(&mut self, command: Command, bytes: &[u8]) -> Result<Attachment> {
        if bytes.is_empty()
            || bytes.len() > MAX_BYTES
            || command.actor.trim().is_empty()
            || command.request_key.trim().is_empty()
        {
            return Err("attachment needs bytes within 64 MiB, actor and request identity".into());
        }
        if command.original_name.chars().count() > 255
            || command.original_name.chars().any(char::is_control)
        {
            return Err("invalid attachment display name".into());
        }
        let (media_type, extension) = media(bytes)?;
        let digest = hash(bytes);
        let payload = serde_json::to_string(&(&command, &digest, bytes.len()))?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior: Option<(String, String)> = tx
            .query_row(
                "SELECT payload_json,result_json FROM attachment_command WHERE request_key=?1",
                [&command.request_key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((old, result)) = prior {
            if old != payload {
                return Err("attachment request key conflicts with existing payload".into());
            }
            return Ok(serde_json::from_str(&result)?);
        }
        for table in [
            "inventory_operation",
            "purchase_command",
            "settings_command",
            "catalog_command",
            "profile_command",
            "shipping_command",
        ] {
            if tx.query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE request_key=?1)"),
                [&command.request_key],
                |r| r.get::<_, bool>(0),
            )? {
                return Err("request key belongs to another command".into());
            }
        }
        let (purchase, line, definition, copy) = match command.target {
            Target::Purchase(id) => (Some(id.to_string()), None, None, None),
            Target::InvoiceLine(id) => (None, Some(id.to_string()), None, None),
            Target::Definition(id) => (None, None, Some(id.to_string()), None),
            Target::SetCopy(id) => (None, None, None, Some(id.to_string())),
        };
        let root = assets_path(&self.database);
        fs::create_dir_all(&root)?;
        let relative_path = format!("{}.{}", command.id, extension);
        let destination = safe_join(&root, &relative_path)?;
        let mut staged = tempfile::NamedTempFile::new_in(&root)?;
        staged.write_all(bytes)?;
        staged.as_file().sync_all()?;
        match staged.persist_noclobber(&destination) {
            Ok(_) => {}
            Err(error) => {
                if !destination.is_file() || hash(&fs::read(&destination)?) != digest {
                    return Err(Box::new(error));
                }
            }
        }
        File::open(&root)?.sync_all()?;
        let attachment = Attachment {
            id: command.id,
            relative_path,
            original_name: command.original_name.clone(),
            sha256: digest,
            byte_size: bytes.len() as u64,
            media_type: media_type.into(),
            target: command.target.clone(),
        };
        tx.execute("INSERT INTO attachment(id,relative_path,original_name,sha256,byte_size,media_type) VALUES(?1,?2,?3,?4,?5,?6)",params![attachment.id.to_string(),attachment.relative_path,attachment.original_name,attachment.sha256,i64::try_from(attachment.byte_size)?,attachment.media_type])?;
        tx.execute("INSERT INTO attachment_association(id,attachment_id,purchase_id,invoice_line_id,definition_id,set_copy_id) VALUES(?1,?2,?3,?4,?5,?6)",params![Id::new().to_string(),attachment.id.to_string(),purchase,line,definition,copy])?;
        tx.execute("INSERT INTO attachment_command(request_key,attachment_id,actor,payload_json,result_json) VALUES(?1,?2,?3,?4,?5)",params![command.request_key,attachment.id.to_string(),command.actor,payload,serde_json::to_string(&attachment)?])?;
        tx.commit()?;
        Ok(attachment)
    }
    pub fn attachments(&self) -> Result<Vec<Attachment>> {
        if !crate::sqlite::has_attachments(&self.connection)? {
            return Err("attachments require schema 9 or later".into());
        }
        let mut query = self
            .connection
            .prepare("SELECT result_json FROM attachment_command ORDER BY rowid")?;
        let records: Vec<Attachment> = query
            .query_map([], |r| r.get::<_, String>(0))?
            .map(|row| Ok(serde_json::from_str(&row?)?))
            .collect::<Result<_>>()?;
        let count: i64 = self
            .connection
            .query_row("SELECT count(*) FROM attachment", [], |r| r.get(0))?;
        let associations: i64 =
            self.connection
                .query_row("SELECT count(*) FROM attachment_association", [], |r| {
                    r.get(0)
                })?;
        if count != records.len() as i64 || associations != count {
            return Err("attachment audit coverage mismatch".into());
        }
        for record in &records {
            let metadata: (String,String,String,i64,String) = self.connection.query_row("SELECT relative_path,original_name,sha256,byte_size,media_type FROM attachment WHERE id=?1", [record.id.to_string()], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
            if metadata
                != (
                    record.relative_path.clone(),
                    record.original_name.clone(),
                    record.sha256.clone(),
                    i64::try_from(record.byte_size)?,
                    record.media_type.clone(),
                )
            {
                return Err("attachment metadata differs from audit".into());
            }
            let target: (Option<String>,Option<String>,Option<String>,Option<String>) = self.connection.query_row("SELECT purchase_id,invoice_line_id,definition_id,set_copy_id FROM attachment_association WHERE attachment_id=?1", [record.id.to_string()], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
            let expected = match record.target {
                Target::Purchase(id) => (Some(id.to_string()), None, None, None),
                Target::InvoiceLine(id) => (None, Some(id.to_string()), None, None),
                Target::Definition(id) => (None, None, Some(id.to_string()), None),
                Target::SetCopy(id) => (None, None, None, Some(id.to_string())),
            };
            if target != expected {
                return Err("attachment association differs from audit".into());
            }
        }
        Ok(records)
    }
    pub fn attachment_bytes(&self, id: Id) -> Result<(Attachment, Vec<u8>)> {
        let record = self
            .attachments()?
            .into_iter()
            .find(|r| r.id == id)
            .ok_or("attachment does not exist")?;
        let bytes = fs::read(safe_join(
            &assets_path(&self.database),
            &record.relative_path,
        )?)?;
        if bytes.len() as u64 != record.byte_size || hash(&bytes) != record.sha256 {
            return Err("attachment integrity validation failed".into());
        }
        let (mime, extension) = media(&bytes)?;
        if record.media_type != mime
            || record.relative_path != format!("{}.{}", record.id, extension)
        {
            return Err("attachment type or generated filename differs from bytes".into());
        }
        Ok((record, bytes))
    }
}
