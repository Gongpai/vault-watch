---
name: native-nvme-health-rust
description: Design, implement, review, or harden native Linux NVMe SSD health monitoring in Rust using NVMe controller ioctls and sysfs without runtime nvme-cli or smartctl. Use for controller/subsystem/namespace discovery, Identify and Get Log Page commands, SMART/Health and endurance parsing, little-endian 128-bit counters, multipath or NVMe-oF scope, AER-triggered reconciliation, privilege separation, binary fixtures, fuzzing, and safe admin-command allowlists.
---

# Native Linux NVMe Health Monitoring in Rust

Build a Linux-only NVMe health backend with explicit controller, namespace, subsystem, path, and endurance-group scope. Keep kernel transport status, command result, protocol payload, derived metrics, and health policy separate.

## Read the reference selectively

Read [references/native-nvme-health-research.md](references/native-nvme-health-research.md) before changing ioctl ABI, command dwords, Identify layouts, log parsing, multipath identity, or privilege behavior. It contains interface tables, field maps, Rust sketches, event design, tests, standards, and source links.

For narrow tasks, locate sections with these searches:

- Interfaces: `Linux NVMe Interface Comparison` or `Safe Admin-Command Allowlist`.
- Discovery: `Discovery and Identity Graph` or `Multipath`.
- Identify: `Identify Field Tables`.
- Health/endurance: `SMART/Health Log` or `Enterprise/Endurance`.
- Events: `Event-Driven AER Design`.
- Implementation: `Rust Architecture`, `Safe Ioctl`, or `Endian`.
- Security/tests: `Security Model`, `Fixture`, `Fuzz`, or `Hardware Validation`.

Treat the report as research input. Verify release-critical layouts, command fields, ioctl permissions, log scopes, and version gates against current Linux UAPI/kernel source and the applicable NVMe Base/Command Set specification.

## Model topology before health

Represent this hierarchy explicitly:

```text
Subsystem -> Controller/Path -> Namespace
          -> Endurance Group / NVM Set when supported
```

- Enumerate controllers from `/sys/class/nvme` and subsystems from `/sys/class/nvme-subsystem`.
- Associate namespace block devices with their controller paths and subsystem.
- Preserve transport (`pcie`, `tcp`, `rdma`, `fc`, `loop`), controller state, address, controller ID, NSID, and ANA/path state.
- Do not assume `/dev/nvme0`, `/dev/nvme0n1`, or subsystem indices are persistent.
- Do not treat a multipath namespace block device as an admin-command endpoint.

Build persistent identity as a composite: subsystem identity plus namespace identity. An NQN identifies a subsystem, not an individual namespace. Prefer valid namespace UUID/NGUID/EUI-64 according to specification rules, retain NSID as path-local context, and use model/serial only as a lower-confidence fallback.

## Separate the implementation

Keep these components independent:

```text
TopologyDiscovery -> NvmeTransport -> CommandBuilder -> PureParser
                                                  -> ScopedHealthAggregator
                                                  -> EventReconciler
```

- Mock transport responses with raw binary fixtures.
- Keep sysfs-only metadata/temperature available as a degraded unprivileged backend.
- Represent unsupported, unavailable, permission denied, disconnected, malformed, and genuine zero distinctly.
- Attach source, scope, timestamp, controller/path identity, and confidence to every metric.
- Keep UI alert thresholds outside transport and parsers.

## Constrain ioctl execution

- Prefer generated or carefully verified bindings from `<linux/nvme_ioctl.h>`.
- Do not hardcode an architecture-specific ioctl number; generate it using the platform ABI or a maintained ioctl macro.
- Use `#[repr(C)]` and compile-time size/alignment assertions for every supported target.
- Keep data and metadata buffers alive and unmoved for the synchronous ioctl.
- Validate pointer conversion, `data_len`, metadata length, offsets, dword fields, timeout, and maximum transfer before submission.
- Execute blocking ioctls through a bounded blocking worker unless a separately tested io_uring backend is selected.
- Do not hold shared async locks during command execution.

