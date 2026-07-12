# US-MON-34 — Native SATA/ATA SMART Backend

**Sprint:** 10E | **Priority:** Should | **Status:** 🚧 In Progress

สร้าง typed SAT ATA PASS-THROUGH backend สำหรับ IDENTIFY/SMART โดยแยก vendor interpretation และไม่รับ arbitrary taskfile

## Acceptance Criteria

1. allowlist เฉพาะ read-only IDENTIFY/SMART/log operations ที่ผ่าน capability gate
2. ATA return descriptor/status/checksum/length ถูก validate
3. SMART raw attributes ไม่ถูกตีความโดยไม่มี sourced model schema
4. USB/controller fallback explicit และ bounded; unknown bridge ไม่รับ vendor probe
5. privilege helper ใช้ typed requests เท่านั้น
6. vendor fixtures/fuzzing และ hardware qualification ผ่าน

## Implementation Progress

- [x] typed APT-16 builders expose only IDENTIFY, SMART READ DATA, SMART READ THRESHOLDS and SMART RETURN STATUS
- [x] pure 512-byte IDENTIFY parser decodes ATA strings, 28/48-bit capacity, SMART support and rotation/SSD state
- [x] SMART parser validates checksum, preserves six raw vendor bytes, keeps invalid normalized values unavailable and matches thresholds by ID
- [x] descriptor-sense ATA return registers and SMART pass/fail/unknown signatures are bounds checked
- [x] truncated/missing/bad-checksum responses never become success
- [ ] sector-size/capability details, logs, vendor schemas, routing, fuzzing, broker and hardware qualification
