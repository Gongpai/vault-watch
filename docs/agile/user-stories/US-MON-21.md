# US-MON-21: Split Throughput Graph เป็น Read / Write สองช่อง

**Sprint:** 06 | **Estimate:** S (4h) | **Status:** ✅ Done

---

## User Story

**ในฐานะ** ผู้ดูแลระบบที่ดู throughput หลาย disk พร้อมกัน
**ฉันต้องการ** ให้ Throughput graph แยกเป็นช่อง Read และช่อง Write โดยแต่ละช่องใช้สีแยกตาม device
**เพื่อให้** รู้ว่าเส้น Write เป็นของ disk ตัวไหน — ตอนนี้เส้น Write ทุก disk เป็นสีเทา (`DarkGray`) เหมือนกันหมด แยกไม่ออก

---

## ปัญหาปัจจุบัน

`render_throughput_graph()` วาด Read + Write รวมใน chart เดียว — Read ใช้สีตาม `DISK_COLORS` แต่ Write ใช้ `DarkGray` ทุกเส้น เพราะถ้าให้ Write ใช้สีต่อ disk ด้วยจะชนกับสี Read ใน chart เดียวกัน → แก้โดยแยกเป็นสอง chart

---

## Acceptance Criteria

1. คอลัมน์ขวาของ Graph view แยกเป็น 2 panels บน/ล่าง: **Read (MB/s)** และ **Write (MB/s)**
2. ทั้งสอง panel ใช้สีต่อ device จาก `DISK_COLORS` ชุดเดียวกัน — `sda` สีเดียวกันทั้งช่อง Read และ Write
3. แต่ละ panel มี legend แสดง device name (ไม่ต้องมี suffix `R`/`W` แล้ว)
4. `Tab` cycle focus ครอบคลุม panel ใหม่ทั้งสอง, mouse click/scroll ทำงานต่อ panel ที่ถูกต้อง
5. Y-axis สอง panel ใช้ scale เดียวกัน เพื่อเทียบ Read vs Write ด้วยสายตาได้

---

## Technical Notes

**`src/widgets/graph_view.rs`:**
- แตก `render_throughput_graph()` → `render_read_graph()` + `render_write_graph()` (แชร์ helper เดียวกัน รับ `&HashMap<String, VecDeque<u64>>` + title)
- คอลัมน์ขวา: `Layout::vertical([Percentage(50), Percentage(50)])`

**`src/app.rs`:**
- `FocusedPanel::ThroughputGraph` → แทนด้วย `ReadGraph` + `WriteGraph`

**`src/main.rs`:**
- อัปเดต Tab cycling order ใน Graph view: `TempGraph → RaidGraph → ReadGraph → WriteGraph`
- `panel_at()` hit-test ใช้ `panel_rects` ที่ widget insert อยู่แล้ว — ทำงานต่อโดยไม่ต้องแก้

---

## Related

- [US-MON-12](./US-MON-12.md) — Graph View เดิม
- [US-MON-20](./US-MON-20.md) — Temperature legend (Sprint 06 เดียวกัน)
