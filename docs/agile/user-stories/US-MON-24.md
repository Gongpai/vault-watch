# US-MON-24: Proportional Graph Layout — คำนวณตำแหน่งด้วยคณิต ไม่จับวางเอง

**Sprint:** 08 | **Estimate:** S (4h) | **Status:** ✅ Done

---

## User Story

**ในฐานะ** ผู้ดูแลระบบที่อ่านค่าอุณหภูมิจาก Temperature graph
**ฉันต้องการ** ให้ zone background และตัวเลขแกน Y อยู่ตรงตำแหน่งที่ถูกต้องตามสัดส่วนอุณหภูมิจริง
**เพื่อให้** อ่านได้ว่า disk อยู่ใน zone ไหนจริงๆ โดยเส้นกราฟ, เส้นแบ่ง zone และตัวเลขกำกับ ตรงกันทุกจุด

---

## ปัญหาปัจจุบัน (Sprint 07)

จากการใช้งานจริง (screenshot 2026-06-11):

1. **Zone boundaries ไม่ตรงกับอุณหภูมิจริง** — เส้นแบ่งสีไม่ได้อยู่ที่ตำแหน่งสัดส่วนที่ถูกต้อง เช่น zone ม่วง `60-90°C` ควรเริ่มที่ `60/90 = 66.67%` จากล่าง แต่กลับไม่ตรง
2. **ตัวเลขแกน Y ตำแหน่งไม่ตรง** — label ในคอลัมน์ซ้าย (`30`, `40`, `50`, `60`) ไม่ได้อยู่แถวเดียวกับเส้นแบ่ง zone ที่มันกำกับ
3. **Zone ที่ห่างเท่ากันแสดงไม่เท่ากัน** — zone `30-40`, `40-50`, `50-60` ห่างกัน 10°C เท่ากัน ควรสูงเท่ากัน (`10/90 = 11.1%` ละ) แต่กลับสูงไม่เท่ากันเพราะ rounding/manual placement ไม่สอดคล้อง
4. **สาเหตุราก:** การจัดตำแหน่งของ `render_y_labels` (truncate `ratio * height`) กับ `ZoneBackground` (sample row midpoint `(row+0.5)/height`) ใช้วิธี discretize ต่างกัน → ตำแหน่งของค่าเดียวกันคลาดกัน 1 แถว

---

## หลักการใหม่ (Core Principle)

> **ทุกตำแหน่งบนแกน Y ต้องคำนวณจากสูตรสัดส่วนเดียวกัน — ห้ามจับวาง (hardcode/manual offset) เด็ดขาด**

### สูตรหลัก

```
ratio        = value / max_value          # สัดส่วนจากล่าง (0.0 = ล่างสุด, 1.0 = บนสุด)
row_from_top = (1 - ratio) * height       # แถวจากบน (canvas row 0 = บนสุด)
```

ตัวอย่าง (max = 90°C):

| ค่า | ratio = value/90 | สัดส่วนจากล่าง |
|:---|:---|:---|
| `0°C`  | 0/90 = 0.0000   | 0%     |
| `30°C` | 30/90 = 0.3333  | 33.33% |
| `40°C` | 40/90 = 0.4444  | 44.44% |
| `50°C` | 50/90 = 0.5556  | 55.56% |
| `60°C` | 60/90 = 0.6667  | 66.67% |
| `90°C` | 90/90 = 1.0000  | 100%   |

ตัวอย่างจอสูง 1024 px: `60°C → 0.666667 × 1024 = 682.67 px` จากล่าง

### Zone heights เป็นสัดส่วนกับช่วงอุณหภูมิ

| Zone | ช่วง | สัดส่วน = span/90 | สี |
|:---|:---|:---|:---|
| Teal   | `0–30°C`  | 30/90 = **33.3%** | `#08354D` |
| Green  | `30–40°C` | 10/90 = **11.1%** | `#02370F` |
| Amber  | `40–50°C` | 10/90 = **11.1%** | `#473900` |
| Red    | `50–60°C` | 10/90 = **11.1%** | `#400000` |
| Purple | `60–90°C` | 30/90 = **33.3%** | `#270034` |

→ zone ที่ span เท่ากัน (10°C) สูงเท่ากันเสมอ; zone 30°C สูงเป็น 3 เท่าของ zone 10°C

---

## Acceptance Criteria

1. **สูตรเดียวกันทั้งหมด** — zone boundary กับ label ของค่าเดียวกัน ใช้ฟังก์ชันคำนวณตำแหน่งตัวเดียวกัน → อยู่แถวเดียวกันเป๊ะ (ไม่คลาด 1 แถว)
2. **Zone boundaries ตรงสัดส่วน** — เส้นแบ่งแต่ละ zone อยู่ที่ `boundary_temp / max_temp` ของความสูง canvas
3. **Equal-span zones เท่ากัน** — `30-40`, `40-50`, `50-60` สูงเท่ากันทุก frame (ต่างได้ไม่เกิน 1 แถวจาก integer rounding ที่หลีกเลี่ยงไม่ได้)
4. **Label ตรง boundary** — ตัวเลข `30/40/50/60` อยู่แถวเดียวกับเส้นแบ่ง zone ที่มันกำกับ
5. **ใช้ได้กับทุก graph** — สูตรเดียวกันใช้กับ Read/Write (max 200) และ RAID (max dynamic) ด้วย ไม่ใช่เฉพาะ Temperature
6. **ไม่มี manual offset** — ไม่มีการบวก/ลบ pixel หรือแถวแบบ hardcode เพื่อ "ขยับให้ตรง"

---

## Technical Notes

**`src/widgets/graph_view.rs`:**

- เพิ่ม helper ตำแหน่งกลางตัวเดียว ใช้ทั้ง zone background และ label:
  ```rust
  /// แถว (offset จากบนของ area) ของค่า `value` บนแกน [y_min, y_max]
  fn row_for_value(value: f64, y_min: f64, y_max: f64, height: u16) -> u16 {
      let ratio = (value - y_min) / (y_max - y_min);
      (((1.0 - ratio) * height as f64).round() as i32)
          .clamp(0, height as i32 - 1) as u16
  }
  ```
- `ZoneBackground::render` — แทนที่ midpoint sampling ด้วยการคำนวณช่วงแถวของแต่ละ zone จาก `row_for_value(hi)` ถึง `row_for_value(lo)` → เติมสีทั้งช่วง (เส้นแบ่งตรงกับ label แน่นอน)
- `render_y_labels` — ใช้ `row_for_value` ตัวเดียวกัน
- ปรับ rounding ให้ตรงกัน (ใช้ `.round()` ทั้งคู่ แทน truncate)

> หมายเหตุ: integer terminal rows ทำให้สัดส่วนจริงคลาดได้ ±0.5 แถว เป็นข้อจำกัดของ cell grid ที่ยอมรับได้ — แต่ zone boundary กับ label **ต้องใช้สูตรเดียวกัน** จึงคลาดไปด้วยกัน (ยังตรงกันเอง)

---

## Related

- [US-MON-23](./US-MON-23.md) — Canvas graph redesign + zone backgrounds (Sprint 07) — งานที่ requirement นี้มาแก้
- [US-MON-12](./US-MON-12.md) — Graph View เดิม
