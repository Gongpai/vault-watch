# User Story: US-MON-10 — SMART Threshold Warnings

**Status:** ✅ Done (Sprint 03)
**Sprint:** [Sprint 03](../sprint-backlogs/sprint-03.md)
**Epic:** [Should Have — Future Enhancements](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่ต้องการ early warning ก่อน disk จะพัง
**ฉันต้องการ** การแจ้งเตือนบน UI เมื่อ SMART threshold ถูกละเมิด
**เพื่อให้** รู้ทันทีเมื่อ disk เริ่มมีปัญหาและสามารถ plan disk replacement ได้ล่วงหน้า

---

## ✅ Acceptance Criteria

1. [x] Warning banner เมื่อ `grown_defects > 0` สำหรับ disk ใดก็ตาม
2. [x] Critical alert เมื่อ `health_ok == false`
3. [x] Warning เมื่อ `temperature > 55°C`
4. [x] Banner แสดงชัดเจนด้านบนสุดของ UI (ไม่บัง panel อื่น)
5. [x] หน้าจอ blink หรือ highlight สี เพื่อดึงความสนใจ (border Red/Yellow บน affected panels)

---

## 🛠 Technical Tasks

- [x] สร้าง `fn collect_alerts(state: &AppState) -> Vec<Alert>` — ตรวจสอบ conditions ทั้งหมด
- [x] สร้าง `Alert` enum/struct ใน `src/app.rs` (`HighTemp`, `DiskFail`, `GrownDefects`, `RaidDegraded`)
- [x] อัปเดต `ui.rs` — เพิ่ม alert banner ใต้ header (1–2 rows) แสดงเฉพาะเมื่อมี alert
- [x] เพิ่ม `alerts: Vec<Alert>` ใน `AppState` และอัปเดตใน `collector_loop` ทุกรอบ
- [x] Highlight border สีแดงบน panel ที่มี disk มีปัญหา (Red=critical, Yellow=warning)

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- US-MON-09 (Color Coding): [US-MON-09.md](./US-MON-09.md)
- Sprint: [../sprint-backlogs/sprint-03.md](../sprint-backlogs/sprint-03.md)
