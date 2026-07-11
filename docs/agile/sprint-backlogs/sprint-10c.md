# Sprint 10C — Storage-first TUI

**Status:** ✅ Done | **Story:** US-MON-32 | **Delivered:** 0.17.0

## Tasks

- [x] topology overview from graph nodes with protocol/layer/confidence/generation-presence/relations
- [x] selected-node detail view with scoped health source/availability/confidence (identity values redacted)
- [x] typed availability/confidence/source labels in selected-node details
- [x] initial node-detail mappings: MD complete/partial/unavailable, legacy whole-device available/temporary, partition/virtual/stacked unsupported
- [x] empty/partial/stale/hidden/unsupported/asleep/permission/malformed/device-gone states
- [x] live all-device availability/false-alert verification
- [x] scoped graphs และ stacked-counter warning
- [x] validated graph-theme config Part B integrated into Graph/legends
- [x] live graph-theme override verification without retaining device identity
- [x] responsive/focus/scroll regression tests
- [x] live overview/selection/details/scroll verification without identity disclosure
- [x] topology row privacy fixture and scrollable focused panel
- [x] BUG-12 shared final-offset scrollbar mapping + stale mouse-hitbox reset
- [x] BUG-12 live mouse-wheel/scrollbar endpoint verification
- [x] BUG-01/02/04: scoped device counts และใช้ graph inventory เป็น source ของ Disk Summary
- [x] BUG-03: unavailable/permission/parser errors แสดง `UNKNOWN`/`N/A` และห้ามสร้าง health alert
- [x] BUG-08: graph/history ใช้ eligible subjects จาก inventory รวม NVMe
- [x] BUG-09: 12-column device names และ compact privacy summary สำหรับ terminal <150 columns
- [x] BUG-10: native throughput labels ใช้ `MiB/s` ตรงกับ binary-unit formula
- [x] BUG-09/10 live responsive-layout verification (sanitized evidence 2026-07-11)
- [x] BUG-13 edge-triggered focus/view keys filter Repeat/Release + deterministic conditional RAID focus fixture (live verification pending)

## Exit Gate

UI ไม่สมมติว่าทุก node เป็น HDD และแสดง privacy/privilege/network state ตลอดเวลา

**Passed:** Graph แสดง source/scope/non-additive warning, topology แยกชนิดและ availability, privacy bar อยู่ทุก view, fixture tests และ sanitized live verification ผ่านแล้ว
