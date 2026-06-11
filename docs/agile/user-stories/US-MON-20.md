# US-MON-20: Temperature Graph — Per-Device Legend

**Sprint:** 06 | **Estimate:** S (2h) | **Status:** 🟡 Planned

---

## User Story

**ในฐานะ** ผู้ดูแลระบบที่ดู Graph view
**ฉันต้องการ** เห็น legend บอกว่าเส้น temperature แต่ละสีคือ disk ตัวไหน (แบบเดียวกับ Throughput graph)
**เพื่อให้** รู้ทันทีว่า disk ไหนร้อนเท่าไหร่ โดยไม่ต้องเดาจากสีเส้น

---

## ปัญหาปัจจุบัน

Temperature graph วาดเส้นแยกสีต่อ disk อยู่แล้ว (`DISK_COLORS`) แต่ **legend ไม่แสดง** — ratatui `Chart` ซ่อน legend อัตโนมัติเมื่อจำนวน dataset names เกิน ¼ ของความสูง panel (default `hidden_legend_constraints`) ซึ่ง Temp graph มี 5 disks + 2 threshold names (`45°C`, `55°C`) = 7 รายการ จึงโดนซ่อน ในขณะที่ Throughput graph (สูงเต็มจอ) แสดงได้

---

## Acceptance Criteria

1. Legend แสดง device name พร้อมสีของเส้นที่มุมขวาบนของ Temperature graph (เหมือน Throughput)
2. Legend แสดงครบทุก device ที่ monitor อยู่ (รองรับ ≥ 5 disks)
3. เส้น threshold 45°/55° **ไม่ปรากฏใน legend** (มี label ที่แกน Y อยู่แล้ว — ไม่ต้องกินพื้นที่ legend)
4. Legend ไม่บังเส้น graph จนอ่านไม่ได้ในจอขนาด minimum (110×30)

---

## Technical Notes

**`src/widgets/graph_view.rs` — `render_temp_graph()`:**

- ลบ `.name("45°C")` / `.name("55°C")` ออกจาก threshold datasets — dataset ที่ไม่มี name จะไม่ถูกนับใน legend
- เพิ่ม `.hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)))` บน `Chart` เพื่อให้ legend แสดงได้สูงสุดครึ่ง panel (กัน disk เยอะแล้วโดนซ่อนอีก)

---

## Related

- [US-MON-12](./US-MON-12.md) — Graph View เดิม
- [US-MON-21](./US-MON-21.md) — Read/Write graph split (Sprint 06 เดียวกัน)
