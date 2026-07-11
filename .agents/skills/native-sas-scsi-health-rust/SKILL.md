---
name: native-sas-scsi-health-rust
description: Design, implement, review, or harden native Linux SAS/SCSI disk health monitoring in Rust using SG_IO and read-only SCSI commands. Use for block-to-scsi_generic mapping, SG_IO ABI wrappers, CDB construction, INQUIRY/VPD/LOG SENSE parsing, fixed or descriptor sense decoding, SAS-versus-SAT detection, privilege separation, controller passthrough failures, binary-fixture tests, or replacing smartctl with a native backend plus fallback.
---

# Native SAS/SCSI Health Monitoring in Rust

Build a Linux-only, read-mostly SCSI health backend with a narrow command allowlist. Separate transport correctness, protocol parsing, device capability discovery, and health interpretation.

## Read the reference selectively

Read [references/native-sas-scsi-health-research.md](references/native-sas-scsi-health-research.md) before changing the SG_IO ABI, command bytes, parser layouts, permissions, or retry behavior. It contains command/page tables, request flow, Rust sketches, sense mappings, security guidance, tests, standards, and source links.

For a narrow task, locate the relevant section first:

- Commands and pages: search for `Minimum Viable`, `Command/Log Page`, or `VPD Pages`.
- SG_IO mechanics: search for `Request/Response Flow`, `Native SG_IO Backend`, or `Safe Rust Code Skeleton`.
- Device mapping: search for `Block Device → Generic`.
- Error handling: search for `Sense/Error Decoding` or `CHECK CONDITION`.
- Routing and privileges: search for `Decision Tree` or `Security and Privilege`.
- Verification: search for `Test and Validation`, `Hardware Integration`, or `Primary Source`.

Treat the report as research input. Confirm release-critical layouts and opcode permissions against the Linux UAPI header, kernel SG documentation, and relevant T10 standard revision.

## Establish the support boundary

Classify every target before issuing health commands:

```text
Native SAS/SCSI -> native SG_IO LOG SENSE backend
SATA through SAT -> ATA passthrough backend or smartctl fallback
Controller-hidden physical disk -> controller-specific backend/fallback
NVMe or non-direct-access device -> out of this skill's SCSI disk path
```

Do not infer native SAS solely from a `/dev/sdX` name. Use standard INQUIRY, supported VPD pages, VPD 0x89 evidence, peripheral device type, sysfs topology, and actual command outcomes. Report ambiguous routing explicitly.

## Layer the implementation

Keep these components independent:

```text
DeviceMapper -> ScsiTransport -> CommandBuilder -> PageParser -> HealthAggregator
                                      |-> SenseDecoder
```

- Make `ScsiTransport` mockable with raw response fixtures.
- Return transport status, SCSI status, host status, driver status, residual, sense bytes, and payload separately.
- Keep parsers pure and free of file descriptors or ioctl calls.
- Represent unsupported and unavailable metrics separately from healthy zero values.
- Keep fallback routing outside individual page parsers.

## Map devices safely

1. Resolve the block device through sysfs.
2. Inspect `device/scsi_generic/` and allow zero, one, or multiple entries.
3. Resolve identities through sysfs and VPD 0x83 rather than persisting `sgN`, whose numbering can change.
4. Handle device removal and mapping changes between discovery and open.
5. Do not automatically run `modprobe sg`; missing SG support is an environmental capability result.
6. Consider using SG_IO on the block device only after documenting permission and controller differences.

Avoid deriving relationships by major/minor equality alone, especially with multipath and controller abstractions.

## Constrain SG_IO unsafety

- Prefer generated or carefully verified bindings matching the target system's `<scsi/sg.h>` over casually retyping `sg_io_hdr_t`.
- If defining the structure locally, use `#[repr(C)]`, compile-time size/alignment checks for supported architectures, correct pointer mutability, and UAPI-derived constants.
- Keep CDB, sense, and data buffers alive and immovable for the synchronous ioctl duration.
- Zero initialize the header, cap all lengths to their ABI field widths, and reject oversized CDBs or buffers.
- Use bounded nonzero timeouts. A userspace future cancellation does not necessarily cancel an in-flight ioctl.
- Execute blocking ioctls in a bounded blocking worker; do not hold an async mutex across the call.
- Centralize the unsafe ioctl in one small reviewed module.

Validate every returned length. `resid` may be unexpected or negative; accept payload length only when it lies within the allocated buffer. Limit sense length by both `sb_len_wr` and buffer capacity.

## Enforce a read-only command policy

Start with an explicit allowlist:

- TEST UNIT READY
- standard INQUIRY
- selected INQUIRY VPD pages
- LOG SENSE for discovered supported pages
- MODE SENSE only when a documented read-only need exists

Never expose arbitrary CDB execution to the UI or configuration. Reject all data-to-device directions in the monitoring transport. Do not include FORMAT UNIT, LOG SELECT, MODE SELECT, WRITE BUFFER, SANITIZE, START STOP UNIT, or vendor commands in the allowlist.

Treat READ DEFECT DATA and ATA PASS-THROUGH as separate, elevated-risk capabilities. Do not place them in the initial periodic polling loop; require explicit support, permission analysis, hardware tests, and conservative cadence.

“Read-only command” does not guarantee access. Linux opcode filtering, node permissions, capabilities, device type, HBA, and controller firmware all affect authorization and forwarding.

## Discover capabilities before polling

Use staged allocation and capability discovery:

