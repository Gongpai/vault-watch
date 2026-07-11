# Sprint 10C — Storage-first TUI

**Status:** 📋 Planned | **Story:** US-MON-32

## Tasks

- [ ] topology overview + detail views ตาม node/scope
- [ ] availability/confidence/source labels
- [ ] empty/partial/hidden/unsupported/asleep states
- [ ] scoped graphs และ stacked-counter warning
- [ ] validated graph-theme config Part B
- [ ] responsive/focus/scroll regression tests
- [x] BUG-01/02/04: scoped device counts และใช้ graph inventory เป็น source ของ Disk Summary
- [x] BUG-03: unavailable/permission/parser errors แสดง `UNKNOWN`/`N/A` และห้ามสร้าง health alert
- [x] BUG-08: graph/history ใช้ eligible subjects จาก inventory รวม NVMe
- [x] BUG-09: 12-column device names และ compact privacy summary สำหรับ terminal <150 columns
- [x] BUG-10: native throughput labels ใช้ `MiB/s` ตรงกับ binary-unit formula
- [x] BUG-09/10 live responsive-layout verification (sanitized evidence 2026-07-11)

## Exit Gate

UI ไม่สมมติว่าทุก node เป็น HDD และแสดง privacy/privilege/network state ตลอดเวลา
