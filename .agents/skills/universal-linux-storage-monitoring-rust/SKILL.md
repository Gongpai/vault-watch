---
name: universal-linux-storage-monitoring-rust
description: Design, implement, review, or migrate a universal Linux storage discovery and monitoring architecture in Rust. Use for graph-based sysfs topology, physical/logical/virtual/remote classification, scoped persistent identity, hot-plug generation tracking, health/throughput/RAID backend routing across SCSI/SAS, SATA/SAT, NVMe, MMC, USB, MD, DM, multipath, hardware RAID, fabrics, and virtual devices, unified metric/error models, safe polling schedulers, privilege brokers, and avoiding cross-layer double counting.
---

# Universal Linux Storage Monitoring Architecture

Build the graph and identity model before selecting protocol backends. Treat a Linux block device as one observable object in a layered storage graph, not automatically as one physical disk.

## Read the reference selectively

Read [references/universal-storage-architecture-research.md](references/universal-storage-architecture-research.md) before changing the graph model, identity hierarchy, routing, metric scope, scheduler, or security boundary. It contains capability matrices, topology design, backend decisions, Rust traits, runtime policy, tests, roadmap, and source links.

For narrow tasks, locate sections with these searches:

- Scope and supported storage: `Executive summary` or `capability matrix`.
- Graph/discovery: `Canonical topology graph` or `Discovery and classification`.
- Identity/routing: `Persistent identity` or `Backend-selection`.
- Runtime: `Polling scheduler`, `hot-plug lifecycle`, or `Unified metric`.
- Security/tests: `Security architecture`, `Fixture`, or `hardware qualification`.

Treat the report as research input. Verify subsystem-specific claims against current Linux ABI/UAPI and protocol specifications. Do not preserve cited crate versions or testing-ABI assumptions as timeless facts.

## Use companion protocol skills

When available, read the relevant sibling skill before implementing its backend:

- `$linux-disk-monitoring-rust` for block discovery and throughput;
- `$linux-md-raid-monitoring-rust` for MD arrays;
- `$native-sas-scsi-health-rust` for SCSI/SAS SG_IO;
- `$native-sata-ata-health-rust` for ATA/SAT SMART;
- `$native-nvme-health-rust` for NVMe health;
- `$linux-usb-removable-storage-rust` for USB, removable media, SD, and eMMC.

This skill owns orchestration, graph semantics, identity, routing, scheduling, and unified output. Protocol skills own wire formats and command safety.

## Follow graph-first, backend-late workflow

Use this order:

1. Capture a bounded sysfs/udev discovery snapshot.
2. Create nodes and typed relationships without issuing raw protocol commands.
3. Classify placement, materialization, protocol view, exposure, removability, and lifecycle state independently.
4. Resolve scoped identities and generation discriminators.
5. Determine safe backend eligibility.
6. Select and schedule eligible backends under policy and resource limits.
7. Emit scoped metrics and availability results.
8. Reconcile events against a new topology snapshot.

Never probe health merely to discover what an object is when sysfs topology can answer safely.

## Model a directed multigraph

Represent nodes such as:

- block device and partition;
- physical-medium candidate;
- DM mapping/transform and multipath aggregate;
- MD array;
- NVMe subsystem, controller/path, and namespace;
- SCSI host, target, LUN, and remote port;
- USB device/interface and PCI/Thunderbolt function;
- MMC card/device;
- transport endpoint, hardware controller logical volume, and virtual backing object.

Use typed edges such as:

- `contains_partition`;
- `maps_to` or `backed_by`;
- `member_of`;
- `exports_block`;
- `controller_of` and `namespace_of`;
- `path_to` and `same_identity_group`;
- `parent_bus`;
- `backed_by_file`.

Do not force the graph into a parent-pointer tree. Support fan-in, fan-out, aliases, multiple paths, and defensive cycle detection. Separate kernel-object nodes from deduplicated logical-identity groups.

## Capture topology defensively

- Enumerate `/sys/class/block` as a flat class and resolve relationships through symlinks/subsystem parents.
- Do not depend on `/sys/devices` path spelling, fixed parent counts, or device-name patterns as stable ABI.
- Read block `dev`, `diskseq`, partition relation, hidden state, queue/capacity metadata, holders, slaves, and optional subsystem attributes.
- Add DM, MD, NVMe, SCSI, USB, MMC, FC/iSCSI, loop/NBD, virtio, and other enrichments only when their interfaces exist.
- Mark each attribute as stable ABI, testing ABI, implementation hint, or inferred classification.
- Treat disappearance during traversal as a normal hot-plug race.

