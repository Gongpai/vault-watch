# US-MON-35 — Native NVMe Health Backend

**Sprint:** 10F | **Priority:** Should | **Status:** 📋 Planned

สร้าง NVMe Identify/Get Log Page backend ที่แยก subsystem/controller/path/namespace/endurance scope

## Acceptance Criteria

1. ioctl ABI generated/verified; positive ioctl status ไม่สับสนกับ command result
2. admin-command allowlist ปิด format/sanitize/firmware/reset/namespace/security/data-out
3. u128 counters, Kelvin และ Data Unit rounding ถูกต้อง
4. identity เป็น subsystem+namespace composite; multipath ไม่ sum counters
5. userspace ไม่ส่ง AER แข่ง kernel; event trigger reconciliation
6. fixtures/fuzzing/QEMU และ consumer/enterprise/NVMe-oF hardware matrix ผ่าน
