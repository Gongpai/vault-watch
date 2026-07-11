# US-MON-33 — Native SAS/SCSI Health Backend

**Sprint:** 10D | **Priority:** Should | **Status:** 📋 Planned

สร้าง SG_IO backend สำหรับ read-only INQUIRY/VPD/LOG SENSE โดยไม่เปิด arbitrary CDB surface

## Acceptance Criteria

1. typed command allowlist และ reject data-to-device/vendor/destructive commands
2. SG_IO ABI, buffer lifetime, residual และ sense bounds ถูกตรวจ
3. fixed/descriptor sense และ retry policy bounded
4. unsupported page ไม่กลายเป็น healthy
5. TUI ไม่ถือ raw-I/O capability; helper design ใช้ US-MON-37
6. binary fixtures/fuzzing และ SAS/SAT/controller-hidden hardware matrix ผ่าน
