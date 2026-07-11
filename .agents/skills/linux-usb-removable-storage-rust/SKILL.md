---
name: linux-usb-removable-storage-rust
description: Design, implement, review, or harden Linux USB and removable-storage monitoring in Rust using sysfs, udev, SG_IO, SAT, and MMC ioctls. Use for USB block topology, BOT versus UAS detection, USB-to-SATA or USB-to-NVMe bridge routing, removable SCSI media, SD/eMMC classification and EXT_CSD health, no-wake polling, autosuspend, hot-plug identity, bridge quirks, capability probing, privilege separation, fixtures, fuzzing, and graceful unsupported states.
---

# Linux USB and Removable Storage Monitoring in Rust

Build a topology-first monitoring layer for devices whose visible block protocol may differ from the enclosed media. Prefer safe standard discovery, avoid waking or resetting hardware, and report unsupported health honestly.

## Read the reference selectively

Read [references/linux-usb-removable-storage-research.md](references/linux-usb-removable-storage-research.md) before changing topology traversal, bridge probing, SG_IO/MMC commands, power policy, or privilege behavior. It contains capability matrices, sysfs fields, routing guidance, bridge observations, eMMC layouts, tests, and source links.

For narrow tasks, locate sections with these searches:

- Capabilities/routing: `Feasibility Matrix`, `Backend-Selection`, or `Capability Matrix`.
- USB topology: `Sysfs Topology Mapping`, `UAS`, or `BOT`.
- Bridge handling: `Bridge Quirk Architecture` or `Probe Order`.
- SD/eMMC: `SD/eMMC Health Interfaces` or `EXT_CSD`.
- Power: `Power Management and Polling Policy`.
- Security/tests: `Safe Command Allowlist`, `Security Model`, or `Hardware Test Matrix`.

Treat the report as research input. Verify release-critical ioctl layouts, MMC flags, bridge protocols, permissions, and vendor claims against current Linux UAPI/kernel source and the relevant USB, T10/T13, JEDEC, or SD specification.

## Build topology before selecting a backend

Represent the complete relationship:

```text
Block device -> SCSI/MMC/NVMe layer -> USB interface -> USB device -> physical port
```

- Enumerate block devices through `/sys/class/block` and use udev/sysfs parent relationships.
- Walk canonical ancestors and inspect subsystem/driver symlinks; do not climb a fixed number of directories or parse one assumed path shape.
- Record block identity, SCSI LUN, USB interface, USB VID:PID, firmware-relevant revision, optional serial, driver, speed, port topology, removable flag, read-only state, and runtime power state.
- Treat absent attributes as normal and label their ABI stability.
- Do not infer external storage solely from `removable`; many USB enclosures report zero.
- Do not infer media protocol solely from `/dev/sdX`; USB bridges commonly expose SATA or NVMe media as SCSI block devices.

USB bus/port paths are useful topology and reconnect hints, not universally stable device identities. Build identity from multiple sourced properties and retain confidence.

## Separate the implementation

Keep these components independent:

```text
TopologyDiscovery -> DeviceClassifier -> CapabilityProbe -> BackendRouter
                                       -> ProtocolTransport -> PureParser
                                       -> PowerAwareScheduler
```

- Inject a sysfs root and mock protocol transports for tests.
- Keep native SATA, native SCSI, native NVMe, eMMC, SD, standard SAT, and bridge-specific backends separate.
- Represent unsupported, no media, asleep, permission denied, removed, malformed, unstable bridge, and failed media distinctly.
- Cache capability probes by validated device/bridge identity and invalidate them on reconnect or firmware/topology change.
- Attach protocol, bridge, scope, source, timestamp, and confidence to health results.

## Route conservatively

Use this order:

1. Identify native NVMe and route to the native NVMe backend.
2. Identify the Linux MMC subsystem, then distinguish MMC/eMMC from SD before issuing protocol commands.
3. For USB-backed SCSI devices, identify BOT versus UAS and collect standard SCSI identity/capabilities first.
4. Use standard SAT only when supported by evidence and safe command results.
5. Select a bridge-specific SATA or SCSI-NVMe-translation backend only for a verified VID:PID, firmware, protocol schema, and hardware-qualified implementation.
6. Otherwise expose generic SCSI identity/health if supported, or `HealthUnsupported`.

Thunderbolt/USB4 PCIe tunneling may expose native NVMe and is not equivalent to a USB mass-storage bridge. Route according to actual kernel topology.

## Probe capabilities without destabilizing the bus

- Start with sysfs and standard SCSI INQUIRY.
- Query only advertised VPD/LOG SENSE pages where practical.
- Use typed read-only SAT IDENTIFY as a bounded probe only when topology and policy permit it.
- Do not send vendor opcodes to unknown devices to guess the bridge family.
- Do not cycle through every known passthrough dialect.
- Limit probe count, concurrency, transfer size, and timeout per USB device/controller.
- After timeout, reset, disconnect, or malformed response, stop probing and mark the bridge unstable until reconnect or an explicit cooldown expires.
- Treat capability rejection as a normal result, not a warning storm.

APT-16 to APT-12 fallback must be backed by a known SAT/bridge compatibility rule; it is not a universal recovery action.

## Handle bridge quirks with provenance

- Match quirks using VID:PID plus firmware/product/interface evidence when possible.
- Avoid broad PID masks unless backed by tests across the entire family.
- Store protocol, supported command subset, transfer limits, timeout, wake behavior, test evidence, source, and confidence.
- Feature-gate reverse-engineered SNT or legacy vendor tunnels and disable them by default until hardware fixtures and safety tests exist.
- Never copy smartmontools GPL databases or vendor tunnel implementations into permissively licensed code.
- Treat public VID:PID data as identification only; it does not prove passthrough support.

