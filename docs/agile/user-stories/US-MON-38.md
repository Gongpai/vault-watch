# US-MON-38 — Carry-over, Security Review & Hardware Qualification

**Sprint:** 10H | **Priority:** Must for release | **Status:** 📋 Planned

รวมงานที่ยังไม่เคย verify จริงและ release gates เพื่อไม่ให้เอกสารประกาศ Done ก่อนมีหลักฐาน

## Acceptance Criteria

1. device discovery/no-device/active list verified บนหลาย topology
2. live MD rebuild/multi-array legends และ graph label screenshot verified
3. config/theme/distro helper tests ครบ
4. musl static binary รันบน Alpine 3.19+ และ size target ถูกบันทึก
5. protocol hardware matrix ระบุ pass/fail/unsupported ต่อรุ่น/firmware
6. threat model, dependency/license audit, fuzz results และ security review ผ่าน
7. docs/Cargo version/changelog/status sync กับผลจริง
8. Intel DC P4618 6.4 TB ผ่าน qualification แบบหนึ่ง PCIe card/สอง NVMe controllers โดยตรวจ health, temperature, endurance และ errors แยก controller
9. real-hardware regression ใน [BUG-2026-07-11](../bug-fixes/BUG-2026-07-11-real-storage-inventory.md) ถูกปิดพร้อมหลักฐาน retest
