# US-MON-32 — Storage-first TUI & Scoped Metrics

**Sprint:** 10C | **Priority:** Must | **Status:** 📋 Planned

ออกแบบ UI ใหม่ให้แสดง storage topology, protocol, scope, source, availability และ security posture แทน UI ที่สมมติว่าทุกอย่างเป็น SAS HDD

## Acceptance Criteria

1. Overview แสดง node type/protocol/exposure/health availability และไม่ใช้คำว่า disk กับทุก node
2. privacy bar แสดง content/network/legacy/privileged state ตลอดเวลา
3. unsupported, hidden, asleep, permission denied, stale และ malformed แยกจาก healthy
4. metric ทุกค่ามี source/scope และ UI เตือนว่า stacked counters ไม่ additive
5. no-device/partial inventory มี empty state ที่ชัดเจน
6. graph theme config Part B ถูก integrate แบบ validated config
7. responsive/focus/scroll/legend ผ่าน fixture UI tests และ hardware verification
