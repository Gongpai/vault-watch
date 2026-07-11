# US-MON-29 — Universal Storage Inventory Graph

**Sprint:** 10A | **Priority:** Must | **Status:** ✅ Done

ในฐานะผู้ดูแลระบบ ฉันต้องการ inventory ที่แยก block node, partition, array, logical layer, controller/path และ physical candidate เพื่อให้ข้อมูลไม่ซ้ำชั้นและไม่ระบุอุปกรณ์ผิดประเภท

## Acceptance Criteria

1. initial discovery อ่าน `/sys/class/block` แบบ read-only และ inject root สำหรับ fixtures ได้
2. classification แยก SCSI-like, NVMe, MMC, MD, DM, virtual และ unknown โดยไม่ claim physical media จากชื่ออย่างเดียว
3. topology model รองรับ typed edges, multipath/fan-in/fan-out และ cycle-safe traversal
4. identity claim มี scope/source/confidence และ generation (`diskseq` เมื่อมี)
5. no-device/partial-inventory แสดงชัดเจน ไม่ crash
6. hot-plug event เป็น hint และต้อง periodic reconciliation
7. ไม่มีการเปิด device node หรือ filesystem content ใน discovery phase

## Implementation Progress

- [x] AC1–AC5, AC7: sysfs fixture root, independent node classification, directed typed graph, cycle-safe traversal, scoped identity claims, `diskseq`/`dev_t` generation และ partial/empty state
- [x] AC6 (periodic path): bounded sysfs resnapshot + atomic topology reconciliation
- [x] AC6 periodic path verified on live removable storage add/remove without retaining device identifiers
- [x] AC6 event core: read-only `NETLINK_KOBJECT_UEVENT` block hints are filtered and coalesced before waking the same transactional sysfs resnapshot; socket/read failure silently retains periodic correctness fallback
- [x] event parser and burst coalescing fixtures; a burst emits one reconciliation hint
- [x] live event-assisted add/remove qualification: removable storage, whole/removable/node counts and rows reconcile immediately on add/remove before the periodic deadline without restart/crash (sanitized operator evidence 2026-07-11)
