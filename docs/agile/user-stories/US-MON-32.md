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
- [ ] node detail view and full health availability taxonomy
- [ ] validated graph-theme config Part B
- [ ] responsive/focus/scroll hardware qualification
