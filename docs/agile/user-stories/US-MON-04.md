# User Story: US-MON-04 — TUI Application Foundation

**Status:** 🚧 Planned
**Sprint:** [Sprint 01](../sprint-backlogs/sprint-01.md)
**Epic:** [Must Have — MVP Scope](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** นักพัฒนาที่จะต่อยอด UI ใน Sprint 02
**ฉันต้องการ** โครงสร้าง TUI application พื้นฐานที่ทำงานได้
**เพื่อให้** มี main event loop, terminal lifecycle, shared state และ keyboard handling พร้อมสำหรับใส่ widget จริง

---

## ✅ Acceptance Criteria

1. [ ] ตั้งค่า `Cargo.toml` ด้วย dependencies ที่จำเป็น (ratatui, crossterm, tokio, serde, regex)
2. [ ] Terminal เข้า raw mode เมื่อเริ่ม และออก raw mode เมื่อ quit หรือ panic
3. [ ] `AppState` struct มี fields ครบ: `raid`, `disks`, `io_stats`, `last_updated`, `disk_devices`
4. [ ] `Arc<Mutex<AppState>>` share ระหว่าง collector task และ render task ได้
5. [ ] กด `q` หรือ `Ctrl+C` → ออกจากโปรแกรมสะอาด terminal mode ถูก restore
6. [ ] กด `r` → set flag `force_refresh = true` เพื่อให้ collector รัน cycle ใหม่ทันที
7. [ ] Placeholder UI (กล่องว่าง + ข้อความ "Loading...") แสดงได้ก่อนข้อมูลพร้อม

---

## 🛠 Technical Tasks

- [ ] อัปเดต `Cargo.toml` เพิ่ม dependencies ทั้งหมด
- [ ] สร้าง `src/app.rs` — `AppState` struct + constructor
- [ ] สร้าง `src/main.rs` — tokio runtime, terminal setup, render loop, event loop
- [ ] สร้าง `src/ui.rs` — placeholder `draw()` function ที่รับ `&AppState`
- [ ] สร้าง `src/collectors/mod.rs` — module declarations
- [ ] ทดสอบ: รัน binary, เห็นหน้าจอ, กด `q` ออกได้สะอาด

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- Architecture: [../../software/00-architecture.md](../../software/00-architecture.md)
- Sprint: [../sprint-backlogs/sprint-01.md](../sprint-backlogs/sprint-01.md)
