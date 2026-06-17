# Sprint 08 — Proportional Graph Layout

**Version:** 0.8.1 | **Duration:** 2026-09-16 → 2026-09-30 | **Status:** ✅ Done (Part A) — US-MON-26 Part B carried to backlog

---

## Sprint Goal

ปรับ Graph view สองด้าน: (1) จัดตำแหน่งแกน Y ด้วยสูตรสัดส่วนเดียว (`value / max_value`) แทนการจับวาง เพื่อให้ zone background, เส้นแบ่ง zone และตัวเลขแกน Y ตรงตำแหน่งจริงและตรงกัน; (2) ปรับสีให้เส้นกราฟสว่างขึ้นและพื้นหลัง zone มืดลง 10% เพื่อเพิ่ม contrast

ที่มา: feedback 2026-06-11 — "เขต temperature ไม่ตรงกับ temperature จริง, ตัวหนังสือแถบซ้ายอยู่ตำแหน่งไม่ตรง, เขตที่ห่างกัน 10° ดูไม่เท่ากัน ทั้งที่ควรคำนวณง่ายๆ — 90° = 100%, เขต 60° = 66.67%" + "เส้นอยากให้สว่างขึ้น พื้นหลัง temperature มืดลงอีก 10%"

---

## User Stories

| ID | Story | Estimate | Priority |
|:---|:------|:---------|:---------|
| [US-MON-24](../user-stories/US-MON-24.md) | Proportional Graph Layout — คำนวณตำแหน่งด้วยคณิต ไม่จับวางเอง | **S** (4h) | 🔴 Must |
| [US-MON-25](../user-stories/US-MON-25.md) | Graph Color Tuning — เส้นสว่างขึ้น + พื้นหลัง zone มืดลง 10% | **S** (2h) | 🔴 Must |
| [US-MON-26](../user-stories/US-MON-26.md) Part A | Centralized Theme Constants — รวมค่าสี/zone/Y-max เป็น block เดียว | **S** (3h) | 🔴 Must |

**Total estimate:** 9h

> **US-MON-26 Part B** (config-driven theme — โหลดจาก `config.toml [graph]`) เป็น requirement สำหรับ backlog/sprint ถัดไป ไม่อยู่ใน scope Sprint 08 — Part A วาง constants ไว้ให้ Part B override ได้ทีหลัง

---

## Core Principle

> **ทุกตำแหน่งบนแกน Y คำนวณจากสูตรสัดส่วนเดียว — ห้ามจับวาง**

```
ratio        = value / max_value          # 60/90 = 0.666667
row_from_top = (1 - ratio) * height       # canvas row 0 = บนสุด
```

ตัวอย่าง: จอสูง 1024 px, ค่า 60°C → `0.666667 × 1024 = 682.67 px` จากล่าง

### Zone heights = สัดส่วนกับ span อุณหภูมิ

```
0–30°C  (teal)   30/90 = 33.3%  ████████████
30–40°C (green)  10/90 = 11.1%  ████
40–50°C (amber)  10/90 = 11.1%  ████
50–60°C (red)    10/90 = 11.1%  ████
60–90°C (purple) 30/90 = 33.3%  ████████████
```

---

## Target Visual (Temperature graph — สัดส่วนถูกต้อง)

```
┌ Temperature (°C) ──────────────────┐
│ 90 │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │  ← purple 60-90 = 33.3% (boundary ตรง 60)
│    │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
│    │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
│ 60 │████████████████████████████ │  ← เส้นแบ่ง = label "60" แถวเดียวกัน
│ 50 │████████████████████████████ │  ← red 50-60 = 11.1%
│ 40 │████████████████████████████ │  ← amber 40-50 = 11.1%
│ 30 │████████████████████████████ │  ← green 30-40 = 11.1%
│    │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
│    │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │  ← teal 0-30 = 33.3%
│  0 │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
└────┴──────────────────────────────┘
```

---

## Implementation Plan

### 1. Single positioning helper

```rust
/// แถว (offset จากบนของ area) ของค่า value บนแกน [y_min, y_max]
fn row_for_value(value: f64, y_min: f64, y_max: f64, height: u16) -> u16 {
    let ratio = (value - y_min) / (y_max - y_min);
    (((1.0 - ratio) * height as f64).round() as i32)
        .clamp(0, height as i32 - 1) as u16
}
```

### 2. `ZoneBackground::render` — fill จาก boundary ที่คำนวณ

แทน midpoint sampling เดิม → แต่ละ zone เติมสีตั้งแต่ `row_for_value(hi)` ถึง `row_for_value(lo)` (เส้นแบ่งจึงตรงกับ label เป๊ะ)

### 3. `render_y_labels` — ใช้ `row_for_value` ตัวเดียวกัน

ลบสูตร truncate เดิม → เรียก helper เดียวกับ ZoneBackground → ตำแหน่งตรงกันโดยอัตโนมัติ

### 4. ใช้กับทุก graph

Read/Write (max 200), RAID (max dynamic) เรียก `row_for_value` เหมือนกัน

---

## Color Tuning (US-MON-25)

### เส้นกราฟ — Bright RGB palette (`DISK_COLORS`)

