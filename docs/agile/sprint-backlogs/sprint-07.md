# Sprint 07 — Canvas Graph Redesign

**Version:** 0.8.0 | **Duration:** 2026-09-02 → 2026-09-16 | **Status:** ✅ Done

---

## Sprint Goal

แทนที่ `ratatui::Chart` ทุกช่องใน Graph view ด้วย `Canvas` เพื่อให้รองรับ zone background สีตามระดับความร้อนบน Temperature graph และ dark background แบบ unified บน Read/Write/RAID graphs — ทุก panel มี visual style เดียวกัน อ่านได้ง่ายขึ้น และเหมาะกับการมอนิเตอร์ระยะยาว

ที่มา: feedback 2026-06-11 — "สามารถแยก zone สีระดับความร้อนออกเป็น 0°/30°/40°/50°/60°/90° ได้ไหม" + "ถ้าเปลี่ยนเป็น Canvas ก็ต้องเปลี่ยน Read/Write และ RAID ด้วย ให้มัน style เดียวกัน"

---

## User Stories

| ID | Story | Estimate | Priority |
|:---|:------|:---------|:---------|
| [US-MON-23](../user-stories/US-MON-23.md) | Canvas Graph Redesign — Zone Backgrounds + Unified Dark Theme | **M** (7h) | 🔴 Must |

**Total estimate:** 7h

---

## Target Visual (Graph view)

```
┌ Temperature (°C) ──────────────────┐ ┌ Read (MB/s) ───────────────────────┐
│ 90 │████████████████████░░░░░░░░░░ │ │200│                         ┌─────┐ │
│    │████████████████████████░░░░░  │ │   │  [dark bg #0A0D14]      │█ sda│ │
│ 60 │████████████████░░░░░░░░░░░░░  │ │   │                         │█ sdb│ │
│ 50 │████████████████████████░░░░░░ │ │100│ ⠿⠿⠿⠿⠿⠿⠿⠿⠿⠿⠿⠿⠿          └─────┘ │
│ 40 │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │ │   │                                 │
│ 30 │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │ │  0│                                 │
│  0 │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │ ├ Write (MB/s) ──────────────────────┤
│    │  [zone colors per 30/40/50/60] │ │   │  [dark bg #0A0D14]              │
│    │  ┌──────┐                      │ │   │                                 │
│    │  │█ sda │                      │ │   │                                 │
│    │  │█ sdb │                      │ │   │                                 │
└────┴──┴──────┴──────────────────────┘ └───┴─────────────────────────────────┘
```

Zone colors (แสดงเป็น `░` / `█`):
- `0–30°C` `#08354D` dark teal
- `30–40°C` `#02370F` dark green
- `40–50°C` `#473900` dark amber
- `50–60°C` `#400000` dark red
- `60–90°C` `#270034` dark purple

---

## Implementation Details

### ZoneBackground Widget (2-pass rendering)

```rust
// Pass 1: Canvas renders braille lines (bg=Color::Reset)
f.render_widget(Canvas::default()
    .background_color(Color::Reset)
    .paint(|ctx| { /* draw lines */ }), canvas_area);

// Pass 2: ZoneBackground sets bg per row (char/fg untouched)
f.render_widget(ZoneBackground { zones: &TEMP_ZONES, y_min: 0.0, y_max: 90.0 }, canvas_area);
```

เหตุที่ใช้ 2-pass: `Canvas::Rectangle` วาดแค่ outline ไม่ใช่ filled rect — ต้องใช้ `Widget::render()` แตะ `buf.cell_mut()` โดยตรงเพื่อ set background

### Layout ต่อ panel

```
inner area (หลัง block.inner()):
  [Constraint::Length(4)] = y-axis label column (bg: black)
  [Constraint::Min(0)]    = canvas area
```

### IO/RAID Background

ใช้ `Canvas.background_color(IO_BG)` โดยตรง — Canvas เติม bg ทุก cell ให้เอง ไม่ต้อง ZoneBackground

---

## Files Changed

| File | Change |
|:-----|:-------|
| `src/widgets/graph_view.rs` | **Rewrite** — Chart → Canvas, เพิ่ม ZoneBackground widget, helpers |

ไม่มีไฟล์อื่นเปลี่ยน — behavioral logic (focus, scroll, RAID conditional panel) ไม่แตะ

---

## Definition of Done

- [x] `make build` ผ่านไม่มี error/warning (`cargo clippy` clean)
- [x] Temperature graph แสดง 5 zone background colors แยกตามช่วงอุณหภูมิ
- [x] เส้น device braille lines ทับบน zone background ได้ — foreground color ไม่ถูกบัง
- [x] ไม่มี threshold lines บน Temperature graph — zone background ทำหน้าที่แทน
- [x] Read, Write, RAID graphs แสดง dark background `#0A0D14`
- [x] Y-axis labels แสดงตรงตำแหน่งถูกต้องทุก panel
- [x] Legend overlay (top-right, black bg) แสดงบนทุก panel, อ่านได้ชัดเจน
- [x] Double border เมื่อ focused panel ยังทำงาน
- [x] RAID conditional panel (แสดงเฉพาะตอน rebuild) ยังทำงานเดิม
- [x] Tab/mouse focus ทุก panel ยังทำงานเดิม

---

## Known Gaps

- Y-axis label column (4 chars) มี bg สีดำ ต่างจาก canvas ที่มี zone colors — ยอมรับได้ เป็น visual separator ที่ชัดเจน
- เส้นที่มีแค่ 1 data point (ช่วงแรกก่อน history เต็ม) ไม่ได้ draw จุดเดี่ยว — `windows(2)` ไม่มีผล; เส้นจะโผล่หลังมี ≥ 2 points
