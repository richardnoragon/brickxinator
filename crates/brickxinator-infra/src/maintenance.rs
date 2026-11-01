//! OS-backed maintenance leases. No timeout may bypass a live owner.
use crate::sqlite::Result;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

pub struct Lease {
    _file: File,
}
pub struct Maintenance {
    _file: File,
    pub(crate) database: PathBuf,
}

pub(crate) fn database_path(path: &Path) -> Result<PathBuf> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let normalized = parent
        .canonicalize()?
        .join(path.file_name().ok_or("database filename required")?);
    if fs::symlink_metadata(&normalized).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("database symlinks are not supported".into());
    }
    Ok(normalized)
}
fn lock_file(database: &Path) -> Result<File> {
    let mut name = database.as_os_str().to_os_string();
    name.push(".maintenance.lock");
    let path = PathBuf::from(name);
    if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("maintenance lock cannot be a symlink".into());
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    options.mode(0o600);
    Ok(options.open(path)?)
}
pub(crate) fn shared(database: &Path) -> Result<Lease> {
    let file = lock_file(database)?;
    file.try_lock_shared()
        .map_err(|_| "database is busy with maintenance; retry after it finishes")?;
    Ok(Lease { _file: file })
}
impl Maintenance {
    pub fn acquire(path: impl AsRef<Path>) -> Result<Self> {
        let database = database_path(path.as_ref())?;
        let file = lock_file(&database)?;
        file.try_lock().map_err(
            |_| "database has active readers/writers; retry maintenance when they finish",
        )?;
        Ok(Self {
            _file: file,
            database,
        })
    }
}