Interpret ioctl completion correctly:

- return `< 0`: syscall failure; inspect `errno`;
- return `0`: NVMe command success;
- return `> 0`: NVMe command status returned by the kernel; decode it according to the Linux UAPI/kernel behavior;
- `cmd.result`: command-specific completion result, not the generic status field.

Never decode status bits from `cmd.result` unless the particular command defines those bits there.

## Enforce an admin-command allowlist

Expose typed high-level requests only. Start with:

- Identify Controller;
- Identify Namespace;
- Active Namespace List and Namespace Identification Descriptor List when supported;
- Get Log Page for Supported Logs, SMART/Health, Error Information, and Firmware Slot Information;
- Endurance Group Information only after capability and identifier discovery;
- other read-only logs only after specification/version/size validation.

Reject arbitrary opcode/CDW input, metadata pointers, I/O passthrough, and data-out operations. Never expose Format NVM, Sanitize, Firmware Download/Commit, Namespace Management/Attachment, Security Send, Set Features, controller reset, subsystem reset, rescan, or queue management through the health API.

Treat Device Self-test initiation, telemetry capture, and Persistent Event Log context operations as separately reviewed capabilities, not default polling.

## Discover capabilities before commands

1. Read controller state and identity from sysfs.
2. Identify Controller and parse version, controller type, OACS, LPA, ELPE, AERL, MDTS, namespace count, temperature thresholds, and relevant capability identifiers.
3. Enumerate active namespaces and identify each namespace.
4. Query Supported Log Pages when the version/capability permits; otherwise use a conservative version-gated set.
5. Select controller-wide, namespace-specific, or endurance-group log scope only when advertised.
6. Cache static identity/capability data and invalidate it after reset, reconnect, firmware activation, namespace change, or controller replacement.

Unknown versions, bits, log identifiers, and descriptor types must remain representable.

## Build commands from typed fields

- Encode opcode, NSID, CNS, CSI, LID, LSP, RAE, NUMD, log-page offset, UUID index, and LSI/EGID according to the selected specification revision.
- Calculate NUMDL/NUMDU from an aligned dword transfer length with checked arithmetic.
- Split large logs by supported log-page offset and maximum transfer.
- Derive MDTS from the controller's minimum memory page size context; do not assume the base is always 4096 bytes.
- Enforce both protocol and kernel/backend transfer limits.
- Reject unaligned, zero, overflowing, or over-limit requests before ioctl.

Keep each allowed command builder private and test its exact dword encoding.

## Parse Identify data defensively

- Require the correct payload length before reading fixed offsets.
- Decode all multibyte fields as little-endian.
- Trim ASCII space padding while preserving internal characters.
- Validate version and capability bits before interpreting later-revision fields.
- Parse namespace LBA formats using the selected FLBAS index and checked `1 << LBADS` arithmetic.
- Calculate capacity from namespace logical blocks and selected LBA size with overflow checks.
- Treat all-zero UUID, NGUID, EUI-64, capacity, or optional fields according to their specification-defined unavailable semantics.
- Parse namespace descriptor lists as bounded TLV records and preserve unknown descriptor types.

Do not copy packed protocol data directly into Rust structs unless layout, alignment, and endianness are proven; explicit slice parsers are safer.

## Parse SMART/Health with correct units

- Require the full defined log payload and ignore reserved bytes.
- Decode `critical_warning` as independent bits rather than one Boolean.
- Decode Kelvin temperatures and preserve raw Kelvin. For precise display use `millidegrees_c = kelvin * 1000 - 273150`; integer `kelvin - 273` is only an approximation.
- Treat zero temperature sensors as not implemented where specified.
- Preserve `percentage_used` beyond 100 and its saturation behavior; do not clamp the stored value to 100.
- Parse 16-byte counters as little-endian `u128`.
- Use checked arithmetic for unit conversions.

NVMe Data Units Read/Written are reported in units based on 1000 × 512 bytes and rounded up as specified. Multiplying the counter yields a conventional estimate, not necessarily the exact byte count; label it accordingly or preserve a range/rounding note.

