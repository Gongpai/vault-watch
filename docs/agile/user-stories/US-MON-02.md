# User Story: US-MON-02 — SMART Data Collector

**Status:** ✅ Done
**Sprint:** [Sprint 01](../sprint-backlogs/sprint-01.md)
**Epic:** [Must Have — MVP Scope](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่ต้องการตรวจสอบสุขภาพ SAS HDD
**ฉันต้องการ** ให้โปรแกรมรัน `smartctl -a -d scsi` สำหรับแต่ละ disk และ parse ผลลัพธ์
**เพื่อให้** ได้ข้อมูลอุณหภูมิ, health status, serial number และ error counts ที่แม่นยำ

> **เหตุผลที่ใช้ `smartctl`:** `lm-sensors` + `drivetemp` แสดงเพียง disk เดียวจากทั้งหมด 3 ตัวในระบบ LSI HBA IT Mode — `smartctl -d scsi` คือ source of truth เดียวที่เชื่อถือได้สำหรับ SAS disk

---

## ✅ Acceptance Criteria

1. [ ] รัน `smartctl -a -d scsi /dev/sdX` สำหรับแต่ละ disk ใน device list ด้วย `tokio::process::Command`
2. [ ] Parse serial number
3. [ ] Parse `Current Drive Temperature` เป็น `u8` (°C)
4. [ ] Parse `SMART Health Status` → `health_ok: true` ถ้าเป็น "OK" หรือ "PASSED"
5. [ ] Parse `Power_On_Hours`
6. [ ] Parse `Elements in grown defect list`
7. [ ] Parse `Non-medium error count`
8. [ ] Parse read/write errors จาก error counter table
9. [ ] Handle กรณี disk ไม่ response / permission denied → คืน `DiskInfo` ที่มีค่า None ในทุก optional field
10. [ ] รัน disk ทั้งหมดพร้อมกัน (`join_all`) ไม่รอทีละตัว

---

## 🛠 Technical Tasks

- [x] สร้าง `src/collectors/smart.rs`
- [x] สร้าง `struct DiskInfo` ตาม spec ใน [System Design](../../software/01-system-design.md)
- [x] Implement `async fn collect_all(devices: &[String]) -> Vec<DiskInfo>`
- [x] ใช้ `tokio::process::Command::new("sudo").args(["smartctl", "-a", "-d", "scsi", device])` 
- [x] สร้าง regex patterns ตาม System Design Section 2.2
- [x] ใช้ `futures::future::join_all` สำหรับ concurrent collection
- [x] เขียน unit tests ด้วย mock smartctl output

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- System Design: [../../software/01-system-design.md](../../software/01-system-design.md)
- Sprint: [../sprint-backlogs/sprint-01.md](../sprint-backlogs/sprint-01.md)
