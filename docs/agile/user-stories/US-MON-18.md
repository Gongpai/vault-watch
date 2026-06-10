# US-MON-18: Auto-detect Disk Devices

**Sprint:** 05 | **Estimate:** S (5h) | **Status:** 🟡 Planned

---

## User Story

**ในฐานะ** ผู้ดูแลระบบที่มี disk setup ต่างจาก `sdc/sdd/sde`
**ฉันต้องการ** ให้ VaultWatch ค้นหา disk device บนระบบอัตโนมัติ
**เพื่อให้** รันได้ทันทีโดยไม่ต้อง hardcode ชื่อ device ใน source code

---

## Acceptance Criteria

1. **Auto-detect** — อ่าน `/sys/block/sd*` ที่ startup แล้วใช้เป็น device list อัตโนมัติ
2. **Config override** — ถ้า config มี `[system] devices = [...]` ให้ใช้ list นั้นแทน (ไม่ auto-detect)
3. **Filter** — กรองเฉพาะ block device จริง ตัดออก: partition (`sda1`), loop (`loop0`), ram (`ram0`), virtual (`dm-*`, `md*`)
4. **Empty fallback** — ถ้าไม่พบ device ใดเลย แสดงข้อความ "No disk devices found" ใน UI แทนที่จะ crash
5. **Visible** — แสดง device list ที่ใช้งานอยู่ใน header หรือ status bar เพื่อให้ตรวจสอบได้

---

## Technical Notes

**Auto-detect logic** (`src/config.rs`):
```rust
pub fn detect_disk_devices() -> Vec<String> {
    // อ่าน /sys/block/ และกรอง sd* entries
    // ตรวจว่าเป็น physical disk (ไม่ใช่ partition) ด้วย
    // /sys/block/sdX/device/type หรือตรวจ symlink target
}
```

**Config schema** (เพิ่มใน `[system]`):
```toml
[system]
devices = ["sda", "sdb", "sdc"]   # optional — auto-detect ถ้าไม่ระบุ
```

**Resolution order:**
1. `config.toml` มี `devices` → ใช้ตามนั้น (exact list)
2. ไม่มีใน config → `detect_disk_devices()` จาก `/sys/block/`
3. ไม่พบ device → แสดง warning + รอ retry

**ของเดิมที่ต้องแก้:**
- `main.rs` — ลบ `const DISK_DEVICES` ออก, อ่านจาก `config::resolve_devices(&cfg)` แทน
- `src/config.rs` — เพิ่ม `devices: Option<Vec<String>>` ใน `SystemConfig` + `resolve_devices()` + `detect_disk_devices()`
- `docs/agile/user-stories/US-MON-17.md` (README) — เพิ่มตัวอย่าง `devices` ใน config example

---

## Related

- [US-MON-14](./US-MON-14.md) — Configurable smartctl Privilege (ใช้ config.rs เดียวกัน)
- [US-MON-15](./US-MON-15.md) — Startup Dependency Check (อาจแสดงร่วมกับ "no devices" warning)
- [contrib/config.example.toml](../../contrib/config.example.toml) — ต้องเพิ่ม `devices` option
