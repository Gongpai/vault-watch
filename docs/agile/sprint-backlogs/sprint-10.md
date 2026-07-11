# Sprint 10 — Universal Storage Architecture & Security Hardening

**Version target:** 1.0 architecture migration | **Status:** 🚧 In Progress | **Started:** 2026-07-11

## Sprint Goal

เปลี่ยน VaultWatch จาก HDD/SAS-specific monitor ไปเป็น Linux storage monitor ที่รองรับ topology หลายชั้น โดยเริ่มจาก read-only kernel metadata, แยก backend ตาม protocol, เปิดเผย privacy/security posture ต่อผู้ใช้ และห้าม raw user-data access หรือ arbitrary privileged commands โดย design

## Non-negotiable Security Boundary

- โปรแกรม monitoring ห้ามอ่านไฟล์ของผู้ใช้, filesystem contents หรือ raw data sectors
- network egress ปิดโดย default; Discord ทำงานเฉพาะเมื่อมี webhook URL ที่ตั้งค่าอย่างชัดเจน
- TUI ห้ามถือ `CAP_SYS_RAWIO`/`CAP_SYS_ADMIN`
- future privileged helper รับเฉพาะ typed allowlisted requests; ห้ามรับ raw CDB/taskfile/CDW/ioctl
- format, sanitize, firmware, security, reset, namespace management, write/data-out และ vendor opcode เป็น default-deny
- unsupported/hidden/permission-denied ต้องไม่ถูกตีความเป็น healthy หรือ zero

## Sub-sprints

| Sub-sprint | Skill/Source | Stories | Status |
|:---|:---|:---|:---|
| [**10A — Privacy & Graph Foundation**](sprint-10a.md) | `$universal-linux-storage-monitoring-rust` | [US-MON-28](../user-stories/US-MON-28.md), [US-MON-29](../user-stories/US-MON-29.md) | 🚧 In Progress |
| [**10B — Native Counters & MD RAID**](sprint-10b.md) | `$linux-disk-monitoring-rust`, `$linux-md-raid-monitoring-rust` | [US-MON-30](../user-stories/US-MON-30.md), [US-MON-31](../user-stories/US-MON-31.md) | 📋 Planned |
| [**10C — Storage-first TUI**](sprint-10c.md) | universal metric/scope model | [US-MON-32](../user-stories/US-MON-32.md) | 📋 Planned |
| [**10D — Native SAS/SCSI**](sprint-10d.md) | `$native-sas-scsi-health-rust` | [US-MON-33](../user-stories/US-MON-33.md) | 📋 Planned |
| [**10E — Native SATA/ATA**](sprint-10e.md) | `$native-sata-ata-health-rust` | [US-MON-34](../user-stories/US-MON-34.md) | 📋 Planned |
| [**10F — Native NVMe**](sprint-10f.md) | `$native-nvme-health-rust` | [US-MON-35](../user-stories/US-MON-35.md) | 📋 Planned |
| [**10G — USB/Removable/MMC**](sprint-10g.md) | `$linux-usb-removable-storage-rust` | [US-MON-36](../user-stories/US-MON-36.md) | 📋 Planned |
| [**10H — Privilege Broker & Qualification**](sprint-10h.md) | all storage skills | [US-MON-37](../user-stories/US-MON-37.md), [US-MON-38](../user-stories/US-MON-38.md) | 📋 Planned |

## Carry-over Moved from Earlier Sprints

| Previous item | New owner |
|:---|:---|
| Sprint 05: verify device discovery on different hardware; no-device warning | US-MON-29 / US-MON-32 |
| Sprint 06: real-device legends and live multi-array rebuild verification | US-MON-31 / US-MON-38 |
| Sprint 06: config helper tests | US-MON-28 / US-MON-38 |
| Sprint 09: visual verification of Y label offset | US-MON-38 |
| US-MON-16: Alpine static binary and size verification | US-MON-38 |
| US-MON-26 Part B: config-driven graph theme | US-MON-32 / US-MON-38 |

## Open Bug Fixes from Real Hardware

- [BUG-2026-07-11 — Real Storage Inventory and Intel DC P4618](../bug-fixes/BUG-2026-07-11-real-storage-inventory.md)
  - incorrect NVMe/partition and block-node counts
  - NVMe inventory not connected to Disk Summary
  - unavailable legacy SMART incorrectly rendered as `FAIL`/`0°C`
  - one-card/two-controller P4618 topology and per-controller health qualification

## Definition of Done for Sprint 10 Umbrella

- [ ] graph-first inventory รองรับ block/partition/MD/DM/NVMe/MMC/SCSI-like/virtual โดยไม่อ้างว่า block node ทุกตัวคือ physical disk
- [ ] `iostat` และ `/proc/mdstat` ถูกแทนด้วย native stable kernel interfaces; legacy fallback ถูก label และ opt-in/temporary
- [ ] protocol backends มี typed read-only allowlists และ binary fixture tests
- [ ] UI แสดง source, scope, availability, confidence และ privacy/network posture
- [ ] privileged broker แยก process พร้อม peer auth, device identity binding, audit และ default-deny
- [ ] ไม่มี API สำหรับ raw user-content access หรือ arbitrary privileged command
- [ ] threat model, security review, fuzzing และ hardware qualification matrix ผ่านตาม story
- [ ] build/test/clippy สะอาด และ docs/status ไม่ประกาศ Done ก่อน real verification

## Implementation Order

ทำ 10A → 10B → 10C ก่อน raw protocol backends จากนั้นทำ 10D/10E/10F แบบ feature-gated, ต่อด้วย 10G และปิดท้าย 10H ห้ามข้าม security gates เพื่อเร่ง hardware support
