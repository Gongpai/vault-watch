# US-MON-32 — Storage-first TUI & Scoped Metrics

**Sprint:** 10C | **Priority:** Must | **Status:** 🚧 In Progress

ออกแบบ UI ใหม่ให้แสดง storage topology, protocol, scope, source, availability และ security posture แทน UI ที่สมมติว่าทุกอย่างเป็น SAS HDD

## Acceptance Criteria

1. Overview แสดง node type/protocol/exposure/health availability และไม่ใช้คำว่า disk กับทุก node
2. privacy bar แสดง content/network/legacy/privileged state ตลอดเวลา
3. unsupported, hidden, asleep, permission denied, stale และ malformed แยกจาก healthy
4. metric ทุกค่ามี source/scope และ UI เตือนว่า stacked counters ไม่ additive
5. no-device/partial inventory มี empty state ที่ชัดเจน
6. graph theme config Part B ถูก integrate แบบ validated config
7. responsive/focus/scroll/legend ผ่าน fixture UI tests และ hardware verification

## Implementation Progress

- [x] `t` toggles a graph-backed Topology Overview without replacing Table/Graph views
- [x] overview displays node locator, layer/materialization, protocol view, removability, confidence, generation presence and typed relation counts
- [x] source/availability banner distinguishes `AVAILABLE`, `PARTIAL` and `EMPTY` and warns that stacked counters are not additive
- [x] topology rows never render identity claim values, `dev_t` values or `diskseq` values
- [x] shared scrollbar offset semantics and per-frame panel hitbox reset (BUG-12 fixtures)
- [x] live mouse-wheel and scrollbar endpoint qualification across Topology/Table/Device Details (sanitized evidence 2026-07-11)
- [x] selected-node detail panel exposes health availability/source/scope, topology confidence, generation presence and relation counts without identity values
- [x] typed health availability taxonomy: `Available`, `Unsupported`, `Hidden`, `PermissionDenied`, `Asleep`, `TemporarilyUnavailable`, `Stale`, `Malformed`, `DeviceGone`
- [x] collector diagnostics and topology context preserve availability reason without creating health-failure alerts
- [x] live all-device verification: unavailable reasons produce no false `FAIL` alert (sanitized evidence 2026-07-11)
- [x] validated graph-theme config Part B: line colors, temperature zones, I/O background and label offset
- [x] topology overview/selection/detail and scroll hardware qualification (sanitized evidence 2026-07-11)
