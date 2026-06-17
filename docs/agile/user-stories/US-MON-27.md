# US-MON-27: Tunable Y-Axis Label Offset — ตัวแปรปรับตำแหน่งตัวเลขแกน Y ให้ตรงเส้นแบ่ง zone

**Sprint:** 09 | **Estimate:** S (2h) | **Status:** 📋 Planned

---

## User Story

**ในฐานะ** ผู้ดูแลระบบที่อ่านค่าอุณหภูมิจาก Temperature graph
**ฉันต้องการ** ให้มี **ตัวแปร offset** สำหรับปรับตำแหน่งตัวเลขแกน Y ให้ตรงกับเส้นแบ่ง zone มากขึ้น (ปัจจุบันตัวเลขลอยอยู่ใต้เส้นราวครึ่งบรรทัด)
**เพื่อให้** ปรับตำแหน่งตัวเลขได้จากตัวแปรจุดเดียว โดยไม่ต้องไปแก้ค่าฝังในสูตร และจูนให้ตรงได้พอดี

---

## ปัญหาปัจจุบัน (Sprint 08)

จากการใช้งานจริง (screenshot 2026-06-17 10:51):

1. **Layout เขต temperature ถูกต้องแล้ว** — zone boundaries ตรงสัดส่วน, zone span เท่ากันสูงเท่ากัน (ผล US-MON-24) ✅
2. **แต่ตัวเลขแกน Y ยังลอยใต้เส้นแบ่ง** — label `30/40/50/60` ถูกวาด **ใต้** เส้นแบ่งสีลงมาราวครึ่ง cell
3. **ผู้ใช้ต้องการ:** ตัวแปรสำหรับปรับ offset ตำแหน่งตัวเลข เพื่อจูนให้ตรงเส้นแบ่งได้ — **ไม่ใช่** การ hardcode ค่าตายตัว แต่เป็นตัวแปรที่ตั้ง/แก้ได้ที่เดียว

### สาเหตุราก

`render_y_labels` วาง label ที่แถว `row_for_value(value) = round(row_pos(value))` โดย label กิน 1 cell แบบ **top-aligned** ขณะที่เส้นแบ่ง zone อยู่ที่ขอบบนของช่วงแถว → จุดกึ่งกลาง glyph (`row + 0.5`) อยู่ **ใต้** เส้นแบ่งราวครึ่ง cell

---

## หลักการ (Core Principle)

> **ตำแหน่งตัวเลข label = ตำแหน่งสัดส่วน + offset ที่ตั้งได้** — offset เป็น **named constant** ในกลุ่ม theme (ปรับที่เดียว มีผลทุก label) ไม่ใช่ magic number กระจายในสูตร

### ตัวแปร offset (ใหม่)

วางในกลุ่ม theme constants หัวไฟล์ `graph_view.rs` (ร่วมกับ `DISK_COLORS`/`TEMP_ZONES`/… จาก US-MON-26 Part A)

```rust
/// ปรับตำแหน่งแนวตั้งของ "ตัวเลข label" แกน Y (หน่วย: แถว)
/// label 1 cell ถูกวาดแบบ top-aligned → glyph center อยู่ที่ row + 0.5
/// ค่า -0.5 = เลื่อนขึ้นครึ่ง cell ให้ glyph center ตรงเส้นแบ่งพอดี
/// จูนค่านี้ (เช่น -1.0 ..= 0.0) เพื่อขยับตัวเลขขึ้น/ลงเทียบเส้นแบ่ง zone
const Y_LABEL_OFFSET: f64 = -0.5;
```

### สูตร

```
boundary row (เส้นแบ่ง zone — ไม่เปลี่ยน):  row_for_value(v) = round(row_pos(v))
label row    (ตัวเลข — ใหม่):               row_for_label(v) = round(row_pos(v) + Y_LABEL_OFFSET)
```

- `Y_LABEL_OFFSET = -0.5` เป็น **ค่า default** ที่มาจาก geometry (cell center = `row + 0.5`) — center glyph บนเส้นแบ่งพอดี
- เป็น **ตัวแปร** ปรับได้ ไม่ใช่ค่าฝัง — ถ้าฟอนต์/terminal ไหนตัวเลขดูเยื้อง ปรับค่านี้จุดเดียวจูนได้ทุก label
- terminal วาด text แบบ cell-granular (ไม่มี sub-cell positioning) → offset เป็นหน่วย "แถว" (รับทศนิยม เพราะปัดตอนท้าย)
- **เส้นแบ่ง zone (`ZoneBackground`) ไม่แตะ** — เฉพาะการวาง label

