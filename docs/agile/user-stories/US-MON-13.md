# User Story: US-MON-13 — Panel Focus & Scroll

**Status:** ✅ Done
**Sprint:** [Sprint 02](../sprint-backlogs/sprint-02.md)
**Epic:** [Must Have — MVP Scope](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่ติดตั้ง HDD มากกว่า 3–5 ลูก
**ฉันต้องการ** scroll ขึ้น/ลงภายใน panel ใดก็ได้ด้วย mouse wheel หรือ keyboard และสลับ focus ระหว่าง panel ด้วย Tab
**เพื่อให้** ดูข้อมูลของ disk ทุกลูกได้แม้หน้าจอ terminal มีพื้นที่จำกัด

---

## ✅ Acceptance Criteria

### Panel Focus
1. [ ] กด `Tab` ย้าย focus ไปยัง panel ถัดไปตาม cycle: DiskTable → SmartDetails → DiskTable (Table View) หรือ TempGraph → ThroughputGraph → RaidGraph → TempGraph (Graph View)
2. [ ] กด `Shift+Tab` ย้าย focus ย้อนกลับ
3. [ ] Click mouse บน panel ใดก็ได้ โฟกัส panel นั้นทันที
4. [ ] Panel ที่ focused แสดง double border `╔╗╚╝` สีสว่าง; panel ที่ไม่ focused ใช้ single border สีปกติ
5. [ ] Status bar ระหว่าง DiskTable กับ SmartDetails แสดง `● FocusedPanel [N/total — hint]` และ `○ UnfocusedPanel`

### Keyboard Scroll
6. [ ] `↑` / `k` เลื่อน focused panel ขึ้น 1 แถว
7. [ ] `↓` / `j` เลื่อน focused panel ลง 1 แถว
8. [ ] `PgUp` เลื่อนขึ้น `visible_rows - 1` แถว
9. [ ] `PgDn` เลื่อนลง `visible_rows - 1` แถว
10. [ ] `Home` กลับไปแถวแรก, `End` ไปแถวสุดท้าย
11. [ ] ไม่สามารถ scroll เกินขอบเขตได้ (clamp ที่ 0 และ max)

### Mouse Scroll
12. [ ] Scroll wheel up บน panel → scroll panel นั้นขึ้น 3 แถว (ไม่สนใจว่า focus อยู่ที่ panel ใด)
13. [ ] Scroll wheel down บน panel → scroll panel นั้นลง 3 แถว
14. [ ] Mouse scroll ทำงานได้ทั้ง Table View และ Graph View

### Scrollbar Visual
15. [ ] ทุก panel ที่ scroll ได้แสดง `ratatui::widgets::Scrollbar` ทางขวา (`▲ █ ░ ▼`)
16. [ ] Scrollbar thumb เคลื่อนที่สัมพัทธ์กับตำแหน่ง scroll จริง
17. [ ] เมื่อ content ทั้งหมดพอดีกับ panel (ไม่ต้อง scroll) ไม่แสดง scrollbar หรือแสดง thumb เต็มความสูง

### Overflow Indicator
18. [ ] เมื่อมีแถวซ่อนอยู่ด้านล่าง แสดง `↓ N more` ที่แถวสุดท้ายของ panel (แทนแถวข้อมูลสุดท้าย)

---

## 🛠 Technical Tasks

- [x] เพิ่ม `focused_panel: FocusedPanel`, `disk_table_scroll: usize`, `smart_details_scroll: usize`, `graph_scroll: usize` ใน `AppState` (ทำใน US-MON-04 หรือ US-MON-13)
- [x] เพิ่ม `panel_rects: HashMap<FocusedPanel, ratatui::layout::Rect>` ใน `AppState`
- [x] เพิ่ม `FocusedPanel` enum พร้อม `#[derive(Hash, Eq, PartialEq, Clone, Copy)]`
- [x] สร้าง `fn panel_at(rects, col, row) -> Option<FocusedPanel>` สำหรับ mouse hit-testing
- [x] อัปเดต keyboard handler รับ `Tab`, `Shift+Tab`, `↑`, `↓`, `k`, `j`, `PgUp`, `PgDn`, `Home`, `End`
- [x] เพิ่ม `EnableMouseCapture` ใน terminal setup และ handle `MouseEvent::ScrollUp/Down/Click`
- [x] อัปเดต `disk_table.rs` บันทึก `Rect` ลง `panel_rects` ทุก frame และ render double border เมื่อ focused
- [x] อัปเดต `smart_details.rs` เช่นเดียวกัน
- [x] เพิ่ม `Scrollbar` widget ทางขวาของทุก panel ที่ scroll ได้
- [x] เพิ่ม overflow indicator (`↓ N more`) ที่แถวสุดท้ายของ panel เมื่อมี content ซ่อนอยู่
- [x] เพิ่ม status bar บรรทัดระหว่าง DiskTable กับ SmartDetails แสดง focus state

---

## 📐 ratatui Widgets ที่ใช้

| Widget | ใช้กับ |
|:---|:---|
| `Scrollbar` + `ScrollbarState` | ทุก panel ที่ scroll ได้ (DiskTable, SmartDetails, Graph panels) |
| `Block` (border style) | Double border สำหรับ focused panel, single border สำหรับ unfocused |
| `Paragraph` | Status bar แสดง focus indicator |

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- System Design (AppState + Layout + Interaction): [../../software/01-system-design.md](../../software/01-system-design.md)
- Sprint: [../sprint-backlogs/sprint-02.md](../sprint-backlogs/sprint-02.md)
- US-MON-04 (TUI Foundation): [US-MON-04.md](./US-MON-04.md)
- US-MON-06 (Disk Table): [US-MON-06.md](./US-MON-06.md)
- US-MON-07 (SMART Details Panel): [US-MON-07.md](./US-MON-07.md)
