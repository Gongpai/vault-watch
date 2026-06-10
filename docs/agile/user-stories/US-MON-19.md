# US-MON-19: Key Hint Bar (nano-style)

**Sprint:** 05 | **Estimate:** S (3h) | **Status:** 🟡 Planned

---

## User Story

**ในฐานะ** ผู้ใช้ที่ไม่คุ้นเคยกับ keyboard shortcuts ของ VaultWatch
**ฉันต้องการ** แถบแสดง keyboard shortcuts ที่ด้านล่างสุดของหน้าจอ
**เพื่อให้** รู้ว่ากดปุ่มไหนได้บ้างโดยไม่ต้องจำหรืออ่าน README

---

## Acceptance Criteria

1. **Always visible** — แถบแสดงอยู่ที่ด้านล่างสุดของ terminal ตลอดเวลา (Table view และ Graph view)
2. **Context-aware** — แสดง shortcut ที่ใช้งานได้ใน view/focus ปัจจุบัน (เช่น `g:graph` เปลี่ยนเป็น `g:table` เมื่ออยู่ใน Graph view)
3. **nano-style layout** — จัดเป็นคู่ `[key] action` เรียงแนวนอน คั่นด้วย space
4. **สี** — key แสดงสีต่างจาก action label (เช่น key = Cyan/White invert, label = DarkGray)
5. **ไม่ล้น** — ถ้า terminal แคบเกินไปให้ตัด shortcut ที่สำคัญน้อยกว่าออก แทนที่จะล้น

---

## UI Mockup

```
^Q Quit  ^R Refresh  G Graph  Tab Panel  ↑↓ Scroll  PgUp/Dn Page  ? Help
```

หรือแบบ nano ที่มี invert background:

```
[ q ]Quit  [ r ]Refresh  [ g ]Graph  [Tab]Panel  [ ↑↓]Scroll  [PgU]Page
```

สีที่วางแผน:
- Key label: `Style::default().fg(Color::Black).bg(Color::Cyan)` (invert — เหมือน nano)
- Action text: `Style::default().fg(Color::Gray)`

---

## Technical Notes

**Layout change** (`src/ui.rs`):
- เพิ่ม `Constraint::Length(1)` ที่ท้ายสุดของ constraints ทั้ง Table view และ Graph view
- สร้าง `render_key_bar(f, area, state)` function ใหม่

**Context-aware shortcuts:**

| View | Shortcuts แสดง |
|:-----|:--------------|
| Table (DiskTable focused) | `q`Quit `r`Refresh `g`Graph `Tab`Panel `↑↓`Scroll `PgU/D`Page |
| Table (SmartDetails focused) | `q`Quit `r`Refresh `g`Graph `Tab`Panel `↑↓`Scroll `Home/End`Jump |
| Graph view | `q`Quit `r`Refresh `g`Table `Tab`Panel `↑↓`Scroll |

**ของที่ต้องแก้:**
- `src/ui.rs` — เพิ่ม `render_key_bar()` + ปรับ layout constraints ทั้ง `render_table_view` และ `render_graph_view`
- `render_header()` — ลบ key hints ออกจาก header (ย้ายมาที่ bar แทน เพื่อไม่ซ้ำซ้อน)

---

## Related

- [US-MON-13](./US-MON-13.md) — Panel Focus & Scroll (shortcuts ที่จะแสดงใน bar)
- [US-MON-18](./US-MON-18.md) — Auto-detect Disk Devices (Sprint 05 เดียวกัน)
