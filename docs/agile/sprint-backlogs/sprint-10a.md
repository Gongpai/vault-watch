# Sprint 10A — Privacy & Graph Foundation

**Status:** 🚧 In Progress | **Stories:** US-MON-28, US-MON-29

## Outcome

สร้าง security/privacy contract และ sysfs-only storage inventory ก่อน protocol commands

## Tasks

- [x] privacy/network/legacy disclosure บน TUI
- [x] config error ไม่ถูกกลืน; empty Discord table regression test
- [x] reject command/device/webhook injection จาก config
- [x] initial whole-block classification สำหรับ SCSI-like/NVMe/MMC/MD/DM/virtual
- [x] typed graph edges, scoped identities, confidence และ generation model
- [x] fixture-root discovery tests + no-device/partial state
- [x] periodic topology reconciliation with atomic publish and failed-empty snapshot retention
- [x] BUG-01/04 fixture: whole-device, partition และ virtual counts แยก scope ชัดเจน
- [x] live hot-add/hot-remove verification: counts/rows reconcile without restart or crash (sanitized evidence 2026-07-11)

## Exit Gate

ไม่มี device node/raw command access และ threat-model controls ของ 10A ผ่าน tests/clippy
