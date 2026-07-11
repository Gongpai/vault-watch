# Sprint 09 — Tunable Y-Axis Label Offset

**Version:** 0.9.0 | **Duration:** 2026-09-30 → 2026-10-14 | **Status:** ✅ Done

---

## Sprint Goal

เพิ่ม **ตัวแปร offset** สำหรับปรับตำแหน่งตัวเลขแกน Y ของ Graph view ให้ตรงกับเส้นแบ่ง zone (ปัจจุบันตัวเลขลอยอยู่ใต้เส้นราวครึ่ง cell) — offset เป็น named constant ในกลุ่ม theme ปรับที่เดียวได้ ค่า default `-0.5` center glyph บนเส้นแบ่งพอดี โดยไม่ขยับ zone background ที่ถูกต้องแล้ว

ที่มา: feedback 2026-06-17 (screenshot 10:51) — "layout เขต temperature พอดีแล้ว แต่ตัวเลขแกนมันยังอยู่ไม่ตรงขอบ ช่วยเลื่อนตัวเลขขึ้นหน่อย" + (ชี้แจงเพิ่ม) "ไม่ได้ให้ hardcode แต่ปรับ offset ให้ตัวเลขที่ไม่ตรง อาจต้องมีตัวแปรสำหรับกำหนด offset ตำแหน่งตัวเลข temperature"

---

## User Stories

| ID | Story | Estimate | Priority |
|:---|:------|:---------|:---------|
| [US-MON-27](../user-stories/US-MON-27.md) | Tunable Y-Axis Label Offset — ตัวแปรปรับตำแหน่งตัวเลขแกน Y ให้ตรงเส้นแบ่ง | **S** (2h) | 🔴 Must |

**Total estimate:** 2h

---

## Core Principle

> **ตำแหน่งตัวเลข = สัดส่วน + offset ที่ตั้งได้** — offset เป็น named constant ในกลุ่ม theme (ปรับที่เดียวมีผลทุก label) ไม่ใช่ magic number กระจายในสูตร

```
boundary row (เดิม, ไม่เปลี่ยน):  ZoneBackground ใช้ row_pos(v).round() inline
label row    (ใหม่):              row_for_label(v) = round(row_pos(v) + Y_LABEL_OFFSET)
```

```rust
/// ปรับตำแหน่งแนวตั้งของตัวเลข label แกน Y (หน่วย: แถว)
/// -0.5 = เลื่อนขึ้นครึ่ง cell ให้ glyph center ตรงเส้นแบ่ง; จูนได้ตามฟอนต์/terminal
const Y_LABEL_OFFSET: f64 = -0.5;
```

`-0.5` เป็น **ค่า default** จาก geometry (cell center = `row + 0.5`) — แต่เป็น **ตัวแปร** ปรับได้ ไม่ใช่ค่าฝัง; terminal วาด text แบบ cell-granular จึง offset เป็นหน่วยแถว (รับทศนิยม ปัดตอนท้าย)

---

## Target Visual (Temperature graph — ตัวเลข center บนเส้น)

```
ก่อน (Sprint 08)                    หลัง (Sprint 09)
┌ Temperature (°C) ──────┐          ┌ Temperature (°C) ──────┐
│    │░░░░░░░░░░░░░░░░░░ │          │ 60 │░░░░░░░░░░░░░░░░░░ │  ← 60 คร่อมเส้นแบ่ง
│ 60 │████████████████  │  ← ใต้เส้น│    │████████████████  │
│ 50 │████████████████  │          │ 50 │████████████████  │  ← 50 คร่อมเส้นแบ่ง
└────┴──────────────────┘          └────┴──────────────────┘
```

---

## Implementation Plan

### 1. เพิ่ม `Y_LABEL_OFFSET` ในกลุ่ม theme constants

```rust
/// ปรับตำแหน่งแนวตั้งของตัวเลข label แกน Y (หน่วย: แถว) — จูนเทียบเส้นแบ่ง zone
const Y_LABEL_OFFSET: f64 = -0.5;
```

