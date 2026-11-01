use brickxinator_app::{AppState, router};
use brickxinator_infra::{
    backups, exchange, recovery, snapshots,
    sqlite::{Result, Store},
};
use std::path::PathBuf;
fn option(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}
fn data_root(args: &[String]) -> PathBuf {
    option(args, "--data-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
                })
                .join("brickxinator")
        })
}
fn existing_server(root: &std::path::Path) -> Result<String> {
    use std::io::{Read, Write};
    let runtime: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("runtime.json"))?)?;
    let url = runtime["url"]
        .as_str()
        .ok_or("existing service URL is missing")?;
    let port: u16 = url
        .strip_prefix("http://127.0.0.1:")
        .ok_or("existing service is not loopback")?
        .parse()?;
    let address = std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, port));
    let mut connection =
        std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_secs(1))?;
    connection.set_read_timeout(Some(std::time::Duration::from_secs(1)))?;
    connection.set_write_timeout(Some(std::time::Duration::from_secs(1)))?;
    write!(
        connection,
        "GET /api/v1/health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )?;
    let mut response = String::new();
    connection.take(8192).read_to_string(&mut response)?;
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or("invalid existing service response")?;
    if !headers.starts_with("HTTP/1.1 200 ") {
        return Err("existing local service health check failed".into());
    }
    let health: serde_json::Value = serde_json::from_str(body)?;
    if health["application"] != "brickxinator"
        || runtime["instance"].as_str().is_none()
        || health["instance"] != runtime["instance"]
    {
        return Err("existing service does not match this data directory".into());
    }
    Ok(url.to_owned())
}
#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Brickxinator: {error}");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("help");
    if command == "help" || command == "--help" {
        println!(
            "brickxinator-app <serve|init|doctor|upgrade|snapshot|restore|recover|backup-init|backup-run|backup-status|export-native|import-native|export-csv> --data-dir DIR\nserve: --port PORT --assets frontend/dist --open --idle-seconds 90\nsnapshot: --output FILE.zip\nrestore: --input FILE.zip\nexport-native: --output DIRECTORY\nimport-native: --input DIRECTORY (init a fresh target first)\nexport-csv: --output FILE.csv\nDefault data root: XDG_DATA_HOME/brickxinator or ~/.local/share/brickxinator"
        );
        return Ok(());
    }
    if ![
        "init",
        "doctor",
        "upgrade",
        "snapshot",
        "restore",
        "recover",
        "serve",
        "backup-init",
        "backup-run",
        "backup-status",
        "export-native",
        "import-native",
        "export-csv",
    ]
    .contains(&command)
    {
        return Err("unknown command; use --help".into());
    }
    let root = data_root(&args);
    let missing_database = !root.join("database.sqlite").exists();
    let interrupted_restore = root.join("database.sqlite.restore.json").exists();
    if missing_database
        && !matches!(command, "init" | "serve" | "backup-init")
        && !(command == "recover" && interrupted_restore)
    {
        return Err("database does not exist; backup/export/maintenance will not initialize an empty collection. Use init explicitly for a new collection, or recover/restore your existing data".into());
    }
    if missing_database
        && matches!(command, "serve" | "backup-init")
        && root.exists()
        && std::fs::read_dir(&root)?.next().is_some()
    {
        return Err("nonempty data directory has no database; automatic initialization is refused. Recover an interrupted restore or restore a backup; use init explicitly only for a new collection".into());
    }
    std::fs::create_dir_all(&root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))?;
    }
    let root = root.canonicalize()?;
    let database = root.join("database.sqlite");
    let safety = root.join("backups/safety");
    if command == "recover" {
        println!(
            "Recovered interrupted restore: {}",
            recovery::recover_interrupted(&database)?
        );
        return Ok(());
    }
    if !database.exists() {
        drop(Store::create(&database)?);
    }
    match command {
        "export-native" => {
            let output = option(&args, "--output").ok_or("--output directory is required")?;
            exchange::export(&database, output)?;
            println!("Native JSON and asset bundle exported");
        }
        "import-native" => {
            let input = option(&args, "--input").ok_or("--input directory is required")?;
            println!(
                "Collection imported: {}",
                exchange::import(&database, input, safety)?
            );
        }
        "export-csv" => {
            let output = option(&args, "--output").ok_or("--output CSV file is required")?;
            exchange::inventory_csv(&database, output)?;
            println!("Inventory CSV exported");
        }
        "backup-init" => {
            backups::initialize(&root, &std::env::current_dir()?)?;
            println!(
                "Created disabled backup settings at {}",
                backups::config_path(&root).display()
            );
        }
        "backup-run" => {
            backups::run(&root, &database, args.iter().any(|arg| arg == "--force"))?;
            println!("Backup jobs completed");
        }
        "backup-status" => println!(
            "{}",
            serde_json::to_string_pretty(&backups::status(&root)?)?
        ),
        "init" => {
            drop(Store::open(&database)?);
            println!("Database available at {}", database.display());
        }
        "doctor" => {
            let store = Store::open(&database)?;
            for record in store.attachments()? {
                store.attachment_bytes(record.id)?;
            }
            println!("Database, journal, metadata and attachment integrity: OK");
        }
        "upgrade" => println!(
            "{}",
            serde_json::to_string_pretty(&recovery::upgrade(&database, safety)?)?
        ),
        "snapshot" => {
            let output = option(&args, "--output").ok_or("--output is required")?;
            println!(
                "{}",
                serde_json::to_string_pretty(&snapshots::snapshot(
                    &database,
                    output,
                    "manual",
                    backups::config_path(&root)
                        .exists()
                        .then_some(backups::config_path(&root).as_path())
                )?)?
            );
        }
        "restore" => {
            let input = option(&args, "--input").ok_or("--input is required")?;
            println!(
                "{}",
                serde_json::to_string_pretty(&recovery::restore(&database, input, safety)?)?
            );
        }
        "serve" => {
            let _instance = match brickxinator_infra::maintenance::Maintenance::acquire(
                root.join("server-instance"),
            ) {
                Ok(guard) => guard,
                Err(_) => {
                    let url = existing_server(&root)?;
                    println!("Brickxinator already running at {url}");
                    if args.iter().any(|arg| arg == "--open") {
                        std::process::Command::new("xdg-open").arg(&url).spawn()?;
                    }
                    return Ok(());
                }
            };
            recovery::upgrade(&database, &safety)?;
            if let Err(error) = backups::initialize(&root, &std::env::current_dir()?) {
                eprintln!("Backup settings unavailable; inventory remains usable: {error}");
            }
            let port: u16 = option(&args, "--port").unwrap_or("4217".into()).parse()?;
            let listener =
                tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
            let origin = format!("http://{}", listener.local_addr()?);
            let assets = option(&args, "--assets")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../frontend/dist")
                });
            if !assets.join("index.html").exists() {
                return Err(
                    "frontend assets missing; run npm ci and npm run build in frontend".into(),
                );
            }
            let state = AppState::new(
                database,
                assets,
                origin.clone(),
                option(&args, "--idle-seconds")
                    .unwrap_or("90".into())
                    .parse()?,
            );
            println!("Brickxinator listening at {origin}");
            std::fs::write(
                root.join("runtime.json"),
                serde_json::to_vec(
                    &serde_json::json!({"url":origin,"pid":std::process::id(),"instance":state.instance}),
                )?,
            )?;
            if args.iter().any(|arg| arg == "--open") {
                std::process::Command::new("xdg-open")
                    .arg(&origin)
                    .spawn()?;
            }
            let idle = state.clone();
            axum::serve(listener,router(state)).with_graceful_shutdown(async move {loop {tokio::select! {_=tokio::signal::ctrl_c()=>break,_=tokio::time::sleep(std::time::Duration::from_secs(1))=>{if idle.should_stop(){break;}}}}}).await?;
            let _ = std::fs::remove_file(root.join("runtime.json"));
        }
        _ => return Err("unknown command; use --help".into()),
    }
    Ok(())
}
