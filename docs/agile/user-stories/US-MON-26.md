# US-MON-26: Configurable Graph Theme — Centralized Constants + Config Support

**Sprint:** 08 (Part A) / 10C (Part B) | **Estimate:** S (3h) + M (config) | **Status:** ✅ Done

---

## User Story

**ในฐานะ** ผู้ดูแลระบบ / ผู้พัฒนาที่อยากปรับแต่งสี graph เอง
**ฉันต้องการ** ให้ค่าสีเส้น, สี zone และเส้นแบ่งอุณหภูมิ ถูกรวมไว้ที่เดียวเป็นตัวแปรที่ชื่อชัดเจน และในอนาคตกำหนดผ่าน config file ได้
**เพื่อให้** เปลี่ยน theme ได้ง่ายโดยไม่ต้องไล่แก้หลายจุดในโค้ด และผู้ใช้ปรับแต่งเองได้โดยไม่ต้อง compile ใหม่

---

## ขอบเขต 2 ส่วน

งานนี้แบ่งเป็น 2 ส่วนชัดเจน — Part A ทำใน Sprint 08, Part B เป็น requirement สำหรับอนาคต

### 🅰 Part A — Centralized Constants (Sprint 08, 🔴 Must)

รวมค่า theme ทั้งหมดที่กระจายอยู่ใน `graph_view.rs` มาไว้ใน **block เดียว** ที่หัวไฟล์ พร้อม doc comment — แก้ที่เดียว มีผลทุกที่

**ค่าที่ต้องรวม:**
- `DISK_COLORS` — สีเส้นต่อ device/array (ใช้ทั้ง 4 graph + legend)
- `TEMP_ZONES` — ขอบเขต + สีของ 5 zone อุณหภูมิ
- `IO_BG` — สีพื้นหลัง Read/Write/RAID
- `TEMP_Y_MAX` (90.0), `IO_Y_MAX` (200.0) — ค่าสูงสุดแกน Y ที่ปัจจุบัน hardcode กระจายอยู่

### 🅱 Part B — Config-Driven Theme (Backlog, 🟡 Should — requirement)

โหลดค่า theme จาก `~/.config/hdd-monitor/config.toml` section `[graph]` — ต่อยอดจากระบบ config เดิม (`[system]`, `[discord]`)

ดู §Config Schema ด้านล่าง

---

## Acceptance Criteria

### Part A (Sprint 08)
1. ค่า theme ทั้งหมด (`DISK_COLORS`, `TEMP_ZONES`, `IO_BG`, Y-max ต่างๆ) อยู่ใน block เดียวที่หัว `graph_view.rs` พร้อม doc comment อธิบายแต่ละค่า
2. ไม่มี magic number สี/ขอบเขต/Y-max กระจายในฟังก์ชัน render ต่างๆ — ทุกที่อ้างถึง constant กลาง
3. เปลี่ยนสี 1 ค่าแล้วมีผลทั้ง graph + legend โดยแก้จุดเดียว
4. ไม่มี regression — build/clippy clean, การแสดงผลเหมือนเดิม (ค่า default = ค่าจาก US-MON-25)

### Part B (Backlog — requirement)
5. รองรับ config `[graph]` section: `line_colors`, `temp_zones`, `io_background`
6. ทุก field เป็น optional — ไม่ใส่ → ใช้ค่า default (constant จาก Part A)
7. Hex string (`"#RRGGBB"`) → parse เป็น `Color::Rgb`; ค่าผิดรูป → fallback default + ไม่ crash
8. `temp_zones` กำหนดได้ทั้งขอบเขต (°C) และสี; เรียงตาม `max` จากน้อยไปมาก
9. `config.example.toml` มี `[graph]` section พร้อมคอมเมนต์อธิบาย (commented out)

**Implemented in 0.16.0:** validated `#RRGGBB` colors, bounded line palette, strictly increasing finite temperature zones, bounded finite label offset และ runtime Graph/legend integration; invalid config แสดง error banner และใช้ safe defaults ทั้งชุด

**Hardware-qualified in 0.16.1:** custom three-color palette ถูกใช้กับ legend/series และวนซ้ำข้ามหลาย subjects พร้อม custom I/O background หลัง restart; บันทึกผลแบบ sanitized โดยไม่เก็บ device identity

---

## Config Schema (Part B)

```toml
[graph]
# สีเส้นต่อ device (hex). วนซ้ำถ้า device มากกว่าจำนวนสี
# line_colors = ["#50FAFA", "#FAE65A", "#78FA78", "#FA82FA", "#7AAAFA", "#FA825A"]

# ขอบเขต zone อุณหภูมิ (°C) + สีพื้นหลัง — เรียง max จากน้อยไปมาก
# temp_zones = [
#   { max = 30, color = "#073045" },
#   { max = 40, color = "#02320E" },
#   { max = 50, color = "#403300" },
#   { max = 60, color = "#3A0000" },
#   { max = 90, color = "#23002F" },
# ]

# พื้นหลัง solid ของ Read/Write/RAID graph
# io_background = "#0A0D14"
```

---

## Technical Notes

### Part A — `src/widgets/graph_view.rs`

จัด block theme ที่หัวไฟล์ พร้อม comment:

```rust
// ── Graph theme ───────────────────────────────────────────────────────────────
// แก้ค่าในบล็อกนี้เพื่อปรับ theme ทั้งหมด (Part B จะ override ค่าเหล่านี้จาก config)

/// สีเส้นต่อ device/array — ใช้ทั้ง 4 graph และ legend (วนซ้ำเมื่อเกิน 6)
const DISK_COLORS: [Color; 6] = [ /* bright RGB จาก US-MON-25 */ ];

/// Temperature zone: (อุณหภูมิต่ำสุด, สูงสุด, สีพื้นหลัง) — มืดลง 10% จาก US-MON-25
const TEMP_ZONES: [(f64, f64, Color); 5] = [ /* ... */ ];

/// พื้นหลัง solid ของ Read/Write/RAID graph
const IO_BG: Color = Color::Rgb(10, 13, 20);

/// ค่าสูงสุดแกน Y
const TEMP_Y_MAX: f64 = 90.0;
const IO_Y_MAX: f64 = 200.0;
```

แทนที่ `90.0` / `200.0` ที่ hardcode ในฟังก์ชัน render ด้วย constant

### Part B — `src/config.rs`

- เพิ่ม `GraphConfig` struct (optional fields) ใน `Config`
- helper `parse_hex(&str) -> Option<Color>` (`#RRGGBB` → `Color::Rgb`)
- `resolve_graph_theme(config) -> Theme` — config override → constant default
- `graph_view.rs` รับ `Theme` (จาก `AppState` หรือ param) แทนอ้าง constant ตรงๆ
- จุดเชื่อม: pattern เดียวกับ `resolve_devices()` / `smartctl_base_cmd()` ที่มี config-override-แล้ว-fallback อยู่แล้ว

---

## Related

- [US-MON-25](./US-MON-25.md) — ค่าสี default ที่ Part A จะรวมเข้า block เดียว
- [US-MON-24](./US-MON-24.md) — positioning (ใช้ `TEMP_Y_MAX`/`IO_Y_MAX` ร่วมกัน)
- [US-MON-14](./US-MON-14.md) — ระบบ config เดิม (`[system]`) ที่ Part B ต่อยอด
