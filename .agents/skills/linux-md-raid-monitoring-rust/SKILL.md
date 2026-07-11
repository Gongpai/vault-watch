---
name: linux-md-raid-monitoring-rust
description: Design, implement, review, or migrate native Linux MD software RAID monitoring in Rust using /sys/block/*/md attributes without a runtime mdadm dependency. Use for sysfs array/member discovery, degraded-state and rebuild/resync/check/repair/reshape tracking, progress and ETA metrics, poll/udev event handling, race-safe snapshots, external IMSM/DDF metadata awareness, fixture tests, or replacing a /proc/mdstat parser.
---

# Native Linux MD RAID Monitoring in Rust

Build a read-only Linux MD monitoring backend around documented sysfs attributes. Treat `/proc/mdstat` as a compatibility or validation source rather than the primary machine interface.

## Read the reference selectively

Read [references/linux-md-raid-monitoring-research.md](references/linux-md-raid-monitoring-research.md) before changing architecture, field mappings, state handling, or event logic. It contains attribute tables, formulas, Rust sketches, migration guidance, kernel-version caveats, and primary-source links.

For narrow tasks, locate sections with these searches:

- Attribute schema: `ตาราง Sysfs Attributes`, `Array-Level`, or `Member Device`.
- Snapshot implementation: `Snapshot Collection Algorithm`, `Rust Data Model`, or `Sysfs Native Backend`.
- Progress and ETA: `Rebuild/Progress/ETA` or `sync_completed`.
- Events: `Event-Monitoring Design`, `poll`, or `udev`.
- Correctness hazards: `Race-Condition`, `External Metadata`, or `Known Limitations`.
- Migration and tests: `Test Matrix` or `Migration Plan`.

Treat the report as research input. Verify disputed, kernel-version-specific, or release-critical claims against current Linux kernel MD documentation and source linked in section M.

## Select data sources

Use this priority:

1. Read `/sys/class/block/<device>/md/` or `/sys/block/<device>/md/` for array and member state.
2. Use pollable MD sysfs attributes and udev/netlink only to reduce reaction latency.
3. Reconcile with periodic full snapshots even when events are enabled.
4. Use `/proc/mdstat` only as a temporary fallback, migration oracle, or source for behavior not confirmed in sysfs.
5. Use `mdadm` only for test setup and comparison, never as the runtime collector.

Avoid MD ioctls unless a required datum is unavailable through documented read-only sysfs ABI.

## Separate the implementation

Keep these concerns independent:

```text
ArrayEnumerator -> SysfsReader -> SnapshotAssembler -> StateDiffer -> Metrics/Events
```

- Inject a sysfs root or reader trait for fixture tests.
- Model unknown enum values as `Unknown(String)` for forward compatibility.
- Preserve unavailable values as `Option<T>` or explicit field errors.
- Separate a raw snapshot from derived health, progress, ETA, alerts, and UI formatting.
- Never write MD sysfs attributes from a monitoring backend.

## Enumerate arrays and members

1. Enumerate block-class entries and identify arrays by the existence of an `md/` directory or documented subsystem metadata; do not rely only on `md0`-style names.
2. Read `array_state` early. Keep inactive or assembling arrays visible if product requirements need diagnostic state rather than silently dropping them.
3. Read array attributes such as `level`, `raid_disks`, `metadata_version`, `degraded`, `sync_action`, `sync_completed`, `sync_speed`, `mismatch_cnt`, `reshape_position`, and `consistency_policy` when present.
4. Enumerate `md/dev-*` member directories and read `state`, `slot`, `errors`, and `recovery_start` independently.
5. Resolve member block symlinks rather than deriving a device name by stripping `dev-`; kernel names and escaping can be surprising.
6. Sort presentation by slot, but retain spare, replacement, faulty, removed, and unslotted members.

Do not infer health from member count alone. Combine `degraded`, array state, expected slots, and member flags while retaining the raw evidence.

## Assemble race-aware snapshots

Sysfs exposes separate files, not an atomic multi-attribute snapshot:

1. Read a generation boundary such as `array_state` and `sync_action` before dependent fields.
2. Read required identity/topology fields and then optional fields.
3. Read `sync_completed` only in the context of the observed `sync_action`.
4. Enumerate members and collect each member independently.
5. Re-read `array_state` and `sync_action` after collection.
6. If a boundary changed incompatibly, retry a bounded number of times or return a marked inconsistent snapshot.
7. Treat `ENOENT` during traversal as normal disappearance or transition, not a panic.

Never use `unwrap_or_default()` to turn a missing or malformed safety-critical value into `0`, `idle`, or `healthy`. Distinguish unavailable, malformed, transient, and genuine zero.

## Parse states defensively

- Parse enum strings with an `Unknown(String)` variant.
- Parse member `state` as a set of comma-separated flags; preserve unknown flags if diagnostics matter.
- Accept special values such as `none`, empty attributes during assembly, and optional `(local)` or `(system)` suffixes only where documented.
- Do not equate `sync_action=idle` with a completed operation unless corroborated by state and prior snapshot.
- Treat `check` as verification, `repair` as corrective verification, `resync` as redundancy synchronization, and `recover` as member recovery; do not collapse them all into “rebuild”.
- Represent reshape independently from synchronization because topology and progress semantics can differ.

## Derive progress and ETA

For a valid `sync_completed` value `N / M`:

```text
percent = N / M * 100
remaining_bytes = (M - N) * 512
kernel ETA seconds = remaining_bytes / (sync_speed_kib_s * 1024)
delta speed KiB/s = delta(completed_sectors) * 512 / 1024 / elapsed_seconds
```

Use checked or saturating subtraction after validating `N <= M`. Linux MD sector counts use 512-byte sectors for these calculations. Return no ETA when speed is zero, state changed, progress regressed, total changed, or the sample pair belongs to different operations.

Prefer a smoothed delta speed for responsive UI while retaining kernel `sync_speed` as a separately labelled metric. Reset history when `sync_action`, total sectors, reshape topology, or array identity changes.

## Handle events without trusting them for correctness

- Use poll/select only on attributes documented as pollable for the supported kernels.
- Perform an initial read and follow correct sysfs poll semantics; after notification, seek/re-read or reopen as required by the documented attribute behavior and verified tests. Do not encode an unverified universal close/reopen rule.
- Use udev/netlink for array and member add/remove/change hints.
- Funnel all event hints into a fresh snapshot; do not mutate cached health solely from event payloads.
- Maintain periodic reconciliation to recover from missed, coalesced, or watcher-registration races.
- Run blocking poll loops on dedicated threads or bounded blocking tasks and provide explicit shutdown/cancellation.

## Treat external metadata carefully

If `metadata_version` begins with `external:`:

- Flag the array/container explicitly.
- Remain read-only and do not interfere with `mdmon`.
- Expect container/member-array relationships and transient `inactive` or `write-pending` states.
- Build hierarchy from sysfs relationships rather than assuming every MD device is an independently usable array.
- Report unsupported or ambiguous topology instead of declaring it healthy.

Also account for nested MD arrays and partition-backed members without flattening identities.

## Test before migration

Build fixture trees for at least:

- healthy, inactive, assembling, degraded, and read-only arrays;
- RAID0/1/5/6/10 plus unknown future levels;
- recover, resync, check, repair, reshape, and idle states;
- valid, `none`, empty, malformed, regressing, and changing-total progress;
- faulty, spare, replacement, write-mostly, blocked, and vanished members;
- multiple arrays, nested arrays, named arrays, IMSM/DDF metadata, bitmap, PPL, and missing optional attributes;
- array/member disappearance mid-snapshot and state changes across boundary reads;
- exact progress, speed, and ETA calculations including zero-speed and overflow boundaries.

For migration from `/proc/mdstat`, run both collectors against the same snapshot window in debug/integration tests, compare semantically equivalent fields, then make sysfs primary. Do not require textual equality between sources.

## Review checklist

Before completing work, confirm:

- Runtime operation does not invoke `mdadm` or shell commands.
- Monitoring code never writes to MD sysfs.
- Required parse failures cannot become false healthy values.
- Unknown states and optional kernel attributes remain representable.
- Array removal and member TOCTOU races do not panic.
- Progress history resets across operation or topology changes.
- External metadata arrays are identified and handled read-only.
- Event hints always lead to snapshot reconciliation.
- Fixture-root injection supports unit tests without root or MD hardware.
- Kernel ABI assumptions are linked or documented near parsers.
- `cargo fmt`, targeted tests, full tests, and clippy pass.
