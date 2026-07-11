# BUG-2026-07-11 — Real Storage Inventory and Intel DC P4618

**Reported:** 2026-07-11 | **Status:** 🚧 BUG-01–04/08–11 hardware-verified; BUG-05–07 open | **Source:** sanitized real-hardware observations

## Privacy Handling

- Do not commit raw command output, serial numbers, WWNs, hostnames, usernames, mount paths or unique device identifiers.
- Store only protocol/classification behavior and pass/fail acceptance evidence.
- Use synthetic identifiers and counters in fixtures.
- The raw pasted diagnostic file remains outside this repository and is not a test fixture or documentation source.

## Test Hardware

- Intel/Solidigm DC P4618 6.4 TB PCIe x8 card
  - one physical add-in card
  - two independent NVMe controllers/drives, approximately 3.2 TB each
  - Linux exposure: `nvme0n1` and `nvme1n1`
- three SCSI-like block disks: `sda`, `sdb`, `sdc`
  - reported deployment: two SATA SSDs and one HDD
- Snap loop devices and partitions are present

## Observed Bugs

### BUG-01 — NVMe summary counts namespaces/partitions as drives

**Severity:** High | **Owner:** US-MON-29 / US-MON-32 | **Status:** ✅ Hardware verified

- Observed: Privacy bar displays `NVMe 8`.
- Actual: two NVMe controllers/whole namespaces are visible; the remaining six entries are partitions.
- Expected: UI must label and count controllers, namespaces, whole block devices and partitions separately. A partition must never increase the physical/controller count.

### BUG-02 — New inventory is not the source of the Disk Summary

**Severity:** High | **Owner:** US-MON-32 | **Status:** ✅ Hardware verified; unavailable NVMe rows until native backend

- Observed: Disk Summary contains only `sda`, `sdb`, `sdc`.
- Actual cause: legacy device selection still includes only `sd*`; discovered NVMe nodes are not routed into collectors/UI rows.
- Expected: eligible whole-device subjects from the topology graph drive the UI. NVMe must remain visible even while health is `Unsupported`, `PermissionDenied` or not implemented.

### BUG-03 — Collection failure is rendered as disk failure and zero temperature

**Severity:** Critical | **Owner:** US-MON-32 / US-MON-33 / US-MON-34 | **Status:** ✅ Hardware verified

- Observed: `sda` and `sdb` produce red `SMART health FAIL`; all three disks show `0°C` while detailed fields are `--`.
- Risk: a missing tool, wrong protocol option, permission denial, parse error or unsupported attribute can trigger a false device-failure alert.
- Expected: unavailable health/temperature must be typed (`Unsupported`, `PermissionDenied`, `ToolUnavailable`, `Malformed`, `TemporarilyUnavailable`) and rendered as `N/A`/`UNKNOWN`, never `FAIL` or `0°C`. Alerts must require an affirmative device-health failure result.

### BUG-04 — Block-node total is not an operator-meaningful device count

**Severity:** Medium | **Owner:** US-MON-29 / US-MON-32 | **Status:** ✅ Hardware verified

- Observed: Privacy bar displays `block nodes 47`, dominated by loop devices and partitions.
- Expected: retain the raw graph-node count for diagnostics, but present operator-facing counts by scope: physical candidates, controller/logical device, whole block device, partition, stacked, virtual and hidden/unsupported.

### BUG-05 — P4618 needs card grouping without merging controller health

**Severity:** High | **Owner:** US-MON-29 / US-MON-35 / US-MON-38 | **Status:** 🐛 Open

- Observed topology: one P4618 card appears as two approximately 3.2 TB NVMe devices.
- Expected: topology may group both controllers under one PCIe/card placement object, but must preserve separate controller identity, generation, SMART/health, temperature, endurance, media errors and namespace metrics. Never average or silently merge the two health records.
- RAID note: grouping the card is not an MD RAID relationship. If the two sides are combined by MD/DM, that relationship must appear as a separate logical layer.

### BUG-06 — Legacy SMART forces SCSI mode for ATA devices

**Severity:** High | **Owner:** US-MON-34 | **Status:** 🐛 Open

- Sanitized evidence: read-only protocol auto-detection returns ATA health and temperature on the same openSUSE host, so this is not an OS permission denial.
- Actual cause: the legacy collector always adds `-d scsi`, while its parser recognizes only SCSI-formatted health, temperature and hours.
- Expected: classify protocol before collection; ATA/SATA must use the ATA backend or an explicitly labelled auto-detect fallback. Unsupported/mismatched protocol remains `N/A`, never failed.

