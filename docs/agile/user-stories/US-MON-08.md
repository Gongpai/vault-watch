# User Story: US-MON-08 — Auto-Refresh Async Loop

**Status:** ✅ Done
**Sprint:** [Sprint 02](../sprint-backlogs/sprint-02.md)
**Epic:** [Must Have — MVP Scope](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่ leave dashboard ทิ้งไว้บน server
**ฉันต้องการ** ให้หน้าจออัปเดตข้อมูลอัตโนมัติโดยไม่ต้องกด manual
**เพื่อให้** ติดตาม RAID rebuild progress และอุณหภูมิ disk ได้ต่อเนื่องแบบ realtime

---

## ✅ Acceptance Criteria

1. [ ] Collector task รันทุก **2 วินาที** — อ่าน /proc/mdstat + รัน smartctl + รัน iostat
2. [ ] Render task รันทุก **250ms** — วาด UI ใหม่จากข้อมูลล่าสุดใน AppState
3. [ ] แสดง `Last update: HH:MM:SS` บน header ของ UI
4. [ ] กด `r` → collector รันทันทีโดยไม่รอ 2 วินาที
5. [ ] หน้าจอไม่ flicker (ใช้ double buffering ของ ratatui)
6. [ ] Collector task ไม่ block render task (ทำงาน concurrently)

---

## 🛠 Technical Tasks

- [x] แยก collector loop เป็น `tokio::spawn` task แยกจาก render loop
- [x] ใช้ `tokio::time::interval(Duration::from_secs(2))` สำหรับ collector
- [x] ใช้ `tokio::time::interval(Duration::from_millis(250))` สำหรับ render
- [x] สร้าง `Arc<tokio::sync::Notify>` แยกต่างหาก ส่งไปยัง collector task — event loop เรียก `notify.notify_one()` เมื่อกด `r` ทำให้ collector ถูกปลุกทันทีโดยไม่รอ 2s
- [x] อัปเดต `last_updated: Instant` ใน AppState ทุกครั้งที่ collector complete
- [x] Format timestamp เป็น `HH:MM:SS` สำหรับแสดงใน UI

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- Architecture (Async Flow): [../../software/00-architecture.md](../../software/00-architecture.md)
- Sprint: [../sprint-backlogs/sprint-02.md](../sprint-backlogs/sprint-02.md)
