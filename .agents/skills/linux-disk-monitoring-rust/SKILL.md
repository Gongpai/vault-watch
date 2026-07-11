---
name: linux-disk-monitoring-rust
description: Design, implement, review, or migrate native Linux block-device discovery and disk I/O monitoring in Rust using sysfs and procfs without runtime calls to iostat, lsblk, sysstat, or shell commands. Use for /proc/diskstats or /sys/class/block parsing, physical-device classification, throughput/IOPS/utilization/latency/queue metrics, hot-plug and counter-reset handling, fixture-based tests, or replacing an external iostat collector.
---

# Native Linux Disk Monitoring in Rust

Build a Linux-only monitoring backend from documented kernel interfaces. Prefer stable ABI, isolate driver-dependent metadata, and keep collection testable without real hardware.

## Read the reference selectively

Read [references/kernel-disk-monitoring-research.md](references/kernel-disk-monitoring-research.md) before making architecture or implementation decisions. It contains source links, field tables, formulas, Rust sketches, kernel-version notes, and test fixtures.

For a narrow task, locate the relevant section first:

- Discovery or metadata: search for `Device Discovery`, `Device Metadata`, `rotational`, `slaves`, or `holders`.
- Statistics parsing: search for `diskstats`, `RawStat`, or `Different Kernel Versions`.
- Metric formulas: search for `สูตรคำนวณ Metrics`.
- Failure handling: search for `Edge Cases`, `Counter Wraparound`, `diskseq`, or `Hot-plug`.
- Implementation planning: search for `Rust Code Skeleton`, `Testing Strategy`, or `Implementation Roadmap`.

Treat the report as research input, not unquestionable specification. Verify disputed or release-critical claims against the primary kernel documentation linked in section K.

## Choose the kernel interface

1. Use `/proc/diskstats` for one batch snapshot covering many devices.
2. Use `/sys/class/block/<dev>/stat` or `/sys/block/<dev>/stat` for a device-scoped snapshot.
3. Enumerate `/sys/class/block`; do not parse `lsblk` output.
4. Read queue and identity metadata from sysfs, accepting missing files as normal.
5. Add udev/netlink only when event-driven hot-plug is required. Keep periodic reconciliation as the correctness path.

Do not use external commands as the production backend. They may be used as development oracles for comparison tests.

## Implement in separable layers

Keep these concerns independent:

```text
DeviceEnumerator -> DeviceClassifier -> StatSource -> DeltaCalculator -> Metrics
```

- Inject a filesystem root or source trait so tests can use fixtures.
- Represent optional kernel fields with `Option<u64>`.
- Store raw counters and a monotonic timestamp in each sample.
- Key previous samples by a stable identity when available; device names alone can be reused after hot-plug.
- Keep UI formatting, history buffers, and alert thresholds outside the collector.

## Discover and classify devices

Follow this order:

1. Enumerate entries under `/sys/class/block`.
2. Read `uevent` and reject `DEVTYPE=partition` when only whole devices are wanted.
3. Classify known virtual or stacked devices such as `loop*`, `ram*`, `zram*`, `dm-*`, `md*`, and `nbd*` according to product scope.
4. Inspect `slaves/`, `holders/`, and optional `hidden`; do not assume a non-empty `holders/` makes the underlying physical disk virtual.
5. Read `queue/rotational`, block sizes, model, vendor, revision, and NVMe controller metadata as optional values.
6. Make inclusion policy explicit. “Physical disk”, “whole block device”, and “device whose I/O should be counted” are different sets.

Avoid relying solely on device-name patterns or `rotational`; virtual machines, USB bridges, multipath, and RAID controllers can report misleading metadata.

## Parse statistics defensively

- For `/proc/diskstats`, parse `major minor name` followed by the statistic fields.
- Accept the documented base fields and optional discard/flush extensions; do not require one exact column count.
- Parse counters as `u64` and reject malformed mandatory fields with contextual errors.
- In diskstats throughput calculations, one reported sector is 512 bytes. Do not substitute `logical_block_size`.
- Treat `ios_in_progress` as a gauge, not a monotonic counter.
- Preserve unknown trailing fields for forward compatibility by ignoring them after recognized fields.

## Calculate deltas correctly

Use `std::time::Instant` and actual elapsed seconds:

```text
read MiB/s  = delta(sectors_read)    * 512 / 1_048_576 / elapsed_seconds
write MiB/s = delta(sectors_written) * 512 / 1_048_576 / elapsed_seconds
read IOPS   = delta(reads_completed)  / elapsed_seconds
write IOPS  = delta(writes_completed) / elapsed_seconds
utilization = delta(io_ticks_ms) / elapsed_ms * 100
avg read latency ms  = delta(read_time_ms)  / delta(reads_completed)
avg write latency ms = delta(write_time_ms) / delta(writes_completed)
avg queue depth = delta(weighted_time_in_queue_ms) / elapsed_ms
```

Return unavailable metrics when the denominator is zero. Do not silently manufacture zero latency.

Do not use `wrapping_sub` blindly. If a monotonic counter decreases, distinguish a plausible integer wrap from device reset/replacement; for ordinary `u64` counters, treat decreases as a reset and skip that interval unless wrap is demonstrably supported. Use `diskseq` where available to detect replacement. Clamp utilization only for presentation if desired, while retaining the raw computed value for diagnosis.

## Handle runtime changes

- If a sysfs entry disappears during collection, report removal without panicking and discard its previous sample.
- On a new or replaced device, establish a baseline before emitting rate metrics.
- Re-enumerate periodically even when using udev events.
- Do not hold an async mutex while reading the filesystem.
- For a small number of tiny procfs/sysfs reads, prefer simple synchronous reads; if collection could delay an async runtime, perform the whole snapshot in one bounded blocking task rather than spawning one task per file.

## Test before integrating

Create fixture trees covering:

- base, discard, and flush field variants;
- HDD, NVMe, partition, loop, device-mapper, mdraid, and hidden paths;
- missing optional metadata and malformed mandatory statistics;
- zero interval, idle interval, reset/decrease, removal, reappearance, and device-name reuse;
- multiple devices from a single `/proc/diskstats` snapshot;
- known two-sample metric calculations with exact expected values.

Use `iostat` only as an integration-test comparison tool. Account for different sampling windows and rounding before calling a mismatch a defect.

## Review checklist

Before completing work, confirm:

- No runtime shell or external CLI dependency was introduced.
- Kernel paths and units are documented beside parsers.
- The 512-byte diskstats sector rule is covered by a test.
- Optional fields and missing metadata do not crash collection.
- Reset/replacement does not create a huge false throughput spike.
- Discovery policy is explicit for partitions, stacked devices, and NVMe.
- Collection can be tested under an injected filesystem root.
- `cargo fmt`, targeted tests, full tests, and clippy pass.