```
1  Cyan     → Rgb(80, 250, 250)   #50FAFA
2  Yellow   → Rgb(250, 230, 90)   #FAE65A
3  Green    → Rgb(120, 250, 120)  #78FA78
4  Magenta  → Rgb(250, 130, 250)  #FA82FA
5  Blue     → Rgb(122, 170, 250)  #7AAAFA
6  Red      → Rgb(250, 130, 90)   #FA825A
```

### Zone backgrounds — มืดลง 10% (`new = round(old × 0.9)`)

```
0–30°C   #08354D → #073045   (8,53,77)  → (7,48,69)
30–40°C  #02370F → #02320E   (2,55,15)  → (2,50,14)
40–50°C  #473900 → #403300   (71,57,0)  → (64,51,0)
50–60°C  #400000 → #3A0000   (64,0,0)   → (58,0,0)
60–90°C  #270034 → #23002F   (39,0,52)  → (35,0,47)
```

IO/RAID background `#0A0D14` ไม่เปลี่ยน

---

## Centralized Theme Constants (US-MON-26 Part A)

รวมค่า theme ทั้งหมดเป็น block เดียวที่หัว `graph_view.rs` พร้อม doc comment — แก้ที่เดียว มีผลทุกที่ และวางโครงให้ Part B (config override) ต่อยอดได้

```rust
// ── Graph theme ───────────────────────────────────────────────────────────
const DISK_COLORS: [Color; 6] = [ /* bright RGB — US-MON-25 */ ];
const TEMP_ZONES: [(f64, f64, Color); 5] = [ /* มืดลง 10% — US-MON-25 */ ];
const IO_BG: Color = Color::Rgb(10, 13, 20);
const TEMP_Y_MAX: f64 = 90.0;   // แทน hardcoded 90.0 ในฟังก์ชัน
const IO_Y_MAX: f64 = 200.0;    // แทน hardcoded 200.0 ในฟังก์ชัน
```

→ ลบ magic number `90.0` / `200.0` ที่กระจายในฟังก์ชัน render ทุกตัว ให้อ้าง constant กลาง

---

## Files Changed

| File | Change |
|:-----|:-------|
| `src/widgets/graph_view.rs` | เพิ่ม `row_for_value()`, refactor `ZoneBackground` + `render_y_labels`; รวม theme constants (`DISK_COLORS` bright RGB, `TEMP_ZONES` มืดลง 10%, `IO_BG`, `TEMP_Y_MAX`, `IO_Y_MAX`) เป็น block เดียว |
| `contrib/config.example.toml` | เพิ่ม `[graph]` section (commented — planned US-MON-26 Part B) |
| `docs/software/01-system-design.md` | อัพเดท §3.2 (Canvas + สีใหม่), เพิ่มหลัก proportional positioning ใน §3.4 |

---

## Definition of Done

**US-MON-24 (positioning):**
- [x] `make build` ผ่านไม่มี error/warning (`cargo clippy` clean)
- [x] Zone boundary ของค่า X อยู่แถวเดียวกับ label "X" ทุกค่า (`30/40/50/60`) — verify ด้วยตาราง row mapping
- [x] Zone `30-40`, `40-50`, `50-60` สูงเท่ากัน (ต่างได้ ≤ 1 แถว) — 7/6/7 rows ที่ height 60
- [x] Zone `60-90` เริ่มที่ 66.67% ของความสูง (± 1 แถว) — `60 → row 20/60 = 33.3%` จากบน
- [x] ไม่มี hardcoded pixel/row offset ในโค้ด positioning — ทุกตำแหน่งผ่าน `row_pos()` / `row_for_value()`
- [x] สูตรเดียวกันใช้กับ Read/Write/RAID graphs ด้วย

**US-MON-25 (colors):**
- [x] เส้นกราฟทุก panel ใช้ bright RGB palette — สีต่อ device ตรงกันทุก panel
- [x] Zone backgrounds มืดลง 10% ตามค่าใหม่ทั้ง 5 zone
- [x] Legend สียังตรงกับเส้นกราฟ (ใช้ `DISK_COLORS` ร่วมกัน)
- [x] IO/RAID background `#0A0D14` ไม่เปลี่ยน

**US-MON-26 Part A (constants):**
- [x] ค่า theme ทั้งหมดอยู่ใน block เดียวที่หัว `graph_view.rs` พร้อม doc comment
- [x] ไม่มี magic number สี/Y-max กระจายในฟังก์ชัน render — ใช้ `TEMP_Y_MAX` / `IO_Y_MAX`
- [x] `config.example.toml` มี `[graph]` section (commented) สำหรับ Part B

---

## Known Risks

| Risk | Mitigation |
|:-----|:-----------|
| Integer terminal rows ทำให้สัดส่วนคลาด ±0.5 แถว | ยอมรับได้ — zone boundary กับ label ใช้สูตรเดียวกัน คลาดไปด้วยกัน จึงยังตรงกันเอง |
| Panel เตี้ยมาก (height < จำนวน label) label ซ้อนกัน | `clamp` ป้องกัน out-of-bounds; label ที่ชนกันยอมรับได้บนจอเล็กกว่า minimum |
