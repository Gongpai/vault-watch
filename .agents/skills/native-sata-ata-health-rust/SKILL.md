---
name: native-sata-ata-health-rust
description: Design, implement, review, or harden native Linux SATA/ATA disk and SSD health monitoring in Rust using SG_IO and SAT ATA PASS-THROUGH without a runtime smartctl or hdparm dependency. Use for ATA IDENTIFY, SMART READ DATA/THRESHOLDS/RETURN STATUS, General Purpose Logging, ATA return descriptor decoding, SATA SSD endurance, USB bridge or hardware RAID routing, vendor-specific SMART schemas, privilege separation, binary fixtures, fuzzing, and safe command allowlists.
---

# Native SATA/ATA Health Monitoring in Rust

Build a Linux-only ATA health backend with strict command construction and explicit device routing. Keep transport results, ATA register status, raw SMART data, vendor interpretation, and user-facing health conclusions separate.

## Read the reference selectively

Read [references/native-sata-ata-health-research.md](references/native-sata-ata-health-research.md) before changing command bytes, SG_IO ABI, ATA word layouts, SMART interpretation, bridge probing, or privileges. It contains field tables, SAT layouts, vendor examples, Rust sketches, tests, standards, and source links.

For narrow tasks, locate sections with these searches:

- Commands and CDBs: `Command Allowlist`, `ATA PASS-THROUGH`, or `Execution Flow`.
- IDENTIFY parsing: `IDENTIFY DEVICE Field Table`.
- SMART parsing: `SMART Data / Log Structures` or `Status Decoding`.
- SSD endurance: `Vendor-Specific Attribute Matrix` or `Endurance Metric Matrix`.
- Routing: `Decision Tree`, `USB Bridge`, or `Hardware RAID`.
- Security and validation: `Privilege`, `Fixture`, `Fuzzing`, or `Hardware Test`.

Treat the report as research input. Verify release-critical command layouts, permissions, ATA word meanings, and model-specific attributes against Linux UAPI, the applicable T10 SAT/T13 ACS revision, and official vendor documentation.

## Classify the transport first

Route each target explicitly:

```text
Native SATA via libata -> SAT ATA PASS-THROUGH
SATA behind SAS HBA -> SAT if forwarded
USB-to-SATA -> SAT or verified bridge-specific backend
Hardware RAID logical volume -> controller backend/fallback/unsupported
Native SAS/SCSI -> SCSI health backend, not ATA SMART
NVMe -> NVMe backend
```

Do not classify from `/dev/sdX` alone. Combine sysfs topology, standard SCSI INQUIRY, VPD 0x89 where supported, ATA IDENTIFY evidence, and actual safe-command outcomes. Treat ambiguous devices as unknown rather than trying undocumented probes automatically.

## Separate the implementation

Keep these layers independent:

```text
DeviceRouter -> AtaTransport -> SatCdbBuilder -> AtaResponseDecoder
                                              -> PurePageParsers
                                              -> VendorInterpreter
                                              -> HealthAggregator
```

- Mock the transport with captured binary fixtures.
- Preserve raw IDENTIFY words, SMART entries, thresholds, and log pages.
- Represent unsupported, unavailable, malformed, permission denied, transport failure, and genuine zero separately.
- Keep bridge/controller quirks in named backends or a reviewed registry.
- Keep fallback selection outside binary parsers.

## Constrain low-level execution

- Prefer generated or verified bindings matching `<scsi/sg.h>` on supported architectures.
- Centralize `unsafe` SG_IO code and validate structure size, alignment, constants, pointer lifetimes, CDB length, buffer length, sense length, and residual.
- Keep command, data, and sense buffers alive for the entire synchronous ioctl.
- Use bounded timeouts and bounded blocking workers under Tokio.
- Do not hold shared async state while executing an ioctl.
- Use whole-device paths and revalidate identity after removal, reset, or remapping.

Never calculate a payload slice from `resid` until confirming it is nonnegative and no greater than the allocated length.

## Enforce an ATA command allowlist

Expose high-level operations rather than arbitrary taskfiles or CDBs. Begin with only:

- IDENTIFY DEVICE;
- SMART READ DATA;
- SMART READ THRESHOLDS when advertised/implemented;
- SMART RETURN STATUS;
- selected read-only SMART READ LOG or READ LOG EXT pages after capability discovery.

Reject every data-out direction and unknown command. Do not automate SECURITY ERASE, SANITIZE, FORMAT, DOWNLOAD MICROCODE, SET FEATURES, WRITE commands, SMART ENABLE/DISABLE, SMART SAVE, or SMART EXECUTE OFF-LINE IMMEDIATE.

Treat self-test initiation as a separately authorized maintenance action, not monitoring. Treat vendor-specific USB/controller passthrough as separate elevated-risk backends requiring hardware qualification.

## Build SAT commands deliberately

- Select ATA PASS-THROUGH(12) or (16) from the ATA command width and verified transport capability; use the 16-byte form for 48-bit commands.
- Set protocol, transfer direction, block/byte mode, transfer-length source, EXTEND, CK_COND, feature, sector count, LBA, device, and command fields from a typed request.
- Encode SMART magic register values only for commands that require them.
- Request ATA return registers when needed and decode descriptor-format sense descriptor `0x09` safely.
- Recognize that CHECK CONDITION with RECOVERED ERROR can carry a successful ATA return descriptor when CK_COND is used; evaluate ATA status/error registers before classifying failure.
- Do not treat missing return descriptors as success when the operation requires returned registers.

Keep separate builders for non-data, PIO data-in, and 48-bit log commands rather than a caller-controlled generic bitfield API.

## Parse IDENTIFY defensively

