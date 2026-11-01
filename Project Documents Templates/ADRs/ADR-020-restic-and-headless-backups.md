# ADR-020: Restic, independent schedules and destination safety

Date: 2026-11-01. Status: implemented. Restic and default paths were explicitly
selected by Richard; scheduling mechanics are engineering decisions.

## Decision

Each daily job creates and validates a full application ZIP, then invokes external
Restic with argv rather than a shell. ZIP-only success is not daily success.
The repository is `<daily destination>/restic`, scoped by a stable destination
UUID tag plus daily tag. Retention uses that tag and tag grouping, retains thirty
points, prunes, and checks the repository before recording success. A failed,
unprotected ZIP is retained per destination; retry protects it before generating
a current point. Child processes have file-backed output and a one-hour timeout.

Monthly ZIPs run independently, without Restic, and retain twelve validated
snapshots under the destination's own UUID filename prefix. Daily failure never
skips the monthly attempt or another configured destination. Monthly work never
substitutes for daily work. Untouched user files are not retention candidates.

Configuration starts disabled at 03:00 machine-local time, with the requested
working-directory defaults `backups/daily` and `backups/montly`. The UI edits
multiple destinations. External-drive/network-share kinds require an exact mount
point. Mountinfo is checked before creating destination directories; resolved
existing parents must remain inside that mount. A directory descriptor is held
through destination processing, and every Restic invocation rechecks mountinfo
immediately before spawning, including after ZIP staging. Parent traversal and destinations
inside managed attachments are rejected. Changed paths invalidate prior success
status for that configuration. Missing/failed destinations do not block inventory.

A private generated password file is outside the snapshots; only its reference
is in configuration. The operator must preserve an independent copy of that
credential for Restic recovery. Monthly ZIP recovery requires no such credential.
Restored machine configuration stays inactive until reviewed.

An opt-in systemd user timer checks every five minutes, independently of the UI.
The coordinator computes the latest due daily date and monthly period, coalescing
missed work to one current point per schedule. Before the first day's configured
time, the preceding month remains the due monthly period. Separate success/error
fields are persistent per destination; status views also identify never-completed
or overdue work. A single OS job lock prevents overlapping jobs/config edits.
The user timer runs while logged in; lingering/system-level operation is not
enabled by the installer.

## Verification

Restic 0.19.1 completed thirty-one disposable runs with changing stock and a PDF,
retained thirty daily points/twelve monthly ZIPs, and restored every retained
point. All restored databases, manifests and referenced files validated. Monthly
ZIPs were installed through the recovery coordinator without invoking Restic.
Schedule-boundary, missing-mount, independent-monthly, multiple-target and visible
failure cases pass. Generated systemd units passed systemd-analyze validation with
temporary configuration and a fake systemctl; no user timer was activated.

The small measured run had 572,273 retained compressed-ZIP input bytes versus
606,883 repository file bytes. Full precompressed ZIPs can reduce deduplication
opportunities and introduce overhead. This measurement does not promise storage
savings or predict large-collection behavior. See the primary
[Restic retention](https://restic.readthedocs.io/en/stable/060_forget.html) and
[restore](https://restic.readthedocs.io/en/stable/050_restore.html) documentation.
