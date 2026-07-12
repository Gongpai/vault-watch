# US-MON-33 — Native SAS/SCSI Health Backend

**Sprint:** 10D | **Priority:** Should | **Status:** 🚧 In Progress

สร้าง SG_IO backend สำหรับ read-only INQUIRY/VPD/LOG SENSE โดยไม่เปิด arbitrary CDB surface

## Acceptance Criteria

1. typed command allowlist และ reject data-to-device/vendor/destructive commands
2. SG_IO ABI, buffer lifetime, residual และ sense bounds ถูกตรวจ
3. fixed/descriptor sense และ retry policy bounded
4. unsupported page ไม่กลายเป็น healthy
5. TUI ไม่ถือ raw-I/O capability; helper design ใช้ US-MON-37
6. binary fixtures/fuzzing และ SAS/SAT/controller-hidden hardware matrix ผ่าน

## Implementation Progress

- [x] pure typed command foundation exposes only TEST UNIT READY, standard/selected VPD INQUIRY and selected LOG SENSE with `None`/`FromDevice` directions
- [x] bounds-checked standard INQUIRY, supported-VPD, temperature LOG SENSE and fixed/descriptor sense parsers with synthetic identity-free fixtures
- [x] malformed/truncated response and `0xff` unavailable-temperature cases remain explicit and never become healthy zero
- [x] VPD 0x83 descriptor scope, B1 rotation, supported/error/non-medium/informational-exception LOG pages and bounded sense-action policy
- [x] exhaustive truncated-prefix fixture tests reject partial VPD/log records without panic
- [x] operator reran initial pure SCSI suite successfully (6/6, sanitized evidence 2026-07-12)
- [x] operator reran expanded parser/sense suite successfully (12/12, sanitized evidence 2026-07-12)
- [x] untyped raw, data-out and vendor commands are rejected; only advertised VPD/LOG pages can be scheduled
- [x] pure routing distinguishes native SCSI, SAT, controller-hidden, ambiguous SG mapping, missing evidence and unsupported peripheral types
- [x] completion validator bounds residual/sense lengths and preserves ioctl/host/driver/SCSI status failures before payload parsing
- [ ] standalone fuzz targets and remaining optional VPD/log pages
- [ ] SG_IO ABI/transport, broker integration and hardware qualification
