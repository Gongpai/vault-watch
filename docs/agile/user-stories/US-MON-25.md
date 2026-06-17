# US-MON-25: Graph Color Tuning — เส้นสว่างขึ้น + พื้นหลัง zone มืดลง 10%

**Sprint:** 08 | **Estimate:** S (2h) | **Status:** ✅ Done

---

## User Story

**ในฐานะ** ผู้ดูแลระบบที่จ้อง Graph view เป็นเวลานาน
**ฉันต้องการ** ให้เส้นกราฟทุกหน้าใช้สีโทนสว่างขึ้น และพื้นหลัง zone ของ Temperature graph มืดลงอีก 10%
**เพื่อให้** เส้นกราฟเด่นและอ่านง่ายขึ้นบนพื้นหลังที่เข้มขึ้น — contrast ระหว่างเส้นกับพื้นหลังสูงขึ้น

---

## ปัญหาปัจจุบัน (Sprint 07)

1. **เส้นกราฟใช้ ANSI named colors** — `Color::Blue`, `Color::Red` ฯลฯ ซึ่งบาง terminal เรนเดอร์เป็นโทนเข้ม มองเห็นยากบนพื้นหลังสีเข้ม (zone background / dark IO background)
2. **พื้นหลัง zone ยังสว่างเกินไปเล็กน้อย** — contrast กับเส้นกราฟไม่พอ ทำให้เส้นไม่เด่นเท่าที่ควร

---

## Acceptance Criteria

1. **เส้นสว่างขึ้นทุก graph** — Temperature, Read, Write, RAID ใช้ palette สีสว่าง (bright RGB) ชุดเดียวกัน — สีต่อ device ยังตรงกันทุก panel เหมือนเดิม
2. **Zone background มืดลง 10%** — สีพื้นหลัง 5 zone ของ Temperature graph คูณทุก channel ด้วย `0.9`
3. **IO/RAID background ไม่เปลี่ยน** — `#0A0D14` คงเดิม (มืดอยู่แล้ว)
4. **Contrast เพิ่มขึ้นจริง** — เส้นกราฟเด่นชัดบนทุก zone color
5. **ไม่มี regression** — legend สียังตรงกับเส้น, focus border, zone boundary positioning (US-MON-24) ไม่เปลี่ยน

---

## Design Values

### เส้นกราฟ — Bright RGB palette (`DISK_COLORS`)

| # | เดิม (ANSI) | ใหม่ (bright RGB) | Hex |
|:---|:---|:---|:---|
| 1 | `Cyan`    | `Rgb(80, 250, 250)`  | `#50FAFA` |
| 2 | `Yellow`  | `Rgb(250, 230, 90)`  | `#FAE65A` |
| 3 | `Green`   | `Rgb(120, 250, 120)` | `#78FA78` |
| 4 | `Magenta` | `Rgb(250, 130, 250)` | `#FA82FA` |
| 5 | `Blue`    | `Rgb(122, 170, 250)` | `#7AAAFA` |
| 6 | `Red`     | `Rgb(250, 130, 90)`  | `#FA825A` |

### Zone backgrounds — มืดลง 10% (× 0.9)

| Zone | เดิม | RGB เดิม | ใหม่ (−10%) | RGB ใหม่ |
|:---|:---|:---|:---|:---|
| `0–30°C`  | `#08354D` | (8, 53, 77)  | `#073045` | (7, 48, 69)  |
| `30–40°C` | `#02370F` | (2, 55, 15)  | `#02320E` | (2, 50, 14)  |
| `40–50°C` | `#473900` | (71, 57, 0)  | `#403300` | (64, 51, 0)  |
| `50–60°C` | `#400000` | (64, 0, 0)   | `#3A0000` | (58, 0, 0)   |
| `60–90°C` | `#270034` | (39, 0, 52)  | `#23002F` | (35, 0, 47)  |

> สูตร: `new_channel = round(old_channel × 0.9)`

---

## Technical Notes

**`src/widgets/graph_view.rs`:**

- `DISK_COLORS` — เปลี่ยนจาก ANSI `Color::Cyan` ฯลฯ เป็น `Color::Rgb(...)` ตามตารางข้างบน (ใช้ร่วมกันทุก graph + legend อยู่แล้ว — แก้ที่เดียว)
- `TEMP_ZONES` — อัพเดทค่า `Color::Rgb(...)` ทั้ง 5 ให้เป็นค่าใหม่ที่มืดลง 10%

```rust
const DISK_COLORS: [Color; 6] = [
    Color::Rgb(80,  250, 250),
    Color::Rgb(250, 230, 90),
    Color::Rgb(120, 250, 120),
    Color::Rgb(250, 130, 250),
    Color::Rgb(122, 170, 250),
    Color::Rgb(250, 130, 90),
];

const TEMP_ZONES: [(f64, f64, Color); 5] = [
    (0.0,  30.0, Color::Rgb(7,  48, 69)),
    (30.0, 40.0, Color::Rgb(2,  50, 14)),
    (40.0, 50.0, Color::Rgb(64, 51,  0)),
    (50.0, 60.0, Color::Rgb(58,  0,  0)),
    (60.0, 90.0, Color::Rgb(35,  0, 47)),
];
```

---

## Related

- [US-MON-23](./US-MON-23.md) — Canvas redesign + zone colors เดิม (Sprint 07)
- [US-MON-24](./US-MON-24.md) — Proportional positioning (Sprint 08, story คู่กัน)
