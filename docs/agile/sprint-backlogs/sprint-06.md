# Sprint 06 — Graph View Improvements

**Version:** 0.7.0 | **Duration:** 2026-08-19 → 2026-09-02 | **Status:** ✅ Done

---

## Sprint Goal

ปรับปรุง Graph view ให้อ่านได้จริงเมื่อมีหลาย disk / หลาย RAID array: legend บอกว่าเส้นไหนคือ device ไหนครบทุก chart, แยก Read/Write ออกจากกันเพื่อให้แยกสีต่อ device ได้, และช่อง RAID Rebuild แสดงเฉพาะตอนมี rebuild พร้อมรองรับหลาย array แยกสีเส้น

ที่มา: feedback จากการใช้งานจริง (2026-06-11) — "ฉันงงว่า write ของใคร เนื่องจากมันสีเทา" + temp graph ไม่มี legend + rebuild panel ว่างกินพื้นที่

---

## User Stories

| ID | Story | Estimate | Priority |
|:---|:------|:---------|:---------|
| [US-MON-20](../user-stories/US-MON-20.md) | Temperature Graph — Per-Device Legend | **S** (2h) | 🔴 Must |
| [US-MON-21](../user-stories/US-MON-21.md) | Split Throughput เป็น Read / Write สองช่อง | **S** (4h) | 🔴 Must |
| [US-MON-22](../user-stories/US-MON-22.md) | RAID Rebuild Graph — Conditional + Multi-Array | **M** (7h) | 🔴 Must |

**Total estimate:** 13h

---

## Target Layout (Graph view)

```
┌ Temperature (°C) ──────┐ ┌ Read (MB/s) ───────────┐
│  [legend: sda sdb …]   │ │  [legend: sda sdb …]   │
│                        │ │                        │
│ (เต็มคอลัมน์ซ้าย         │ ├ Write (MB/s) ──────────┤
│  เมื่อไม่มี rebuild)      │ │  [legend: sda sdb …]   │
├ RAID Rebuild (MB/s) ───┤ │                        │
│  [legend: md0 md1]     │ │                        │
│  ← แสดงเฉพาะตอน rebuild │ │                        │
└────────────────────────┘ └────────────────────────┘
```

---

## Recommended Implementation Order

```
1. US-MON-20  →  เล็กสุด แก้เฉพาะ render_temp_graph (legend fix)
2. US-MON-21  →  แตก throughput chart + FocusedPanel enum (UI อย่างเดียว)
3. US-MON-22  →  ใหญ่สุด — แตะ collector + AppState + UI (ทำท้ายสุด)
```

US-MON-22 แตะ `RaidStatus` ที่ US-MON-21 ไม่เกี่ยว — ทำแยก commit ได้อิสระ

---

## Definition of Done

**ทั่วไป:**
- [x] `make build` ผ่านไม่มี error/warning ใหม่
- [x] `cargo test` ผ่านทั้งหมด (รวม test ใหม่ของ multi-array parser)

**US-MON-20:**
- [ ] Temp graph มี legend device ครบที่มุมขวาบน (จอ 110×30 ขึ้นไป)
- [x] เส้น threshold 45°/55° ไม่โผล่ใน legend

**US-MON-21:**
- [x] Read กับ Write แยกคนละ panel — สีต่อ device ตรงกันทั้งสองช่อง
- [x] Tab / mouse focus ครอบคลุม panel ใหม่ครบ
- [x] Y-axis สองช่องใช้ scale เดียวกัน

**US-MON-22:**
- [x] ไม่มี rebuild → ไม่เห็นช่อง RAID, Temp เต็มคอลัมน์ซ้าย
- [ ] มี rebuild → ช่อง RAID โผล่พร้อมเส้นแยกสีต่อ array + legend ชื่อ array
- [x] `/proc/mdstat` ที่มี 2+ arrays parse ครบทุกตัว (unit test)
- [x] Table view RAID panel ยังทำงานปกติ

> **หมายเหตุ (2026-06-11):** ข้อที่ยังไม่ติ๊กต้อง verify บนเครื่องจริงที่มี `sd*` disks + mdadm rebuild (เครื่อง dev เป็น NVMe ไม่มี smartctl) — logic ผ่าน unit test และ smoke test ใน pty แล้ว (conditional layout + Read/Write split แสดงถูกต้อง)

---

## Carry-over จาก Sprint 05 (stretch — ทำถ้ามีเวลา)

จาก Known Gaps ใน [changelog 0.6.0](../../changelog.md):

- [ ] US-MON-18 AC4/AC5 — ข้อความ "No disk devices found" + แสดง active device list บน UI
- [ ] US-MON-17 AC6 — Troubleshooting section ใน MANUAL.md
- [ ] Unit tests สำหรับ `config.rs` (`smartctl_base_cmd`, `detect_distro`)
- [x] แสดง `read_errors`/`write_errors` ใน SMART details (ลบ dead_code warning) — ✅ ทำแล้วใน sprint นี้ พร้อมเก็บ clippy warnings เก่าทั้งหมด (clippy สะอาด 100%)

---

## Known Risks

| Risk | Mitigation |
|:-----|:-----------|
| Layout กระพริบเมื่อ rebuild เริ่ม/จบ (panel โผล่/หาย) | Hide delay — ซ่อนเมื่อ history เป็น 0 ทั้ง window แล้วเท่านั้น (AC6 ของ US-MON-22) |
| `FocusedPanel` เปลี่ยน → focus เดิมค้างที่ panel ที่หายไป | เมื่อ RaidGraph ถูกซ่อนขณะ focused → ย้าย focus ไป TempGraph |
| Multi-array `/proc/mdstat` format ต่างกันตาม level (raid0 ไม่มี `[N/M]`) | Unit test ครอบ raid0/raid1/raid10 + inactive block |
| สี array ชนกับสี disk (ใช้ `DISK_COLORS` ชุดเดียวกัน) | ยอมรับได้ — อยู่คนละ chart กัน |
