# User Story: US-MON-12 — History Buffer & Graph UI

**Status:** ✅ Done
**Sprint:** [Sprint 02](../sprint-backlogs/sprint-02.md)
**Epic:** [Must Have — MVP Scope](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่ต้องการเห็น trend ของอุณหภูมิและ throughput
**ฉันต้องการ** ให้ค่าตัวเลขทุกตัว (Temperature, Read MB/s, Write MB/s, RAID speed) แสดงเป็น graph แทนตัวเลขเดี่ยว
**เพื่อให้** เห็นทิศทางการเปลี่ยนแปลงและ pattern ได้ในทันที โดยไม่ต้องรอ alert

---

## ✅ Acceptance Criteria

### History Buffer
1. [ ] `AppState` มี history ring buffer สำหรับ temperature, read speed, write speed ต่อ disk และ RAID rebuild speed
2. [ ] Buffer เก็บค่าได้ 60 sample (2 นาทีที่ผ่านมา ที่ interval 2 วินาที)
3. [ ] ทุกครั้งที่ collector รัน ค่าใหม่ถูก push ต่อท้าย และค่าเก่าสุดถูกตัดออกอัตโนมัติเมื่อ buffer เต็ม
4. [ ] ค่า speed เก็บเป็น `u64` หน่วย MB/s × 10 เพื่อรองรับ 1 decimal ใน sparkline

### Table View — Inline Sparklines
5. [ ] คอลัมน์ Temperature ใน disk table แสดง Sparkline 12 sample ล่าสุด + ค่าปัจจุบัน (°C)
6. [ ] คอลัมน์ Read MB/s แสดง Sparkline 12 sample ล่าสุด + ค่าปัจจุบัน
7. [ ] คอลัมน์ Write MB/s แสดง Sparkline 12 sample ล่าสุด + ค่าปัจจุบัน
8. [ ] RAID panel แสดง Sparkline rebuild speed 20 sample ล่าสุด ใต้ progress bar
9. [ ] Sparkline ใช้สีตาม color scheme: Temperature มีสี Green/Yellow/Red ตาม threshold

### Graph View — Full Chart
10. [ ] กด `g` สลับจาก Table View เป็น Graph View และกลับ
11. [ ] Graph View แสดง Temperature chart (left) และ Throughput chart (right) พร้อม axis labels
12. [ ] Temperature chart แสดง line ต่อ disk พร้อม Y axis เป็น °C และ X axis เป็นวินาที
13. [ ] Throughput chart แสดง Read (solid) และ Write (dashed style) ต่อ disk
14. [ ] RAID rebuild speed chart แสดงที่ล่างซ้าย
15. [ ] แต่ละ disk ใช้คนละสี: sdc=Cyan, sdd=Yellow, sde=Green

---

## 🛠 Technical Tasks

- [x] เพิ่ม `temp_history`, `read_history`, `write_history`, `raid_speed_history` ใน `AppState`
- [x] เพิ่ม `ViewMode` enum และ `view_mode` field ใน `AppState`
- [x] อัปเดต collector loop ให้ `push_back` ค่าใหม่และ `pop_front` เมื่อเกิน `HISTORY_SIZE`
- [x] สร้าง `src/widgets/sparkline_cell.rs` — helper สำหรับ render `Sparkline` + value ภายใน table Cell
- [x] อัปเดต `disk_table.rs` เปลี่ยนคอลัมน์ Temp/Read/Write จาก plain text เป็น Sparkline cell
- [x] อัปเดต `raid_panel.rs` เพิ่ม Sparkline rebuild speed ใต้ progress bar
- [x] สร้าง `src/widgets/graph_view.rs` — render `Chart` สำหรับ temperature และ throughput
- [x] อัปเดต keyboard handler ให้ `g` toggle `view_mode`
- [x] อัปเดต `ui.rs` ให้เรียก `graph_view` หรือ `disk_table` ตาม `view_mode`

---

## 📐 ratatui Widgets ที่ใช้

| Widget | ใช้กับ |
|:---|:---|
| `Sparkline` | Inline sparkline ในแถว disk table และ RAID panel |
| `Chart` + `Dataset` | Full line chart ใน Graph View |
| `Axis` | X/Y axis สำหรับ Chart |

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- System Design (AppState + Layout): [../../software/01-system-design.md](../../software/01-system-design.md)
- Sprint: [../sprint-backlogs/sprint-02.md](../sprint-backlogs/sprint-02.md)
- US-MON-06 (Disk Table): [US-MON-06.md](./US-MON-06.md)
- US-MON-05 (RAID Panel): [US-MON-05.md](./US-MON-05.md)
