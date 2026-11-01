# Source installation (alpha candidate)

This is an alpha review candidate. See ALPHA_STATUS.md for remaining
acceptance and release gates. The supported target is Kubuntu 26.04 with Firefox
or Chromium. No release package or alpha tag has been published.

Install Rust (MSRV 1.89), Node.js compatible with Vite 8 (22.12+ or 24+), npm,
a C compiler, Linux build tools, and Restic. Dependencies are pinned in
Cargo.lock and frontend/package-lock.json. The source-build steps are (use the `alpha-foundation` review branch until it is
merged):

```sh
git clone https://github.com/richardnoragon/brickxinator.git
cd brickxinator
git switch alpha-foundation
```

Then build and launch:

```sh
cd frontend
npm ci --ignore-scripts
npm run check
npm run build
cd ..
cargo test --workspace --locked
cargo build --release --locked
bash scripts/brickxinator
```

The launcher opens the loopback-only service in your default browser. It uses
`$XDG_DATA_HOME/brickxinator` or `~/.local/share/brickxinator`; pass `--data-dir`
to choose another local directory. The application exits after 90 idle seconds
once tabs disconnect. A crashed tab expires after 30 seconds. Keep the source
checkout and its built frontend assets together. Do not serve the database
over a network share or run multiple application instances on the same data.

## Backups

On first launch, disabled backup settings are created. In Recovery, load the
settings, select the Restic executable, and review/enable your destinations.
Defaults are `<initial working directory>/backups/daily` and
`<initial working directory>/backups/montly` (the requested spelling). Add and edit multiple destinations in Recovery. External-drive/NAS destinations need an exact required mount point.
Missing required mounts fail before destination directories are created. Mounts
are checked again before Restic commands; an open directory handle is retained
while a destination is being processed.

The Restic repository is `<daily folder>/restic`. A private random password is
stored at `<data directory>/restic-password`. **Keep an independent, secure copy
of that password file**: snapshots include configuration references, never the
password itself. Losing the password prevents Restic recovery. This is a local
Restic repository adapter; remote URL repositories are not implemented.

Daily success requires a validated full application ZIP, successful Restic
backup, retention/prune, and repository check. One failed, unprotected ZIP per
destination is retained for retry. A later success first protects that point,
then creates a current point. Thirty daily external points are retained.
Monthly full ZIPs are independent, retain twelve per destination, and can be
restored without Restic. Daily failures do not prevent monthly attempts.

To install and activate the user timer, explicitly run:

```sh
bash scripts/install-backup-timer
# Or choose a data directory:
bash scripts/install-backup-timer /absolute/local/data-directory
systemctl --user list-timers brickxinator-backup.timer
```

The timer checks every five minutes and runs while logged in, independently of
the browser. The configured local time defaults to 03:00. Missed daily/monthly
schedules each create one current catch-up point; first-of-month monthly work
never replaces daily work. No timer has been installed by source development.
Running while logged out requires separately enabling user lingering or a system
service; this installer does not change login policy. Failures are shown in
Recovery and cause `backup-run` to exit unsuccessfully. Inspect service logs with
`journalctl --user -u brickxinator-backup.service`.

```sh
target/release/brickxinator-app backup-status --data-dir /path/to/data
target/release/brickxinator-app backup-run --data-dir /path/to/data --force
```

To recover a daily point, use Restic `restore` with the repository and password
file, find the restored `daily.zip`, then use Brickxinator's Restore interface
or CLI. Restored configuration is placed separately for review, never activated.
Restore creates a validated safety ZIP first. To undo, restore that safety ZIP.
If an interrupted restore is reported, run `recover --data-dir ...` before opening
the application. `doctor` validates the database, journal and attachments.

## Verification

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
BRICKXINATOR_TEST_RESTIC=/absolute/path/to/restic cargo test -p brickxinator-infra --test backups -- --ignored
```

The real Restic drill uses disposable directories, runs 31 backups, checks 30/12
retention and restores every retained daily and monthly point. It takes about
107 seconds here. The launcher process tests are separately opt-in:

```sh
cargo test -p brickxinator-app --test launcher -- --ignored
cd frontend
npm run format:check
npm run check
npm run build
npx playwright install chromium firefox
npm run test:browser
```

Browser tests start their own disposable application on 127.0.0.1:43179. They
create synthetic purchases. If overriding `BRICKXINATOR_TEST_URL`, use only a
**disposable** instance, never your collection. Chromium and Firefox are configured.
Native desktop clipboard/default-browser checks remain part of manual review.

## Recovery and exchange commands

```sh
target/release/brickxinator-app doctor --data-dir /path/to/data
target/release/brickxinator-app snapshot --data-dir /path/to/data --output /path/to/snapshot.zip
target/release/brickxinator-app restore --data-dir /path/to/data --input /path/to/snapshot.zip
target/release/brickxinator-app recover --data-dir /path/to/data
target/release/brickxinator-app export-native --data-dir /path/to/data --output /path/to/native-directory
target/release/brickxinator-app export-csv --data-dir /path/to/data --output /path/to/inventory.csv
```

Maintenance and backup commands refuse a missing database. For a new recovery or
native-import target, explicitly run `init --data-dir /path/to/empty-directory`
first. `import-native --data-dir ... --input /path/to/native-directory` initializes
an empty collection or accepts an identical re-import; it does not merge existing
collections. Browser exchange uses a ZIP containing the same native JSON/assets.
CSV is a current-inventory report, not a round-trip format.

Startup also refuses a nonempty data directory whose database is missing. If a
restore journal exists, use `recover` before attempting any reinitialization.
Tested build environment and dependency review are recorded in ALPHA_STATUS.md
and THIRD_PARTY.md. No binary installer or desktop package has been published.