Keep controller busy time, power-on hours, warning temperature time, critical temperature time, and thermal-management time in their specified and different units.

## Respect metric scope

- Read controller-wide SMART with the broadcast/controller scope required by the specification.
- Use per-namespace SMART only if the controller advertises it.
- Read endurance-group logs with a discovered valid endurance-group identifier.
- Do not merge health from multiple controllers/paths by simply summing counters.
- On multipath systems, collect each controller/path separately and derive subsystem-level health through explicit policy such as worst critical state plus path availability.
- For NVMe-oF, distinguish controller health from network/path reachability and tolerate reconnecting states.

Never present namespace capacity, controller health, endurance-group wear, and path state as if they came from one scope.

## Handle events through reconciliation

Do not submit Asynchronous Event Request commands from userspace when the Linux NVMe driver already owns AER handling. Listen for relevant udev/netlink change events such as `NVME_AEN`, then issue fresh allowed log reads.

- Treat events as hints, not authoritative state.
- Decode event fields according to the applicable specification/kernel encoding.
- Apply RAE semantics deliberately; reading a log can affect event retention/unmasking.
- Coalesce bursts and maintain periodic reconciliation.
- Invalidate cached topology on namespace, ANA, firmware, reset, reconnect, and removal events.
- Bound concurrent requests per controller and back off while state is not `live`.

## Isolate privileges

- Prefer unprivileged sysfs/hwmon collection when it satisfies the requested metric.
- Do not grant broad admin capabilities to the TUI or network-facing process.
- If passthrough needs privilege, use a small helper with authenticated IPC, controller allowlists, typed requests, fixed opcode/CDW construction, transfer/time limits, and no raw command interface.
- Open verified controller character devices, not arbitrary paths supplied by callers.
- Revalidate subsystem/controller identity after reconnect or device-node reuse.

Do not assume one fixed capability requirement across all kernel versions and commands; detect permission failure and document the supported deployment model.

## Test before rollout

Create fixtures and tests for:

- Identify Controller across NVMe versions and unknown future bits;
- Identify Namespace, LBA formats, descriptor lists, zero/multiple identities, thin provisioning, and overflow;
- SMART logs with every critical bit, zero and multiple sensors, over-100 percentage used, maximum u128 counters, and truncation;
- exact and overflow-safe Data Unit conversions with rounding semantics;
- supported logs, error entries, firmware slots, endurance groups, and unsupported version/capability paths;
- ioctl negative errno, zero success, positive NVMe status, timeout, removal, reset, and reconnect;
- command allowlist rejection for destructive, data-out, I/O, arbitrary-CDW, and over-limit requests;
- multipath controller/namespace identity, ANA/path changes, NVMe-oF reconnect, and duplicate device-node names;
- event coalescing, cache invalidation, RAE behavior, and periodic recovery from missed events.

Fuzz all binary parsers and descriptor walkers with empty, short, all-zero, all-`0xff`, oversized, and random inputs. Use QEMU for basic protocol coverage but require real consumer, enterprise, multipath, and NVMe-oF hardware tests before claiming those capabilities. Use nvme-cli and smartctl only as development oracles.

## Review checklist

Before completing work, confirm:

- Public APIs cannot submit arbitrary opcodes, CDWs, pointers, or data-out commands.
- ioctl constants and structures are generated or ABI-verified per target.
- ioctl return status is not confused with `cmd.result`.
- MDTS uses the correct controller page-size context.
- Every parser is length, endian, reserved-value, and overflow safe.
- Data Unit conversions disclose rounding rather than claiming exact bytes.
- Identity distinguishes subsystem, controller/path, and namespace.
- Metrics retain their controller/namespace/endurance/path scope.
- Userspace does not compete with kernel AER handling.
- Multipath and NVMe-oF failures cannot become healthy values.
- Privileged access is isolated behind typed allowlisted requests.
- Fixture, fuzz, integration, clippy, and hardware smoke tests pass.
