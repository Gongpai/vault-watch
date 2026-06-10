# User Story: US-MON-07 — SMART Details Panel

**Status:** ✅ Done
**Sprint:** [Sprint 02](../sprint-backlogs/sprint-02.md)
**Epic:** [Must Have — MVP Scope](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่ต้องการรายละเอียด SMART ของแต่ละ disk
**ฉันต้องการ** panel ที่แสดง serial, power-on hours, non-medium errors และ grown defects
**เพื่อให้** เห็นรายละเอียดที่ไม่ fit ในตาราง disk summary โดยไม่ต้องรัน `smartctl` แยก

---

## ✅ Acceptance Criteria

1. [ ] แสดงข้อมูลแยกต่อ disk ในรูปแบบ list
2. [ ] แต่ละแถวแสดง: `[device] Serial: XX  Hours: XX  NME: XX  Defects: XX`
3. [ ] Non-medium errors > 1000 → highlight สีเหลือง
4. [ ] Grown defects > 0 → highlight สีแดง พร้อมข้อความ WARN
5. [ ] กรณีข้อมูลไม่มี (None): แสดง `--`
6. [ ] Panel มี fixed height ตามสัดส่วน terminal (ไม่ขยายตามจำนวน disk) — รองรับ scroll เมื่อ disk มีมากกว่าพื้นที่ที่แสดงได้ (US-MON-13)

---

## 🛠 Technical Tasks

- [x] สร้าง `src/widgets/smart_details.rs`
- [x] Implement `fn render_smart_details(f: &mut Frame, area: Rect, state: &AppState)`
- [x] ใช้ `ratatui::widgets::{Block, List, ListItem}` หรือ `Paragraph` 
- [x] Format แต่ละ disk เป็น `Line` ที่มี `Span` หลายสี

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- System Design (Layout): [../../software/01-system-design.md](../../software/01-system-design.md)
- Sprint: [../sprint-backlogs/sprint-02.md](../sprint-backlogs/sprint-02.md)
