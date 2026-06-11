# US-MON-22: RAID Rebuild Graph — แสดงเฉพาะตอน Rebuild + รองรับหลาย Array

**Sprint:** 06 | **Estimate:** M (7h) | **Status:** ✅ Done

---

## User Story

**ในฐานะ** ผู้ดูแลระบบที่มี mdadm array มากกว่าหนึ่งชุด
**ฉันต้องการ** ให้ช่อง RAID Rebuild Speed แสดงขึ้นมาเฉพาะเมื่อมี rebuild กำลังทำงาน และแสดงเส้น graph แยกสีต่อ rebuild process (ต่อ array) แบบเดียวกับที่แยกสีต่อ disk
**เพื่อให้** ไม่เสียพื้นที่จอกับ panel ว่างเปล่า และเห็นความเร็ว rebuild ของแต่ละ array แยกกันได้

---

## ปัญหาปัจจุบัน

1. `render_raid_graph()` แสดงตลอดเวลา — ตอนไม่มี rebuild เป็น panel ว่างกินพื้นที่ 35% ของคอลัมน์ซ้าย
2. เส้น rebuild มีเส้นเดียวสี Yellow ชื่อ `"rebuild"` — `collectors/raid.rs` parse แค่ array แรกจาก `/proc/mdstat` (`RaidStatus` ตัวเดียว) ระบบที่มี `md0` + `md1` rebuild พร้อมกันเห็นแค่ตัวเดียว

---

## Acceptance Criteria

1. **Conditional panel** — ช่อง RAID Rebuild Speed แสดงเฉพาะเมื่อมี array อย่างน้อยหนึ่งตัวกำลัง rebuild/resync; เมื่อไม่มี → Temperature graph ขยายเต็มคอลัมน์ซ้าย
2. **Multi-array parsing** — `collectors/raid.rs` parse ทุก `mdN` ใน `/proc/mdstat` ไม่ใช่แค่ตัวแรก
3. **Per-array lines** — เส้น rebuild speed แยกสีต่อ array พร้อม legend ชื่อ array (`md0`, `md1`, …) แบบเดียวกับ disk lines
4. **History ต่อ array** — เก็บ speed history แยก key ต่อ array; array ที่ rebuild จบแล้วให้เส้นค่อยๆ ไหลออกจากกราฟตาม history window (ไม่หายทันที)
5. **Table view ไม่พัง** — RAID panel ใน Table view ยังทำงานเดิม (แสดง array ที่กำลัง rebuild เป็นหลัก หรือ array แรกเมื่อไม่มี rebuild)
6. **Hide delay** — panel ยังแสดงค้างต่ออีกช่วงสั้นๆ หลัง rebuild จบ (ตราบใดที่ history ยังมีค่า non-zero) เพื่อไม่ให้ layout กระพริบ

---

## Technical Notes

**`src/collectors/raid.rs`:**
- `collect() -> Option<RaidStatus>` → `collect() -> Vec<RaidStatus>` — loop ทุก block `mdN :` ใน `/proc/mdstat`
- เพิ่ม `name: String` (มีอยู่แล้ว) เป็น key หลัก

**`src/app.rs`:**
- `raid: RaidStatus` → `raids: Vec<RaidStatus>`
- `raid_speed_history: VecDeque<u64>` → `HashMap<String, VecDeque<u64>>` (key = array name, scale ×10 เดิม)
- `collect_alerts()` — `RaidDegraded` ตรวจทุก array (alert message ระบุชื่อ array)

**`src/main.rs` — `collector_loop`:**
- push speed history ต่อ array; array ที่ไม่ rebuild push `0` (รักษา time axis ตรงกัน — pattern เดิมจาก Sprint 01 fix)

**`src/widgets/graph_view.rs`:**
- `render()` — คำนวณ `show_raid = state.raids.iter().any(rebuilding) || raid history มีค่า non-zero`; constraints คอลัมน์ซ้าย: `[Percentage(100)]` หรือ `[Percentage(65), Percentage(35)]`
- `render_raid_graph()` — dataset ต่อ array, สีจากชุดเดียวกับ `DISK_COLORS`, legend = array name

**`src/widgets/raid_panel.rs` (Table view):**
- รับ `Vec<RaidStatus>` — แสดง array ที่ rebuilding ก่อน, fallback array แรก

---

## Related

- [US-MON-01](./US-MON-01.md) — mdstat parser เดิม (single array)
- [US-MON-05](./US-MON-05.md) — RAID panel (Table view)
- [US-MON-12](./US-MON-12.md) — Graph View เดิม
