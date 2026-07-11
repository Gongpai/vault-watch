# BUG-2026-07-11 — Real Storage Inventory and Intel DC P4618

**Reported:** 2026-07-11 | **Status:** 🐛 Open | **Source:** real-hardware TUI screenshot + `lsblk`

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

**Severity:** High | **Owner:** US-MON-29 / US-MON-32

- Observed: Privacy bar displays `NVMe 8`.
- Actual: two NVMe controllers/whole namespaces are visible; the remaining six entries are partitions.
- Expected: UI must label and count controllers, namespaces, whole block devices and partitions separately. A partition must never increase the physical/controller count.

### BUG-02 — New inventory is not the source of the Disk Summary

**Severity:** High | **Owner:** US-MON-32

- Observed: Disk Summary contains only `sda`, `sdb`, `sdc`.
- Actual cause: legacy device selection still includes only `sd*`; discovered NVMe nodes are not routed into collectors/UI rows.
- Expected: eligible whole-device subjects from the topology graph drive the UI. NVMe must remain visible even while health is `Unsupported`, `PermissionDenied` or not implemented.

### BUG-03 — Collection failure is rendered as disk failure and zero temperature

**Severity:** Critical | **Owner:** US-MON-32 / US-MON-33 / US-MON-34

- Observed: `sda` and `sdb` produce red `SMART health FAIL`; all three disks show `0°C` while detailed fields are `--`.
- Risk: a missing tool, wrong protocol option, permission denial, parse error or unsupported attribute can trigger a false device-failure alert.
- Expected: unavailable health/temperature must be typed (`Unsupported`, `PermissionDenied`, `ToolUnavailable`, `Malformed`, `TemporarilyUnavailable`) and rendered as `N/A`/`UNKNOWN`, never `FAIL` or `0°C`. Alerts must require an affirmative device-health failure result.

### BUG-04 — Block-node total is not an operator-meaningful device count

**Severity:** Medium | **Owner:** US-MON-29 / US-MON-32

- Observed: Privacy bar displays `block nodes 47`, dominated by loop devices and partitions.
- Expected: retain the raw graph-node count for diagnostics, but present operator-facing counts by scope: physical candidates, controller/logical device, whole block device, partition, stacked, virtual and hidden/unsupported.

### BUG-05 — P4618 needs card grouping without merging controller health

**Severity:** High | **Owner:** US-MON-29 / US-MON-35 / US-MON-38

- Observed topology: one P4618 card appears as two approximately 3.2 TB NVMe devices.
- Expected: topology may group both controllers under one PCIe/card placement object, but must preserve separate controller identity, generation, SMART/health, temperature, endurance, media errors and namespace metrics. Never average or silently merge the two health records.
- RAID note: grouping the card is not an MD RAID relationship. If the two sides are combined by MD/DM, that relationship must appear as a separate logical layer.

## Regression and Hardware Acceptance

- [ ] Fixture: two NVMe whole devices with six partitions reports `controllers=2`, `partitions=6`, never `NVMe=8 drives`.
- [ ] Fixture: 32 loop devices do not inflate operator-facing physical storage totals.
- [ ] Disk Summary includes both P4618 controller/namespace subjects before native health exists and labels health availability honestly.
- [ ] Permission denied, missing external tool and malformed output never emit `SMART FAIL`, `0°C` or a critical disk alert.
- [ ] Native NVMe health is collected and displayed independently for `/dev/nvme0` and `/dev/nvme1`.
- [ ] UI groups both controllers as one P4618 card only when PCIe/topology evidence supports that relationship.
- [ ] Manual retest captures TUI screenshot plus redacted controller model/firmware and SMART/Health output for both controllers.

## Security Constraint

The fixes may read kernel metadata and allowlisted health/admin-log structures only. They must not read namespace data, mounted filesystem contents or arbitrary user-selected paths, and must not add format/sanitize/firmware/reset/vendor-command access.