Sysfs does not provide an atomic system-wide snapshot. Build a transactional discovery generation, validate critical boundaries, and retry or publish a marked partial snapshot rather than mixing old and new nodes invisibly.

## Classify on independent axes

Avoid a single `DeviceKind` enum that loses information. Track at least:

```text
placement: local | remote | unknown
materialization: physical-candidate | logical | virtual | stacked | partition
protocol_view: scsi | ata-via-sat | nvme | mmc | md | dm | virtio | unknown
exposure: direct | translated | aggregated | hidden | unknown
removability: fixed | removable | media-absent | unknown
health_scope: physical | controller | namespace/lun | array | logical-volume | path | none
```

Do not claim physical media when the host sees only a SAN LUN, virtual disk, hardware RAID volume, bridge-translated object, or guest block device.

## Resolve scoped identity

Store identity as typed claims, not one string:

```text
IdentityClaim { kind, value, scope, issuer/source, confidence }
Generation { boot_id, diskseq?, dev_t?, first_seen }
```

- Scope SCSI VPD identifiers to the association/designator semantics they declare.
- Combine NVMe subsystem and namespace identity; NQN alone does not identify a namespace.
- Distinguish ATA WWN/serial, USB bridge serial, enclosure/port locator, MD UUID, DM UUID, multipath WWID, FC/iSCSI endpoint, and filesystem UUID.
- Never use filesystem UUID as hardware identity.
- Treat USB port, PCI BDF, NSID, LUN path, block name, and `dev_t` primarily as locators or generation context, not universal persistent identity.
- Use `diskseq` where present to detect a new block-device incarnation, but do not assume availability or cross-layer equivalence.
- Preserve collisions and ambiguity rather than silently merging nodes.

Deduplicate only when identity scope and topology evidence agree. Keep aliases and paths attached to the identity group.

## Gate backend eligibility before scoring

Use hard eligibility gates before ranking candidates:

1. Confirm node scope matches the backend.
2. Confirm required kernel interface and protocol capability.
3. Confirm command is allowed by read-only policy.
4. Confirm privilege deployment permits it.
5. Confirm power/no-wake policy permits it.
6. Confirm device/controller/bridge is not quarantined.
7. Confirm backend implementation and hardware qualification cover the variant.

Only then rank by directness, scope fit, confidence, safety, privilege cost, latency, and fallback preference. A high directness score must never override a safety or qualification gate.

Examples:

- native NVMe controller -> native NVMe backend;
- native SAS/SCSI LUN -> SCSI backend at LUN/device scope;
- ATA via confirmed SAT -> SATA backend;
- MD array -> MD backend plus separate member backends;
- DM/multipath -> logical/path state plus health on underlying eligible nodes;
- USB bridge -> standard backend or a specifically qualified quirk backend;
- hardware RAID volume -> logical/controller backend; physical members hidden unless a qualified controller backend exists;
- loop/NBD/virtio/ublk/zram -> logical throughput, physical health unsupported.

Keep external CLI fallback opt-in, lowest priority, out of process, and explicitly labelled.

## Separate metric source from scope

Every observation should include:

```text
key, value, unit, source, scope, subject identity,
availability, confidence, observed_at, age/latency, backend, generation
```

Use availability states such as:

```text
Available | Unsupported | Hidden | PermissionDenied | PolicyBlocked |
Asleep | TemporarilyUnavailable | Stale | Malformed | DeviceGone
```

Do not encode unavailable as numeric zero. Do not collapse transport failure, media failure, path failure, degraded array, and unsupported health into one Boolean.

Block-layer throughput counters at different stack layers are not additive. Export selected layers with explicit scope, or choose one presentation layer. Never sum partition + disk, DM + MD + member, or multipath aggregate + paths as total throughput without a formally defined deduplication model.

## Aggregate health explicitly

- Keep raw backend metrics attached to their original subject.
- Derive array, subsystem, multipath, or controller summaries through named policy.
- For multipath, distinguish path reachability from namespace/LUN health.
- For MD, distinguish array redundancy from member-media health.
- For hardware RAID, distinguish controller/logical-volume health from hidden members.
- For remote storage, distinguish transport/session health from remote physical media health.
- Include contributing node IDs and policy version in derived results.

