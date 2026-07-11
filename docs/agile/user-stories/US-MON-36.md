# US-MON-36 — USB, Removable, SD & eMMC Backends

**Sprint:** 10G | **Priority:** Should | **Status:** 📋 Planned

รองรับ topology/power lifecycle ของ USB/removable storage และอ่าน eMMC health แบบ type-safe โดยไม่ destabilize bridge

## Acceptance Criteria

1. sysfs traversal ตาม relationships ไม่ใช้ fixed parent count
2. BOT/UAS/SAT/SNT/MMC routing เป็น capability-first; unknown bridge ไม่รับ vendor probe
3. NoWake policy ข้าม suspended devices โดยไม่ mark failed
4. eMMC/SD ถูกแยกก่อน CMD8; MMC data-out/SWITCH/sanitize ถูก reject
5. probe budget/quarantine ป้องกัน USB reset storms
6. hot-remove/reconnect/device-name reuse ไม่ใช้ cache ผิดอุปกรณ์
7. bridge/eMMC fixtures และ hardware qualification ผ่าน
