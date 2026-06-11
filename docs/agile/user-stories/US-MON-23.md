# US-MON-23: Canvas Graph Redesign — Zone Backgrounds + Unified Dark Theme

**Sprint:** 07 | **Estimate:** M (7h) | **Status:** ✅ Done

---

## User Story

**ในฐานะ** ผู้ดูแลระบบที่ดู Graph view เป็นประจำ
**ฉันต้องการ** ให้ graph ทุกช่องมี background สีตาม theme — Temperature ใช้สีโซนตามระดับความร้อน, Read/Write/RAID ใช้ dark background เดียวกัน
**เพื่อให้** อ่านค่าได้ง่ายขึ้นและ Graph view มี visual style ที่สอดคล้องกันทั้งหมด

---

## ปัญหาปัจจุบัน

1. ทุก chart ใช้ `ratatui::widgets::Chart` ซึ่งมี background เป็น terminal default (โปร่งใส) — ไม่มี visual context ว่าค่าที่เห็นอยู่ใน zone ไหน
2. `Chart` ไม่รองรับ filled background rectangles ต่อ zone — ต้องการ zone backgrounds ที่ different สีต่อช่วงอุณหภูมิ
3. Graph ต่างช่องมี visual style ต่างกัน — Temperature chart กับ Read/Write chart ดูไม่เป็น theme เดียวกัน

---

## Acceptance Criteria

1. **Temperature zones** — background แบ่ง 5 โซนตามระดับความร้อน พร้อมสีที่แยกแยะได้ชัดเจน:
   - `0–30°C` → `#08354D` (dark teal — ปลอดภัย)
   - `30–40°C` → `#02370F` (dark green — ปกติ)
   - `40–50°C` → `#473900` (dark amber — อุ่น)
   - `50–60°C` → `#400000` (dark red — ร้อน)
   - `60–90°C` → `#270034` (dark purple — อันตราย)
2. **I/O และ RAID graphs** — solid dark background `#0A0D14` เหมือนกันทั้งสาม panel
3. **เส้น graph อ่านได้** — braille lines แสดงบน zone background ได้โดย foreground สีของแต่ละ device ไม่ถูกบัง
4. **Threshold lines** — เส้น 45°C (yellow) และ 55°C (red) บน Temperature graph ยังแสดงอยู่
5. **Y-axis labels** — แสดงตรงตำแหน่งถูกต้องในคอลัมน์ซ้ายของแต่ละ panel:
   - Temperature: `0`, `45°`, `55°`, `90` (แต่ละสีตาม threshold)
   - Read/Write: `0`, `100`, `200`
   - RAID: `0`, `mid`, `max` (dynamic)
6. **Legend overlay** — legend ยังคงอยู่มุมขวาบน, background สีดำ, อ่านได้ชัดเจนบนทุก zone color
7. **Focus style** — double border เมื่อ focused ยังทำงานเหมือนเดิม
8. **ไม่มี regression** — `make build` ผ่านไม่มี error/warning; behavior เดิมของ RAID conditional panel, Tab focus, mouse click ยังทำงานปกติ

---

## Design Details

### Zone Background Rendering (2-pass approach)

ปัญหา: `ratatui::Canvas` ไม่รองรับ filled rectangle — มีแต่ outline `Rectangle` shape

แนวทาง:
1. **Pass 1 — Canvas** (background_color = `Color::Reset`): วาดเส้น braille lines ลง buffer; cells มี `fg=line_color`, `bg=Reset`
2. **Pass 2 — `ZoneBackground` widget**: walk ทุก cell ใน canvas_area, คำนวณ zone จาก row position → `cell.set_bg(zone_color)`; ไม่แตะ character หรือ foreground

ผลลัพธ์: braille chars จาก Canvas แสดงบน zone background color ✅

```
cell หลัง Canvas:    char=⠿  fg=Cyan    bg=Reset
cell หลัง ZoneBg:   char=⠿  fg=Cyan    bg=#473900  ✅
```

### Layout ต่อ panel

```
┌─ Title ───────────────────────────────────────┐
│ 90 │                                  ┌─────┐ │
│    │   Canvas area                    │█ sda│ │
│55° │   (zone backgrounds + lines)     │█ sdb│ │
│    │                                  └─────┘ │
│ 45°│                                          │
│    │                                          │
│  0 │                                          │
└────┴──────────────────────────────────────────┘
  4ch  remaining width
```

### IO/RAID Background (1-pass)

`Canvas.background_color(Color::Rgb(10, 13, 20))` — Canvas เติม bg โดยตรง ไม่ต้องการ ZoneBackground widget

---

## Technical Notes

**`src/widgets/graph_view.rs`** (ไฟล์เดียวที่เปลี่ยน):
- Replace `Chart` import → `Canvas` (จาก `ratatui::widgets::canvas`)
- เพิ่ม `ZoneBackground<'a>` struct implementing `Widget` — `render()` ทำ `cell.set_bg()` per row
- เพิ่ม helpers: `panel_block()`, `draw_line_series()`, `render_y_labels()`, `render_legend()`
- Rewrite: `render_temp_graph()`, `render_io_graph()`, `render_raid_graph()`
- `render()` entry point + `history_to_points()` ไม่เปลี่ยน

**Constants เพิ่ม:**
```rust
const TEMP_ZONES: [(f64, f64, Color); 5] = [
    (0.0,  30.0, Color::Rgb(8,  53, 77)),
    (30.0, 40.0, Color::Rgb(2,  55, 15)),
    (40.0, 50.0, Color::Rgb(71, 57,  0)),
    (50.0, 60.0, Color::Rgb(64,  0,  0)),
    (60.0, 90.0, Color::Rgb(39,  0, 52)),
];
const IO_BG: Color = Color::Rgb(10, 13, 20);
```

---

## Related

- [US-MON-12](./US-MON-12.md) — Graph View เดิม (Chart widget, Sprint 02)
- [US-MON-20](./US-MON-20.md) — Temperature legend (Sprint 06)
- [US-MON-21](./US-MON-21.md) — Read/Write split (Sprint 06)
- [US-MON-22](./US-MON-22.md) — RAID rebuild graph (Sprint 06)