1. Send standard INQUIRY and validate peripheral qualifier/type and response length.
2. Query supported VPD page 0x00, then only request advertised VPD pages such as 0x80, 0x83, 0x89, or 0xB1.
3. Query supported LOG SENSE pages before scheduling individual log pages.
4. Read a short header first when response length is variable, validate the advertised size against a configured maximum, then issue a second request if needed.
5. Cache static identity/capability data and refresh it after unit attention, reset, or device replacement.

Do not assume a page is mandatory merely because common disks implement it. Preserve vendor-specific and unknown parameters without interpreting them as standards-defined health.

## Parse protocol data defensively

- Decode all SCSI multibyte integers as big-endian.
- Check page code, subpage, declared length, allocation length, and actual transferred length before parsing parameters.
- Iterate parameter records with checked arithmetic and reject truncated records.
- Support variable-width integer values only where the parameter definition permits them.
- Preserve raw page bytes or diagnostic context for unsupported layouts without exposing sensitive identifiers unnecessarily.
- Treat sentinel values such as temperature `0xff` as unavailable.
- Never turn malformed records into zero counters.

Keep standardized, vendor-specific, and empirically observed parameter mappings in separate modules or tables.

## Evaluate command completion correctly

Evaluate in layers:

1. ioctl return and `errno`;
2. host/transport status;
3. driver status and SG info bits;
4. SCSI status;
5. bounded sense decode for CHECK CONDITION;
6. payload validation only after a successful data-in outcome.

Decode both fixed-format (`0x70/0x71`) and descriptor-format (`0x72/0x73`) sense. Preserve unknown formats and raw ASC/ASCQ.

Use bounded retry rules:

- Unit Attention: refresh identity/capabilities and retry once when safe.
- BUSY or task-set conditions: retry with bounded backoff.
- NOT READY: expose state and retry only when ASC/ASCQ indicates a transient condition.
- ILLEGAL REQUEST for an optional page: mark that capability unsupported.
- MEDIUM ERROR or HARDWARE ERROR: preserve and surface the failure; do not hide it with retries.
- Reservation conflict or permission denial: report without bypass attempts.

Do not issue REQUEST SENSE reflexively when autosense is already returned; use it only when protocol and transport behavior require it.

## Aggregate health without false certainty

Model each metric as available, unsupported, temporarily unavailable, malformed, or failed. Keep overall outcomes such as:

```text
Healthy | Warning | Failing | Unknown | Unreachable | Unsupported
```

Derive failure only from documented signals and product policy. Do not declare a disk healthy merely because temperature succeeded or unsupported error pages returned no values. Keep corrected errors, uncorrected errors, informational exceptions, background scan results, and transport failures distinct.

Use different polling cadences: temperature and readiness may be frequent; error pages slower; identity static; defect data rare or on demand. Add jitter and avoid synchronized bursts across many disks.

## Design privileges narrowly

- Prefer device-node read permissions for the strict read-only allowlist when sufficient.
- Avoid granting `CAP_SYS_RAWIO` to the full TUI; it broadens the command surface substantially.
- If elevated access is unavoidable, isolate it in a small helper that validates device identity, command kind, direction, allocation length, and timeout before executing.
- Return parsed or structured results to an unprivileged frontend rather than accepting raw CDBs over IPC.
- Apply peer authentication, request limits, and device allowlists to any Unix socket helper.
- Reopen or invalidate descriptors after hot removal and verify the device identity before reusing a mapping.

## Respect licensing

Use Linux UAPI and T10 specifications for ABI and wire formats. Treat sg3_utils utilities and smartmontools as behavioral references or validation oracles. Do not copy GPL implementation code into an MIT-licensed project. Verify the license of any library source used as a structural reference.

## Test before hardware rollout

Create binary fixtures and tests for:

- standard INQUIRY and short/truncated responses;
- VPD 0x00/0x80/0x83/0x89/0xB1 including duplicate and unknown descriptors;
- LOG SENSE supported-pages, temperature, error counters, informational exceptions, and unknown parameters;
- fixed and descriptor sense, short sense, illegal request, unit attention, not ready, medium error, hardware error, and busy status;
- positive, zero, full, negative, and inconsistent residual values;
- unsupported pages, transport failure, permission denial, timeout, removal, and replacement;
- native SAS, SAT, USB bridge, multipath, and controller-hidden routing decisions;
- command allowlist rejection for every unsafe opcode and every data-to-device direction.

Capture fixtures only with explicit authorization on non-production or safely monitored hardware. Strip or document serials and globally unique identifiers before committing fixtures. Compare native results against sg3_utils and smartctl only as development validation, not runtime dependencies.

Require real hardware tests across representative SAS HDDs, SAS SSDs, SAT devices, expanders/HBAs, and supported RAID controllers before claiming compatibility.

## Review checklist

Before completing work, confirm:

- Arbitrary CDB and data-to-device execution are impossible through public APIs.
- The SG_IO layout and constants match supported architectures and kernel UAPI.
- Returned lengths, residuals, and all parser offsets are bounds checked.
- Unsupported or malformed data cannot become a healthy value.
- Retry loops are bounded and sense-aware.
- Blocking ioctls cannot stall the async executor or hold shared-state locks.
- Device identity is revalidated after reset, removal, or remapping.
- SAT and controller-hidden devices route to explicit fallback or unsupported states.
- Elevated privileges are isolated from the main UI.
- Parser tests use raw binary fixtures and unsafe-opcode rejection tests.
- Licensing provenance is recorded for protocol tables and implementation references.
- `cargo fmt`, targeted tests, full tests, clippy, and hardware smoke tests pass.
