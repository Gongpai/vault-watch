# Sprint 05 — Device Discovery & UX Improvements

**Version:** 0.5.0 | **Duration:** 2026-08-05 → 2026-08-19 | **Status:** 🟡 Planned

---

## Sprint Goal

ให้ VaultWatch ค้นหา disk device บนระบบอัตโนมัติ ไม่ต้อง hardcode `sdc/sdd/sde` ใน source code อีกต่อไป และปรับปรุง usability ด้วย key hint bar แบบ nano ที่ด้านล่างหน้าจอ

---

## User Stories

| ID | Story | Estimate | Priority |
|:---|:------|:---------|:---------|
| [US-MON-18](../user-stories/US-MON-18.md) | Auto-detect Disk Devices + Config Override | **S** (5h) | 🔴 Must |
| [US-MON-19](../user-stories/US-MON-19.md) | Key Hint Bar (nano-style) | **S** (3h) | 🟡 Should |

**Total estimate:** 8h

---

## Implementation Plan

### US-MON-18 — Auto-detect Disk Devices (5h)

**ลำดับการทำงาน:**

1. **`src/config.rs`** — เพิ่ม 3 functions:
   - `detect_disk_devices() -> Vec<String>` — อ่าน `/sys/block/` กรอง `sd*`
   - `resolve_devices(config: &Config) -> Vec<String>` — config override หรือ auto-detect
   - เพิ่ม `devices: Option<Vec<String>>` ใน `SystemConfig`

2. **`src/main.rs`** — แทนที่ `const DISK_DEVICES`:
   ```rust
   // เดิม
   const DISK_DEVICES: &[&str] = &["sdc", "sdd", "sde"];
   
   // ใหม่
   let devices = config::resolve_devices(&cfg);
   ```

3. **`src/app.rs`** — ไม่ต้องแก้ (`disk_devices: Vec<String>` รับ dynamic list อยู่แล้ว)

4. **`src/ui.rs`** — เพิ่ม device count ใน header bar

5. **`contrib/config.example.toml`** — เพิ่ม `devices` option พร้อม comment

---

## Auto-detect Logic

```
/sys/block/
├── sda       → ✅ include (physical SAS/SATA)
├── sdb       → ✅ include
├── sdc       → ✅ include
├── loop0     → ❌ skip (loop device)
├── ram0      → ❌ skip (ram disk)
├── dm-0      → ❌ skip (device mapper / LVM)
└── md0       → ❌ skip (mdadm RAID — monitored separately)
```

Filter rule: ชื่อต้องขึ้นต้นด้วย `sd` เท่านั้น

---

## Definition of Done

- [ ] `make build` ผ่านไม่มี error ใหม่
- [ ] `cargo test` ผ่านทั้งหมด
- [ ] รันบน machine ที่มี disk ต่าง setup (เช่น `sda`, `sdb`) แล้วเห็น device ถูกต้อง
- [ ] Config override `devices = [...]` ทำงานถูกต้อง
- [ ] ไม่พบ device → แสดง warning แทน crash
- [ ] `contrib/config.example.toml` มี `devices` option

**US-MON-19:**
- [ ] Key hint bar แสดงที่ด้านล่างสุดตลอดเวลา (Table + Graph view)
- [ ] Key label มี invert background (nano-style)
- [ ] Shortcuts เปลี่ยนตาม view mode ปัจจุบัน
- [ ] ไม่ล้น terminal แคบ
- [ ] Header ไม่มี shortcut ซ้ำซ้อนแล้ว

---

---

### US-MON-19 — Key Hint Bar (3h)

**ลำดับการทำงาน:**

1. **`src/ui.rs`** — เพิ่ม `render_key_bar(f, area, state)` function
   - สร้าง Line จาก vec ของ `(key_label, action_label)` pairs
   - Key label ใช้ `Style fg(Black) bg(Cyan)` (invert — เหมือน nano)
   - Action label ใช้ `Style fg(Gray)`
   - Context-aware: เปลี่ยน shortcuts ตาม `state.view_mode` และ `state.focused_panel`

2. **`src/ui.rs`** — ปรับ layout constraints ใน `render_table_view` และ `render_graph_view`
   - เพิ่ม `Constraint::Length(1)` ที่ท้ายสุด

3. **`src/ui.rs`** — ลบ key hints ออกจาก `render_header()` (ย้ายมาที่ bar แล้ว)

---

## Known Risks

| Risk | Mitigation |
|:-----|:-----------|
| `/sys/block/` ไม่มีบน container | Fallback gracefully, แสดง "Cannot detect devices" warning |
| NVMe (`nvme0n1`) ต้องการ parser ต่างออกไป | Sprint 05 scope แค่ `sd*` — NVMe เป็น future story |
| User อาจ include `md*` โดยตั้งใจ | Config override รับทุก string — ผู้ใช้รับผิดชอบเอง |
