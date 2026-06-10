# User Story: US-MON-01 — RAID Status Parser

**Status:** 🚧 Planned
**Sprint:** [Sprint 01](../sprint-backlogs/sprint-01.md)
**Epic:** [Must Have — MVP Scope](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบ Ubuntu Server ที่รัน mdadm RAID10
**ฉันต้องการ** ให้โปรแกรมอ่านและ parse สถานะ RAID จาก `/proc/mdstat` อัตโนมัติ
**เพื่อให้** ได้ข้อมูล rebuild progress %, speed (MB/s) และ ETA ที่พร้อมนำไปแสดงบน UI

---

## ✅ Acceptance Criteria

1. [ ] Parse array name (เช่น `md0`) และ RAID type (เช่น `raid10`)
2. [ ] Parse active/total disk count (เช่น `[3/3]`)
3. [ ] Parse state: `Active` (ไม่มี rebuild line), `Rebuilding` (มี rebuild line), `Degraded` (disk count ไม่ครบ)
4. [ ] Parse rebuild percentage (เช่น `9.3`)
5. [ ] Parse rebuild speed จาก `speed=XXXXK/sec` และแปลงเป็น MB/s
6. [ ] Parse ETA จาก `finish=XX.Xmin` และแปลงเป็นนาที
7. [ ] Handle กรณีไม่มี RAID array ใน `/proc/mdstat` → คืน `None`
8. [ ] Handle กรณี array active โดยไม่มี rebuild → `rebuild_pct = None`, `eta = None`

---

## 🛠 Technical Tasks

- [ ] สร้าง `src/collectors/raid.rs`
- [ ] สร้าง `struct RaidStatus` ตาม spec ใน [System Design](../../software/01-system-design.md)
- [ ] ใช้ `tokio::fs::read_to_string("/proc/mdstat")` สำหรับ async read
- [ ] สร้าง regex patterns สำหรับแต่ละ field (ดู patterns ใน System Design Section 2.1)
- [ ] เขียน unit tests ด้วย mock `/proc/mdstat` content (active, rebuilding, degraded, no-array)

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- System Design: [../../software/01-system-design.md](../../software/01-system-design.md)
- Sprint: [../sprint-backlogs/sprint-01.md](../sprint-backlogs/sprint-01.md)
