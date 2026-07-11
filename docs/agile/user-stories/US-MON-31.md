# US-MON-31 — Native Linux MD RAID Backend

**Sprint:** 10B | **Priority:** Must | **Status:** 🚧 In Progress

แทน `/proc/mdstat` parser ด้วย read-only MD sysfs snapshots ที่รักษาความต่างระหว่าง recover/resync/check/repair/reshape

## Acceptance Criteria

1. enumerate array/member จาก sysfs โดยไม่ assume ชื่อ `md0`
2. parse array/member state แบบ unknown-safe; malformed/missing ไม่กลายเป็น healthy
3. snapshot ตรวจ state/action ก่อนและหลังเพื่อรับมือ race
4. progress/speed/ETA reset เมื่อ operation/topology เปลี่ยน
5. external metadata/`mdmon` เป็น read-only และ explicit
6. fixture tests + parallel comparison กับ legacy parser ก่อน cutover
7. real multi-array/rebuild verification ถูกบันทึกใน US-MON-38

## Implementation Progress

- [x] AC1–AC3 core: injectable block-class root, md-directory enumeration without name assumptions, unknown-safe array/action/member states, pre/post state+action consistency boundary and bounded retry
- [x] AC4 partial: typed progress, kernel speed and checked ETA; delta-speed operation cache remains
- [x] AC5: `external:*` metadata is explicit and backend is read-only
- [x] AC6 partial: healthy/member/recovery/external/malformed fixtures; sysfs runs in shadow while `/proc/mdstat` remains primary oracle
- [x] targeted MD sysfs fixture suite verified by operator (4/4, sanitized evidence 2026-07-11)
- [ ] state-transition/delta fixtures, semantic oracle comparison, sysfs cutover and live multi-array/rebuild qualification