- Require the expected 512-byte response before interpreting 256 little-endian words.
- Decode ATA string fields by swapping bytes within each 16-bit word, then trim padding without discarding meaningful internal spaces.
- Validate command-set validity bits before trusting supported/enabled words.
- Prefer 48-bit capacity only when reported valid; use checked multiplication by the derived logical-sector size.
- Parse sector-size words according to their validity flags and retain unknown values.
- Treat rotation rate `0x0001` as non-rotating and reserved/sentinel values as unknown.
- Preserve WWN and raw identity for stable mapping, but avoid logging serials unnecessarily.

Do not reuse diskstats' fixed 512-byte sector rule for ATA capacity or vendor counters unless the specific field defines 512-byte units.

## Parse SMART without false standardization

For SMART READ DATA:

- Validate total length and checksum when the response format supplies one.
- Parse up to the defined number of 12-byte entries with checked offsets.
- Retain ID, flags, normalized current/worst values, and the raw six bytes.
- Match thresholds by attribute ID, not array position.
- Treat invalid normalized values and absent/obsolete threshold data explicitly.
- Use SMART RETURN STATUS as a separate device-level signal.

ATA SMART attribute IDs and raw encodings are generally vendor/model-specific. Common IDs such as 5, 9, 194, 197, 198, 199, 241, or 242 are industry conventions, not a universal schema. Never label a raw value as sectors, hours, Celsius, LBAs, NAND writes, or remaining life without a matched and sourced vendor/model rule.

The term “GPL” in ATA specifications means **General Purpose Logging**, not a software license.

## Interpret SSD endurance through sourced schemas

- Match rules by model family and firmware constraints, not vendor name alone.
- Keep normalized value, raw bytes, interpreted value, unit, direction, confidence, and source provenance.
- Do not search a list of possible endurance IDs and accept the first one blindly.
- Keep host writes distinct from NAND writes, and define the unit multiplier before converting to bytes.
- Derive remaining life or percentage used only when the vendor schema defines direction and saturation.
- Compute write amplification only when both numerator and denominator have documented meanings and compatible units.
- Fall back to raw SMART plus Unknown interpretation when no trusted schema matches.

Design the schema database independently. Do not copy smartmontools `drivedb.h` or GPL implementation code into an MIT-licensed project.

## Handle USB bridges and RAID honestly

- Start with standard inquiry/topology and typed SAT commands.
- Use bounded APT-16/APT-12 fallback only when the bridge family is known to support it safely.
- Never send vendor opcodes merely to guess a bridge family.
- Identify quirks by verified VID:PID, firmware, topology, and qualified behavior.
- Limit retries to avoid USB reset storms and distinguish asleep, removed, rejected, timed out, malformed, and unsupported devices.
- For hardware RAID logical volumes, report physical-disk SMART unavailable unless a controller-specific backend can address a verified physical slot.

Do not infer health of hidden physical disks from the health of a logical RAID volume.

## Decode completion in layers

Evaluate:

1. ioctl result and `errno`;
2. SG host and driver status;
3. SCSI status and bounded sense data;
4. ATA return descriptor presence and validity;
5. ATA BSY/DF/ERR status and error register;
6. SMART RETURN STATUS signature when requested;
7. payload structure and checksum.

Retry only bounded, classified transient outcomes. Do not hide ATA errors, permission failures, bridge rejection, medium errors, or malformed responses behind generic “SMART unavailable”.

## Isolate privileges

- Avoid granting `CAP_SYS_RAWIO` to the TUI or network-facing process.
- Put privileged execution in a small helper with fixed high-level requests, a device allowlist, peer authentication, length/time limits, and a hardcoded command allowlist.
- Construct every CDB inside the helper; never accept raw CDB/taskfile bytes over IPC.
- Return structured results rather than unrestricted raw transport access.
- Open only whole devices authorized by stable identity and confirm the path is not a partition.

## Test before hardware rollout

Create fixtures and tests for:

- ATA string swapping, validity bits, capacity, sector sizes, SSD/rotation, and identity truncation;
- valid, empty, duplicated, malformed, checksum-failing, and reordered SMART entries;
- thresholds matched by ID and unsupported threshold commands;
- SMART RETURN STATUS pass/fail/unknown signatures;
- ATA return descriptors in fixed/descriptor sense and missing/truncated descriptors;
- positive, zero, negative, and oversized residual values;
- permission denial, timeout, reset, removal, bridge rejection, and controller-hidden devices;
- allowlist rejection for every unsafe ATA command and data-out direction;
- vendor schemas with exact matching, unit conversion, firmware mismatch, and unknown-model fallback.

Fuzz IDENTIFY, SMART pages, log pages, sense data, ATA descriptors, and schema matching. Ensure malformed input never panics or yields a healthy conclusion.

Qualify real hardware across SATA HDD, consumer and enterprise SATA SSD, native AHCI, SAS HBA/SAT, representative USB bridges, and explicitly supported RAID controllers. Use smartctl, hdparm, and sg3_utils only as development oracles.

## Review checklist

Before completing work, confirm:

- Public APIs cannot submit arbitrary CDBs, ATA taskfiles, or data-out commands.
- SG_IO ABI and SAT CDB layouts are verified against primary sources.
- All offsets, lengths, residuals, capacity arithmetic, and checksums are validated.
- SMART raw values remain uninterpreted without a sourced model schema.
- Unsupported or malformed data cannot become zero, passed, or healthy.
- Bridge and controller fallbacks are explicit and bounded.
- Blocking ioctls cannot stall async execution or hold shared locks.
- Device identity is revalidated after hot-plug/reset.
- Privileged execution is isolated and uses a fixed allowlist.
- GPL reference material was not copied into permissively licensed code.
- Fixture, fuzz, integration, clippy, and hardware smoke tests pass.
