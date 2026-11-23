//! Headless, separately scheduled daily Restic and monthly ZIP jobs.
use crate::{maintenance::Maintenance, snapshots, sqlite::Result};
use brickxinator_core::ledger::Id;
use chrono::{Datelike, Local, Timelike};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Destination {
    pub id: Id,
    #[serde(default)]
    pub kind: DestinationKind,
    pub daily: PathBuf,
    pub monthly: PathBuf,
    /// Exact Linux mount point; checked before any destination directory is created.
    pub required_mount: Option<PathBuf>,
    pub enabled: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DestinationKind {
    #[default]
    Local,
    ExternalDrive,
    NetworkShare,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    pub hour: u32,
    pub minute: u32,
    pub restic: PathBuf,
    pub password_file: PathBuf,
    pub destinations: Vec<Destination>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TargetStatus {
    pub destination: Option<Id>,
    pub daily_period: Option<String>,
    pub monthly_period: Option<String>,
    pub last_daily_success: Option<String>,
    pub last_monthly_success: Option<String>,
    pub daily_error: Option<String>,
    pub monthly_error: Option<String>,
}

pub fn config_path(root: &Path) -> PathBuf {
    root.join("backup-config.json")
}
pub fn load(root: &Path) -> Result<Config> {
    let value: Config = serde_json::from_slice(&fs::read(config_path(root))?)?;
    validate(&value)?;
    validate_paths(root, &value)?;
    Ok(value)
}
fn validate_paths(root: &Path, config: &Config) -> Result<()> {
    let managed_assets = crate::attachments::assets_path(&root.join("database.sqlite"));
    if config
        .destinations
        .iter()
        .any(|d| d.daily.starts_with(&managed_assets) || d.monthly.starts_with(&managed_assets))
    {
        return Err("backup destinations cannot be inside the managed attachment directory".into());
    }
    Ok(())
}
pub fn due_periods(config: &Config, now: chrono::NaiveDateTime) -> Result<(String, String)> {
    let due_today = (now.hour(), now.minute()) >= (config.hour, config.minute);
    let day = if due_today {
        now.date()
    } else {
        now.date().pred_opt().ok_or("invalid schedule date")?
    };
    let month = if now.day() == 1 && !due_today {
        day.format("%Y-%m").to_string()
    } else {
        now.format("%Y-%m").to_string()
    };
    Ok((day.to_string(), month))
}
pub fn status_view(root: &Path, config: &Config) -> Result<Vec<TargetStatus>> {
    let states = status(root)?;
    let (day, month) = due_periods(config, Local::now().naive_local())?;
    Ok(config.destinations.iter().map(|destination|{
        let mut state=states.iter().find(|s|s.destination==Some(destination.id)).cloned().unwrap_or(TargetStatus{destination:Some(destination.id),..Default::default()});
        if destination.enabled {
            if state.daily_period.as_ref().is_none_or(|period|period<&day)&&state.daily_error.is_none() {state.daily_error=Some("Daily backup is due or has never succeeded; run backups and check the OS timer.".into());}
            if state.monthly_period.as_ref().is_none_or(|period|period<&month)&&state.monthly_error.is_none() {state.monthly_error=Some("Monthly ZIP is due or has never succeeded; run backups and check the OS timer.".into());}
        }state
    }).collect())
}
pub fn validate(config: &Config) -> Result<()> {
    if config.version != 1 || config.hour > 23 || config.minute > 59 {
        return Err("invalid backup configuration version or time".into());
    }
    if !config.password_file.is_absolute() {
        return Err("backup password file must be absolute".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for destination in &config.destinations {
        if destination.kind != DestinationKind::Local && destination.required_mount.is_none() {
            return Err(
                "external-drive and network-share destinations require an exact mount point".into(),
            );
        }
        if !ids.insert(destination.id)
            || !destination.daily.is_absolute()
            || !destination.monthly.is_absolute()
            || destination.daily == destination.monthly
            || [&destination.daily, &destination.monthly]
                .iter()
                .any(|path| {
                    path.components()
                        .any(|part| matches!(part, std::path::Component::ParentDir))
                })
        {
            return Err("backup destinations need unique IDs and distinct absolute paths".into());
        }
        if destination
            .required_mount
            .as_ref()
            .is_some_and(|path| !path.is_absolute())
        {
            return Err("required mount must be absolute".into());
        }
    }
    Ok(())
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new_in(path.parent().ok_or("missing parent")?)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    fs::File::open(path.parent().ok_or("missing parent")?)?.sync_all()?;
    Ok(())
}
pub fn save(root: &Path, config: &Config) -> Result<()> {
    validate(config)?;
    validate_paths(root, config)?;
    if config_path(root).exists() {
        let previous = load(root)?;
        let mut states = status(root)?;
        states.retain(|state| {
            config.destinations.iter().any(|new| {
                Some(new.id) == state.destination
                    && previous.destinations.iter().any(|old| {
                        old.id == new.id
                            && old.daily == new.daily
                            && old.monthly == new.monthly
                            && old.required_mount == new.required_mount
                    })
            }) && previous.restic == config.restic
                && previous.password_file == config.password_file
        });
        // Invalidate old success before publishing changed destinations.
        write_json(&root.join("backup-status.json"), &states)?;
    }
    write_json(&config_path(root), config)
}
/// Creates disabled defaults. Scheduling and destination activation remain explicit.
pub fn initialize(root: &Path, working_directory: &Path) -> Result<Config> {
    if config_path(root).exists() {
        return load(root);
    }
    let password_file = root.join("restic-password");
    use std::io::Write;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    if password_file.exists() {
        let metadata = fs::symlink_metadata(&password_file)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > 1024
            || metadata.len() < 32
        {
            return Err(
                "existing Restic password file is not a private generated credential".into(),
            );
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&password_file, fs::Permissions::from_mode(0o600))?;
        }
    } else {
        let mut file = options.open(&password_file)?;
        writeln!(file, "{}{}{}", Id::new(), Id::new(), Id::new())?;
        file.sync_all()?;
    }
    let config = Config {
        version: 1,
        hour: 3,
        minute: 0,
        restic: PathBuf::from("restic"),
        password_file,
        destinations: vec![Destination {
            id: Id::new(),
            kind: DestinationKind::Local,
            daily: working_directory.join("backups/daily"),
            monthly: working_directory.join("backups/montly"),
            required_mount: None,
            enabled: false,
        }],
    };
    save(root, &config)?;
    Ok(config)
}
pub fn status(root: &Path) -> Result<Vec<TargetStatus>> {
    let path = root.join("backup-status.json");
    if !path.exists() {
        return Ok(vec![]);
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn mount_ready(destination: &Destination) -> Result<Option<fs::File>> {
    if let Some(required) = &destination.required_mount {
        // mountinfo escapes spaces, tabs, backslashes and newlines as octal.
        let mounted = || -> Result<bool> {
            Ok(fs::read_to_string("/proc/self/mountinfo")?
                .lines()
                .any(|line| {
                    line.split_whitespace().nth(4).is_some_and(|mount| {
                        let decoded = mount
                            .replace("\\040", " ")
                            .replace("\\011", "\t")
                            .replace("\\012", "\n")
                            .replace("\\134", "\\");
                        Path::new(&decoded) == required
                    })
                }))
        };
        if !mounted()? {
            return Err("required backup mount is unavailable".into());
        }
        if !destination.daily.starts_with(required) || !destination.monthly.starts_with(required) {
            return Err("backup destination is outside its required mount".into());
        }
        // Keep a directory descriptor open for the whole operation so ordinary
        // unmount attempts cannot detach the destination while a ZIP is staged.
        let guard = fs::File::open(required)?;
        if !mounted()? {
            return Err("required backup mount disappeared".into());
        }
        let mount = required.canonicalize()?;
        for destination in [&destination.daily, &destination.monthly] {
            let mut existing = destination.as_path();
            while !existing.exists() {
                existing = existing
                    .parent()
                    .ok_or("backup path has no existing parent")?;
            }
            if !existing.canonicalize()?.starts_with(&mount) {
                return Err("backup destination resolves outside its required mount".into());
            }
        }
        return Ok(Some(guard));
    }
    Ok(None)
}
fn restic(
    config: &Config,
    destination: &Destination,
    repository: &Path,
    arguments: &[&str],
) -> Result<()> {
    // Recheck immediately before every external write, including after staging.
    let _mount = mount_ready(destination)?;
    // File-backed output prevents child pipe deadlock. Never log passwords/output.
    let output = tempfile::tempfile()?;
    let mut child = Command::new(&config.restic)
        .env_remove("RESTIC_PASSWORD")
        .env_remove("RESTIC_PASSWORD_COMMAND")
        .arg("--repo")
        .arg(repository)
        .arg("--password-file")
        .arg(&config.password_file)
        .arg("--no-cache")
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output)
        .spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(());
            }
            return Err(format!(
                "Restic {} failed (exit {:?}); check repository, credentials and storage",
                arguments.first().unwrap_or(&"command"),
                status.code()
            )
            .into());
        }
        if start.elapsed() > Duration::from_secs(3600) {
            child.kill()?;
            child.wait()?;
            return Err("Restic timed out after one hour".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
fn daily(root: &Path, database: &Path, config: &Config, destination: &Destination) -> Result<()> {
    let _mount = mount_ready(destination)?;
    fs::create_dir_all(&destination.daily)?;
    let staged = root.join("backup-staging").join(destination.id.to_string());
    fs::create_dir_all(&staged)?;
    let archive = staged.join("daily.zip");
    // Keep one unprotected failed ZIP per target. Protect that point before
    // replacing it, then take a current snapshot for this schedule period.
    let retry = archive.exists();
    if archive.exists() {
        snapshots::validate(&archive)?;
    } else {
        snapshots::snapshot(database, &archive, "daily", Some(&config_path(root)))?;
    }
    let repository = destination.daily.join("restic");
    if !repository.join("config").exists() {
        restic(config, destination, &repository, &["init"])?;
    }
    let tag = format!("brickxinator-{}", destination.id);
    let path = archive.to_str().ok_or("backup path must be UTF-8")?;
    restic(
        config,
        destination,
        &repository,
        &["backup", "--tag", &tag, "--tag", "daily", path],
    )?;
    if retry {
        restic(config, destination, &repository, &["check"])?;
        fs::remove_file(&archive)?;
        snapshots::snapshot(database, &archive, "daily", Some(&config_path(root)))?;
        restic(
            config,
            destination,
            &repository,
            &["backup", "--tag", &tag, "--tag", "daily", path],
        )?;
    }
    restic(
        config,
        destination,
        &repository,
        &[
            "forget",
            "--tag",
            &tag,
            "--group-by",
            "tags",
            "--keep-last",
            "30",
            "--prune",
        ],
    )?;
    restic(config, destination, &repository, &["check"])?;
    fs::remove_file(archive)?;
    Ok(())
}
fn monthly(root: &Path, database: &Path, destination: &Destination) -> Result<()> {
    let _mount = mount_ready(destination)?;
    fs::create_dir_all(&destination.monthly)?;
    let prefix = format!("brickxinator-monthly-{}-", destination.id);
    let name = format!(
        "{prefix}{}-{}.zip",
        Local::now().format("%Y%m%dT%H%M%S"),
        Id::new()
    );
    snapshots::snapshot(
        database,
        destination.monthly.join(name),
        "monthly",
        Some(&config_path(root)),
    )?;
    let mut archives = Vec::new();
    for item in fs::read_dir(&destination.monthly)? {
        let item = item?;
        let name = item.file_name().to_string_lossy().into_owned();
        if item.file_type()?.is_file() && name.starts_with(&prefix) && name.ends_with(".zip") {
            // Delete only archives validated as our own monthly snapshots.
            if let Ok(checked) = snapshots::validate(item.path())
                && checked.manifest.kind == "monthly"
            {
                archives.push((checked.manifest.created_at, item.path()));
            }
        }
    }
    archives.sort();
    let remove = archives.len().saturating_sub(12);
    for (_, path) in archives.into_iter().take(remove) {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Called by an OS timer. Each due schedule catches up once with a current point.
/// Errors are persistent per target; monthly runs even if daily fails.
pub fn run(root: &Path, database: &Path, force: bool) -> Result<Vec<TargetStatus>> {
    let _lock = Maintenance::acquire(root.join("backup-job"))?;
    let config = load(root)?;
    let (daily_period, month) = due_periods(&config, Local::now().naive_local())?;
    let mut statuses = status(root)?;
    let mut failed = false;
    for destination in config.destinations.iter().filter(|d| d.enabled) {
        let index = statuses
            .iter()
            .position(|s| s.destination == Some(destination.id))
            .unwrap_or_else(|| {
                statuses.push(TargetStatus {
                    destination: Some(destination.id),
                    ..Default::default()
                });
                statuses.len() - 1
            });
        let state = &mut statuses[index];
        if force || state.daily_period.as_ref() != Some(&daily_period) {
            match daily(root, database, &config, destination) {
                Ok(()) => {
                    state.daily_period = Some(daily_period.clone());
                    state.last_daily_success = Some(Local::now().to_rfc3339());
                    state.daily_error = None;
                }
                Err(error) => {
                    state.daily_error = Some(error.to_string());
                    failed = true;
                }
            }
        }
        if force || state.monthly_period.as_ref() != Some(&month) {
            match monthly(root, database, destination) {
                Ok(()) => {
                    state.monthly_period = Some(month.clone());
                    state.last_monthly_success = Some(Local::now().to_rfc3339());
                    state.monthly_error = None;
                }
                Err(error) => {
                    state.monthly_error = Some(error.to_string());
                    failed = true;
                }
            }
        }
        write_json(&root.join("backup-status.json"), &statuses)?;
    }
    if failed {
        return Err("one or more backup jobs failed; inspect backup status".into());
    }
    Ok(statuses)
}