USB-to-NVMe health through SNT is vendor-specific. Do not reuse native NVMe ioctl assumptions or claim general NVMe enclosure support.

## Preserve power state

Adopt an explicit no-wake policy:

- Read runtime power information as a hint; its exact location can be on a parent device rather than the block-device directory.
- If the relevant USB/SCSI device is suspended and policy is `NoWake`, skip protocol health commands and retain the last successful snapshot with age.
- Distinguish skipped-asleep from unreachable and failed.
- Do not change `power/control`, autosuspend delay, or spin state from monitoring code.
- Avoid START STOP UNIT and other commands that deliberately change power state.
- Apply longer cadence and jitter to rotating USB disks; avoid synchronized polling across hubs.

Document that even nominally read-only queries may wake some devices or bridges. Qualify no-wake behavior on real hardware.

## Monitor standard SCSI/SAT safely

Reuse strict native SCSI and SATA skills/backends rather than duplicating arbitrary SG_IO execution:

- allow only typed data-in or non-data monitoring operations;
- reject arbitrary CDB/taskfile bytes and all data-out directions;
- validate sense data, ATA return descriptors, residuals, lengths, and checksums;
- bound retries by classified transport/sense outcomes;
- use whole-device nodes and revalidate identity after reconnect;
- execute blocking ioctls in bounded blocking workers.

Do not issue REQUEST SENSE reflexively when autosense already contains the required result.

## Distinguish eMMC from SD before MMC ioctls

- Confirm the kernel MMC device type and card identity before selecting commands.
- Treat MMC `SEND_EXT_CSD` as an eMMC/MMC operation; SD CMD8 has different semantics.
- Use generated or ABI-verified `mmc_ioc_cmd` bindings, including pointer-width and ioctl-number checks.
- Hardcode a read-only MMC command allowlist and reject `write_flag != 0`, SWITCH, sanitize, erase, partition configuration, boot configuration, and arbitrary opcodes.
- Require the expected 512-byte EXT_CSD response and check revision before interpreting later fields.
- Parse `PRE_EOL_INFO` and lifetime estimate A/B as specification-defined ranges, not exact percentages remaining.
- Preserve reserved, not-defined, and out-of-range values as unknown.
- Treat debugfs EXT_CSD as a development oracle only, not stable runtime ABI.

For SD cards, report health/endurance unsupported unless a public, implemented standard and Linux interface proves otherwise. Do not invent SMART semantics from vendor registers.

## Handle removable media lifecycle

- Treat udev events as hints and reconcile the full sysfs topology.
- Separate reader/enclosure presence from media presence.
- Handle empty card readers, media insertion, removal during ioctl, reconnect under a reused block name, and multiple LUNs.
- Establish a fresh baseline after reconnect; never attach cached health to a new device solely because `/dev/sdX` matches.
- Keep tombstone/last-seen state keyed by composite identity and topology confidence.
- Coalesce event bursts and debounce add/change sequences.

## Isolate privileges

- Prefer unprivileged sysfs identity/topology collection where possible.
- Keep SG_IO, ATA passthrough, SNT, and MMC raw access out of the TUI/network-facing process.
- Use a small helper with authenticated IPC, stable-device allowlists, typed requests, fixed command construction, transfer/time limits, per-bus concurrency limits, and no raw command interface.
- Detect permission failures rather than assuming one capability/group rule works across kernels and device nodes.
- Never grant broad raw-I/O capability merely to obtain data the device does not standardize.

## Test before claiming support

Create topology and binary fixtures for:

- BOT and UAS devices, multiple interfaces/LUNs, missing serials, reused VID:PID, and nested USB hubs;
- USB SATA with standard SAT, APT-12-only qualified devices, rejected passthrough, truncated sense, reset, and timeout;
- USB NVMe bridges with explicit supported/unsupported SNT backends;
- generic flash drives, empty/removable card readers, removable SCSI disks, and no-health devices;
- native eMMC revisions, healthy/warning/urgent/undefined EXT_CSD values, SD cards, and malformed MMC payloads;
- suspended/active/unknown power state and proof that NoWake skips commands;
- hot unplug during discovery/ioctl, reconnect under a reused device name, and identity collision;
- allowlist rejection for vendor commands, MMC writes, arbitrary CDBs, and data-out operations;
- probe budgets, cooldown, event coalescing, and bridge-instability quarantine.

Fuzz topology parsers, SCSI/ATA responses, bridge descriptors, EXT_CSD, CID/CSD, and event property parsing. Use external tools only as development oracles. Require hardware qualification for every named bridge/firmware and for no-wake claims.

## Review checklist

Before completing work, confirm:

- Sysfs traversal follows relationships rather than fixed parent counts.
- Device identity does not rely solely on block name, USB serial, port path, or removable flag.
- Unknown bridges never receive vendor-specific probes.
- Probe attempts, concurrency, timeout, and reset recovery are bounded.
- Suspended devices are skipped under NoWake policy without being marked failed.
- eMMC commands cannot be sent to SD cards through type confusion.
- MMC/SG_IO public APIs cannot submit arbitrary or data-out commands.
- Unsupported health remains distinct from healthy state.
- Bridge quirks include provenance and hardware evidence.
- Reconnect invalidates cached transport capability and health identity.
- Privileged execution is isolated behind typed allowlisted requests.
- Fixture, fuzz, integration, clippy, hot-plug, and hardware tests pass.