### BUG-07 — Native NVMe health is readable but not connected

**Severity:** High | **Owner:** US-MON-35 | **Status:** 🐛 Open

- Sanitized evidence: both controllers return standard read-only SMART/Health logs without critical or media-error indications.
- Expected: collect critical warning, composite temperature, spare, percentage used, read/write units, power state counters, unsafe shutdowns, media errors and error-log count independently per controller.
- Do not store captured raw values or persistent identifiers in repository fixtures.

### BUG-08 — NVMe is absent from throughput graphs

**Severity:** High | **Owner:** US-MON-30 / US-MON-32 | **Status:** ✅ Hardware verified

- Observed: table contains both NVMe namespaces as `N/A`, but graph legends/history contain only legacy `sd*` devices.
- Expected: native block counters and graph subjects come from eligible whole-device graph nodes, with explicit scope and no partition/stack double counting.

### BUG-09 — Long device names and privacy summary are clipped

**Severity:** Medium | **Owner:** US-MON-32 | **Status:** ✅ Hardware verified

- Observed: NVMe namespace names are truncated by the five-column Disk field; the privacy summary can lose trailing fields at the tested terminal width.
- Expected: device column accommodates common NVMe/MMC/DM names or safely ellipsizes with an unambiguous detail view; privacy/security state remains visible with responsive compact formatting.

### BUG-10 — Throughput unit label says MB/s for MiB/s values

**Severity:** Medium | **Owner:** US-MON-30 / US-MON-32 | **Status:** ✅ Hardware verified

- Actual: native formula divides by 1,048,576 bytes but table/graph labels said `MB/s`.
- Expected: label native throughput `MiB/s`; keep RAID rebuild units separately scoped to their source.

### BUG-11 — MD rebuild speed and ETA disappear after refresh

**Severity:** High | **Owner:** US-MON-31 | **Status:** ✅ Hardware verified

- Observed: initial native MD snapshot displays kernel rebuild speed/ETA, but a later refresh retains only progress percentage.
- Root cause: when two short-interval samples had identical `sync_completed`, the delta sampler replaced valid kernel `sync_speed`/derived ETA with unavailable values.
- Fix: unchanged progress retains kernel speed/ETA and the older delta baseline; once progress advances, delta speed spans the actual progress interval.
- Follow-up glitch: a block-event hint could create a sub-second sample immediately after startup; extrapolating its progress delta produced a one-frame multi-GiB/s spike.
- Follow-up fix: delta speed requires a minimum 2-second observation window. Short event-driven samples retain kernel speed/ETA and the prior baseline without clamping legitimate hardware values.
- Expected: speed/ETA remain visible throughout an active operation whenever kernel `sync_speed` is available; a repeated progress counter must not erase them.

## Regression and Hardware Acceptance

- [x] Fixture: two NVMe whole devices with six partitions reports `NVMe whole=2`, `partitions=6`, never `NVMe=8 drives`.
- [x] Fixture: 32 loop devices do not inflate operator-facing whole-storage totals.
- [x] Disk Summary includes both P4618 namespace subjects before native health exists and labels health availability honestly.
- [x] Permission denied, missing external tool, missing/ambiguous status and malformed output never emit `SMART FAIL`, `0°C` or a critical disk alert.
- [ ] Native NVMe health is collected and displayed independently for `/dev/nvme0` and `/dev/nvme1`.
- [ ] UI groups both controllers as one P4618 card only when PCIe/topology evidence supports that relationship.
- [x] Manual TUI retest verifies BUG-01–04 without storing raw identifiers.
- [x] Native throughput and removable-device add/read/remove behavior verified without storing raw identifiers.
- [x] Responsive device names, compact privacy counts and MiB/s labels verified without storing raw identifiers.
- [x] MD rebuild speed/ETA remain visible across repeated refreshes and no startup/refetch spike recurs on the live server.
- [ ] Sanitized hardware qualification captures protocol/model/firmware class and pass/fail fields without serial, WWN, host or mount metadata.

## Security Constraint

The fixes may read kernel metadata and allowlisted health/admin-log structures only. They must not read namespace data, mounted filesystem contents or arbitrary user-selected paths, and must not add format/sanitize/firmware/reset/vendor-command access.
