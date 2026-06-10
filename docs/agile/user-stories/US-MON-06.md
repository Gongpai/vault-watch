# User Story: US-MON-06 — Disk Summary Table

**Status:** 🔵 Planned
**Sprint:** [Sprint 02](../sprint-backlogs/sprint-02.md)
**Epic:** [Must Have — MVP Scope](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่ต้องการ overview ของ disk ทุกตัวในคราวเดียว
**ฉันต้องการ** ตารางสรุปที่แสดง temperature, health, read/write throughput และ defect count
**เพื่อให้** มองเห็นสถานะทุก disk พร้อมกันในหน้าจอเดียว โดยไม่ต้องรัน `smartctl` หรือ `iostat` แยกกัน

---

## ✅ Acceptance Criteria

1. [ ] แสดง table ที่มีคอลัมน์: `Disk`, `Temp`, `Health`, `Read MB/s`, `Write MB/s`, `Defects`
2. [ ] Temperature: แสดงพร้อม unit (เช่น `53°C`)
3. [ ] Health: `OK` (สีเขียว), `WARN`/`FAIL` (สีแดง)
4. [ ] Read/Write: แสดง 1 decimal place (เช่น `178.2`)
5. [ ] Defects: แสดงตัวเลข, highlight สีเหลืองถ้า > 0
6. [ ] กรณีข้อมูลไม่มี (None): แสดง `--`
7. [ ] Header row แยกชัดเจนด้วย separator
8. [ ] Column width ไม่ overflow terminal ขนาด 80 columns

---

## 🛠 Technical Tasks

- [ ] สร้าง `src/widgets/disk_table.rs`
- [ ] Implement `fn render_disk_table(f: &mut Frame, area: Rect, state: &AppState)`
- [ ] ใช้ `ratatui::widgets::{Table, Row, Cell}` 
- [ ] Merge ข้อมูลจาก `state.disks` (DiskInfo) กับ `state.io_stats` (IoStats) ตาม device name
- [ ] สร้าง helper function `temp_color(temp: u8) -> Color` สำหรับ color threshold
- [ ] กำหนด column widths: Disk(6), Temp(8), Health(8), Read(10), Write(10), Defects(8)

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- System Design (Layout): [../../software/01-system-design.md](../../software/01-system-design.md)
- Sprint: [../sprint-backlogs/sprint-02.md](../sprint-backlogs/sprint-02.md)