### เตรียมต่อ config override (US-MON-26 Part B)

`Y_LABEL_OFFSET` วางอยู่ในกลุ่ม theme block เดียวกับ constant อื่น → เมื่อทำ Part B (โหลด theme จาก `config.toml [graph]`) ให้ override ค่านี้ผ่าน `[graph] label_offset = -0.5` ได้ทันที (เพิ่มเป็น commented option ใน `config.example.toml`)

---

## Acceptance Criteria

1. **มีตัวแปร offset** — `Y_LABEL_OFFSET` เป็น named constant อยู่ในกลุ่ม theme block หัวไฟล์ พร้อม doc comment อธิบายหน่วยและวิธีจูน
2. **ปรับที่เดียวมีผลทุก label** — เปลี่ยนค่า `Y_LABEL_OFFSET` แล้วตัวเลขทุก graph ขยับตาม (ไม่มี offset ซ้ำกระจายในฟังก์ชัน)
3. **Default = -0.5 ทำให้ตัวเลข center บนเส้นแบ่ง** — `30/40/50/60` (Temperature) อยู่กึ่งกลางเส้นแบ่งสีด้วยตา ไม่ลอยใต้เส้น
4. **เส้นแบ่ง zone ไม่ขยับ** — `ZoneBackground` วาดตำแหน่งเดิม (ไม่ regression US-MON-24/25)
5. **Edge labels** — label บนสุด (`90`/`200`) และล่างสุด (`0`) `clamp` ที่ขอบ canvas (center เกินขอบไม่ได้) — ยอมรับได้
6. **เตรียม config override** — `config.example.toml` มี commented `label_offset` ใน `[graph]` (planned US-MON-26 Part B)
7. **build/clippy/test สะอาด** — `cargo clippy` ไม่มี warning, `cargo test` ผ่านทั้งหมด

---

## Technical Notes

**`src/widgets/graph_view.rs`:**

- เพิ่ม `const Y_LABEL_OFFSET: f64 = -0.5;` ในกลุ่ม theme constants (ใต้ `IO_Y_MAX`)
- เพิ่ม `row_for_label()` แยกจาก `row_for_value()`:
  ```rust
  /// แถวสำหรับวาง "ตัวเลข label" — ใช้ Y_LABEL_OFFSET จูนเทียบเส้นแบ่ง
  fn row_for_label(value: f64, y_min: f64, y_max: f64, height: u16) -> u16 {
      ((row_pos(value, y_min, y_max, height) + Y_LABEL_OFFSET).round() as i32)
          .clamp(0, height as i32 - 1) as u16
  }
  ```
- `render_y_labels` เปลี่ยนจาก `row_for_value` → `row_for_label` (จุดเดียว ครอบทุก graph เพราะ Temperature/Read/Write/RAID เรียก `render_y_labels` ร่วมกัน)
- ไม่แตะ `ZoneBackground::render`, `row_pos`, `row_for_value`

### ตาราง row mapping (height = 60, max = 90, `Y_LABEL_OFFSET = -0.5`)

| value | `row_pos` | boundary row | label row = `round(row_pos − 0.5)` | ผล |
|:---|:---|:---|:---|:---|
| 90 | 0.0   | 0          | clamp → 0  | ตรงขอบบน |
| 60 | 20.0  | 20         | round(19.5) = 20 | center ใกล้เส้น |
| 50 | 26.67 | 27         | round(26.17) = 26 | ขึ้น 1 แถว |
| 40 | 33.33 | 33         | round(32.83) = 33 | center พอดี |
| 30 | 40.0  | 40         | round(39.5) = 40  | center ใกล้เส้น |
| 0  | 60.0  | clamp → 59 | clamp → 59 | ตรงขอบล่าง |

> ปรับ `Y_LABEL_OFFSET` ไป `-1.0` จะดันตัวเลขขึ้นอีก 1 แถวเต็ม, ไป `0.0` จะกลับเป็น top-align เดิม — จูนได้ตามฟอนต์/terminal

---

## Related

- [US-MON-24](./US-MON-24.md) — Proportional layout (Sprint 08) — งานนี้ **ปรับ** AC#4 ของ US-MON-24 จาก "label แถวเดียวกับ boundary (top-align)" → "label center บน boundary ด้วย offset ที่ตั้งได้"
- [US-MON-26](./US-MON-26.md) — Centralized theme (Part A) / config override (Part B) — `Y_LABEL_OFFSET` เข้ากลุ่ม theme block เดียวกัน และเตรียม override ใน Part B
- [US-MON-25](./US-MON-25.md) — Color tuning (Sprint 08)