วางใต้ `IO_Y_MAX` ในกลุ่ม theme block (ร่วม `DISK_COLORS`/`TEMP_ZONES`/… จาก US-MON-26 Part A)

### 2. เพิ่ม `row_for_label()` ใน `graph_view.rs`

```rust
/// แถวสำหรับวาง "ตัวเลข label" — ใช้ Y_LABEL_OFFSET จูนเทียบเส้นแบ่ง
fn row_for_label(value: f64, y_min: f64, y_max: f64, height: u16) -> u16 {
    ((row_pos(value, y_min, y_max, height) + Y_LABEL_OFFSET).round() as i32)
        .clamp(0, height as i32 - 1) as u16
}
```

### 3. `render_y_labels` → ใช้ `row_for_label`

เปลี่ยนบรรทัด `let row = row_for_value(...)` เป็น `row_for_label(...)` จุดเดียว — ครอบ Temperature/Read/Write/RAID เพราะทุก graph เรียก `render_y_labels` ร่วมกัน

### 4. ลบ `row_for_value()` + ไม่แตะ `ZoneBackground`

`render_y_labels` เป็นผู้เรียก `row_for_value` รายเดียว → ย้ายไป `row_for_label` แล้ว `row_for_value` กลายเป็น dead code จึงลบทิ้ง; `ZoneBackground::render` คำนวณ `row_pos(v).round()` inline อยู่แล้ว — ไม่แตะ → เส้นแบ่ง zone อยู่ที่เดิม (US-MON-24/25 ไม่ regression)

### 5. เตรียม config override

เพิ่ม commented `label_offset` ใน `[graph]` ของ `config.example.toml` (planned US-MON-26 Part B)

---

## Files Changed

| File | Change |
|:-----|:-------|
| `src/widgets/graph_view.rs` | เพิ่ม `Y_LABEL_OFFSET` constant + `row_for_label()`; `render_y_labels` เรียก helper ใหม่ |
| `contrib/config.example.toml` | เพิ่ม commented `label_offset` ใน `[graph]` (planned Part B) |
| `docs/software/01-system-design.md` | §3.4 เพิ่มกฎ tunable label offset (boundary vs label row) |

---

## Definition of Done

- [x] `make build` ผ่านไม่มี error/warning (`cargo clippy` clean)
- [x] มี `Y_LABEL_OFFSET` named constant ในกลุ่ม theme block พร้อม doc comment
- [x] เปลี่ยนค่า `Y_LABEL_OFFSET` แล้วตัวเลขทุก graph ขยับตาม (helper เดียว `row_for_label`)
- [x] ย้าย visual screenshot verification ไป [US-MON-38](../user-stories/US-MON-38.md); ยังไม่ถือเป็น hardware-verified จนกว่า story นั้นผ่าน
- [x] เส้นแบ่ง zone ไม่ขยับจาก Sprint 08 (`ZoneBackground` ไม่ถูกแตะ)
- [x] Read/Write/RAID label ใช้ offset เดียวกัน (เรียก `render_y_labels` ร่วมกัน)
- [x] `config.example.toml` มี commented `label_offset` ใน `[graph]`
- [x] `cargo test` ผ่านทั้งหมด (16 passed)

---

## Known Risks

| Risk | Mitigation |
|:-----|:-----------|
| Label บนสุด/ล่างสุด (`90`/`0`) center ไม่ได้เพราะไม่มี cell เลยขอบ canvas | `clamp` ไว้ที่ขอบ — ยอมรับได้ เป็นข้อจำกัด cell grid |
| Integer rounding ทำให้บาง label เลื่อนขึ้น 0 แถว (ค่าที่ frac ≈ .0) | ยอมรับได้ — center ที่ดีที่สุดเท่าที่ cell grid ทำได้; ปรับ `Y_LABEL_OFFSET` จูนเพิ่มได้ |
| Offset เดียวอาจไม่พอดีทุก terminal/ฟอนต์ | เป็น **ตัวแปร** ปรับได้ที่เดียว + เตรียม config override (Part B) ให้ผู้ใช้จูนเองได้ |
