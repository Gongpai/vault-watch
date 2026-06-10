# Sprint 01: Core Data Collectors

**Goal:** พัฒนา async data collector ครบทุก data source (`/proc/mdstat`, `smartctl`, `iostat`) พร้อมโครงสร้าง TUI application พื้นฐานที่มี event loop, keyboard handling และ shared state พร้อมสำหรับต่อยอด UI ใน Sprint 02
**Timeline:** 2026-06-10 → 2026-06-24

## 📅 Internal Timeline

```mermaid
gantt
    title Sprint 01 Tasks
    dateFormat  YYYY-MM-DD
    section Foundation
    Project Setup & Dependencies (US-MON-04 pt.1)  :active, t1, 2026-06-10, 2d
    section Collectors
    RAID Parser - /proc/mdstat (US-MON-01)          :t2, after t1, 3d
    SMART Collector - smartctl (US-MON-02)          :t3, after t1, 3d
    iostat Collector (US-MON-03)                    :t4, after t2, 2d
    section Application
    TUI Foundation - main loop & state (US-MON-04)  :t5, after t3, 3d
    section Integration
    Integration & Manual Testing                    :t6, after t5, 2d
```

---

## 📋 Committed Stories & Tasks

| ID | Story / Task | Owner | Estimate (Hrs) | Status |
|:---|:---|:---|:---|:---|
| [US-MON-04](../user-stories/US-MON-04.md) | **TUI Foundation**<br>- ตั้งค่า Cargo.toml dependencies<br>- สร้าง tokio runtime, crossterm terminal setup<br>- สร้าง `AppState` struct และ `Arc<Mutex<>>` sharing<br>- Keyboard handler: `q` quit, `r` refresh | kong | 8 | 🚧 Planned |
| [US-MON-01](../user-stories/US-MON-01.md) | **RAID Status Parser**<br>- อ่าน `/proc/mdstat` ด้วย `tokio::fs::read_to_string`<br>- Parse array name, state, disk count<br>- Parse rebuild %, speed, ETA ด้วย regex<br>- Handle active / rebuilding / degraded / no-array | kong | 6 | 🚧 Planned |
| [US-MON-02](../user-stories/US-MON-02.md) | **SMART Data Collector**<br>- รัน `smartctl -a -d scsi /dev/sdX` ด้วย `tokio::process::Command`<br>- Parse temperature, health, serial<br>- Parse power-on hours, grown defects, non-medium errors<br>- Handle sudo, timeout, disk not found | kong | 8 | 🚧 Planned |
| [US-MON-03](../user-stories/US-MON-03.md) | **iostat Collector**<br>- รัน `iostat -d -k sdc sdd sde` ด้วย async process<br>- Parse Read/Write kB/s → MB/s per device<br>- Handle iostat ไม่ติดตั้ง | kong | 4 | 🚧 Planned |

---

## 🛠 Sprint Specifics

### Definition of Done (DoD)

- Rust code compile ได้ไม่มี warning หรือ error (`cargo build --release`)
- `cargo clippy` ผ่านโดยไม่มี warning
- RAID parser อ่านค่าจาก `/proc/mdstat` จริงได้ถูกต้อง (ตรวจสอบด้วย `cat /proc/mdstat`)
- SMART collector ดึงอุณหภูมิและ health จาก `smartctl` ตรงกับ manual run
- iostat collector แสดง throughput ตรงกับ `iostat -d -k sdc sdd sde` manual run
- TUI application เปิด-ปิด terminal mode ได้สะอาด ไม่ทิ้ง terminal ค้าง
- Error handling ไม่ panic — ใช้ `Result`/`Option` ครบ

### Known Risks

| ความเสี่ยง | แนวทางแก้ไข |
|:---|:---|
| `smartctl` ต้องการ `sudo` | ตรวจสอบ sudoers config บน target server ก่อน; ถ้าไม่ได้ → เพิ่ม error message ชัดเจน |
| `iostat` อาจไม่ติดตั้ง | ตรวจ exit code → แสดง `N/A` แทน panic |
| Format `/proc/mdstat` อาจต่างในบางเคส | ใช้ `Option<>` สำหรับทุก field ที่อาจไม่มี |