Never let one successful temperature or identity read imply overall healthy status.

## Schedule safely

Use two runtime planes:

- event plane: udev/netlink/subsystem notifications and timers;
- work plane: paced, bounded collection jobs.

Apply:

- one or a small qualified number of management commands per controller/bridge/HBA/session;
- global concurrency limits;
- per-bus/controller token buckets;
- jittered intervals;
- exponential or classified backoff;
- no-wake policy for suspended removable/rotating devices;
- quarantine after resets, repeated timeouts, or malformed bridge responses;
- cancellation-aware shutdown without assuming an in-flight ioctl can be cancelled.

Events are hints. Coalesce them, resnapshot topology, compute a graph patch, validate identities/generations, then publish atomically. Periodic reconciliation is mandatory for missed events.

## Use a universal security boundary

- Keep discovery/sysfs collection unprivileged where possible.
- Isolate raw protocol access in a small command broker.
- Accept typed high-level requests tied to validated `DeviceId`, generation, backend, and policy—not raw paths, opcodes, CDBs, taskfiles, CDWs, MMC commands, or pointers.
- Enforce per-backend command allowlists, data-direction rules, transfer/time limits, device allowlists, peer authentication, audit records, and resource limits.
- Default-deny format, sanitize, firmware, security, namespace management, reset/rescan, start/stop, writes, vendor commands, and state-changing sysfs.
- Revalidate identity after opening and before executing when practical.

Do not grant broad raw-I/O capability to the frontend. Treat plugin and vendor backends as privileged code requiring the same review as the broker.

## Design for partial support

Report unsupported or hidden cases as first-class output:

- physical members behind an unsupported hardware RAID controller;
- USB bridges without qualified passthrough;
- SAN/virtual LUNs without physical-media visibility;
- guest virtual disks;
- SD cards without standardized health;
- missing kernel subsystem interfaces;
- ambiguous or colliding identities.

The topology, logical throughput, and availability state can still be useful when physical health is unavailable.

## Test the whole system

Create synthetic sysfs graphs for:

- whole disks/partitions and block-name reuse;
- stacked LVM, dm-crypt, thin/cache/verity, and multiple layers;
- MD arrays with members/spares and nested MD;
- DM multipath over FC/iSCSI and NVMe multipath;
- NVMe subsystem/controller/namespace/path relations;
- SATA through libata, SAS HBA, USB SAT, and unsupported bridge;
- hardware RAID logical volume with hidden members;
- loop, NBD, virtio-blk/scsi, zram, and ublk;
- eMMC, native SD, USB card reader, and media absence;
- hot-remove/re-add, diskseq change, ID collision, partial snapshot, and graph cycle.

Add protocol fixtures through companion skills. Test backend gates/scoring, no unsafe fallback, metric scopes, no cross-layer double counting, event coalescing, transactional graph patches, cache invalidation, and broker rejection.

Use property tests for graph invariants:

- every edge references existing nodes;
- partition is not a physical medium;
- every metric subject exists in the same generation;
- merged identity claims have compatible scope;
- unsupported nodes receive no privileged probe;
- graph traversal terminates despite cycles;
- derived health lists its contributors;
- removing a node cannot leave live dangling capabilities/jobs.

Require hardware qualification across representative local, remote, bridge, multipath, RAID, removable, and virtual environments before claiming support.

## Review checklist

Before completing work, confirm:

- Discovery is graph-based and does not force one block node to one physical disk.
- Sysfs path spellings and testing ABI are not treated as stable identity.
- Identity claims include scope, source, confidence, and generation.
- Backend safety/qualification gates run before scoring.
- Metrics include source, scope, availability, subject, and generation.
- Cross-layer throughput is never added blindly.
- Derived health names its aggregation policy and contributors.
- Events always reconcile against a new snapshot.
- Scheduler limits work per device, controller, bridge, HBA, and globally.
- Unsupported/hidden storage remains visible without false health claims.
- Public APIs and plugins cannot submit arbitrary privileged commands.
- Companion backend safety rules remain intact.
- Graph, fixture, property, fuzz, integration, clippy, and hardware tests pass.
