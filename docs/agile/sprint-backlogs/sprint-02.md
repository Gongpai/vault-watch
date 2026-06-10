# Sprint 02: Dashboard UI

**Goal:** สร้าง ratatui dashboard ครบ 3 panels และ auto-refresh loop เพื่อให้ได้ HDD Monitor ที่ใช้งานได้จริงบน terminal
**Timeline:** 2026-06-24 → 2026-07-08

## 📅 Internal Timeline

```mermaid
gantt
    title Sprint 02 Tasks
    dateFormat  YYYY-MM-DD
    section UI Panels
    RAID Status Panel (US-MON-05)        :t1, 2026-06-24, 3d
    Disk Summary Table (US-MON-06)       :t2, after t1, 3d
    SMART Details Panel (US-MON-07)      :t3, after t1, 3d
    section Loop
    Auto-Refresh Loop (US-MON-08)        :t4, after t2, 2d
    section Polish
    Layout Composition & Integration     :t5, after t3, 3d
    Manual Testing on Server             :t6, after t5, 2d
```

---

## 📋 Committed Stories & Tasks

| ID | Story / Task | Owner | Estimate (Hrs) | Status |
|:---|:---|:---|:---|:---|
| [US-MON-05](../user-stories/US-MON-05.md) | **RAID Status Panel**<br>- สร้าง `raid_panel.rs` widget<br>- แสดง array name, state, disk count<br>- Progress bar rebuild %<br>- แสดง speed และ ETA | kong | 6 | ✅ Done |
| [US-MON-06](../user-stories/US-MON-06.md) | **Disk Summary Table**<br>- สร้าง `disk_table.rs` widget<br>- Table: Disk, Temp, Health, Read, Write, Defects<br>- Color coding ตาม threshold<br>- Merge ข้อมูลจาก `DiskInfo` + `IoStats` | kong | 8 | ✅ Done |
| [US-MON-07](../user-stories/US-MON-07.md) | **SMART Details Panel**<br>- สร้าง `smart_details.rs` widget<br>- List view: serial, hours, errors per disk<br>- Highlight ค่าที่ผิดปกติ | kong | 4 | ✅ Done |
| [US-MON-08](../user-stories/US-MON-08.md) | **Auto-Refresh Loop**<br>- Collector loop ทุก 2 วินาที<br>- Render loop ทุก 250ms<br>- Last updated timestamp<br>- `r` key force refresh | kong | 4 | ✅ Done |
| [US-MON-12](../user-stories/US-MON-12.md) | **History Buffer & Graph UI**<br>- เพิ่ม history ring buffers ใน AppState (VecDeque × 60 samples)<br>- Inline Sparkline ในคอลัมน์ Temp/Read/Write ของ disk table<br>- Sparkline RAID rebuild speed ใน RAID panel<br>- Full Chart view (Graph View) toggle ด้วย `g` | kong | 10 | ✅ Done |
| [US-MON-13](../user-stories/US-MON-13.md) | **Panel Focus & Scroll**<br>- `Tab`/`Shift+Tab` สลับ focus ระหว่าง panel<br>- `↑↓`/`jk`/`PgUp`/`PgDn`/`Home`/`End` scroll focused panel<br>- Mouse wheel scroll panel ที่เมาส์อยู่, click โฟกัส panel<br>- Double border สำหรับ focused panel<br>- `Scrollbar` widget ทุก panel<br>- Status bar แสดง focus indicator<br>- Mouse hit-testing ผ่าน `panel_rects` | kong | 8 | ✅ Done |

---

## 🛠 Sprint Specifics

### Definition of Done (DoD)

- รัน `sudo ./hdd-monitor` บน server จริง เห็น dashboard ครบ 3 panels
- ข้อมูล RAID, SMART, throughput ตรงกับ manual run ของ tools แต่ละตัว
- หน้าจออัปเดตทุก 2 วินาทีโดยไม่ flicker
- `q` ออกจากโปรแกรมสะอาด, `r` refresh ทันที
- รองรับ terminal ขนาด **100×28** (Table View) หรือ **110×30** (Graph View) หรือใหญ่กว่า
- `cargo clippy` และ `cargo test` ผ่านสะอาด
