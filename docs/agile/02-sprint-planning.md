# Sprint Planning & Roadmap

**Last Updated:** 2026-06-11 | **Version:** 1.5

ยินดีต้อนรับสู่แผนการดำเนินงาน HDD Monitor สำหรับขั้นตอนการพัฒนาต้นแบบ (MVP Development)

## 📅 Sprint Schedule Overview

| Sprint | Timeline | Focus | Status |
|:---|:---|:---|:---|
| [sprint-01](./sprint-backlogs/sprint-01.md) | 2026-06-10 → 2026-06-24 | **Core Data Collectors** (RAID parser, SMART parser, iostat parser, TUI foundation) | ✅ Done |
| [sprint-02](./sprint-backlogs/sprint-02.md) | 2026-06-24 → 2026-07-08 | **Dashboard UI** (RAID panel, Disk table, SMART details, Auto-refresh loop) | ✅ Done |
| [sprint-03](./sprint-backlogs/sprint-03.md) | 2026-07-08 → 2026-07-22 | **Alerts & Notifications** (Temp color coding, SMART warnings banner, Discord webhook) | ✅ Done |
| [sprint-04](./sprint-backlogs/sprint-04.md) | 2026-07-22 → 2026-08-05 | **Cross-Distribution Support** (sudo config, dependency check, musl build, README) | ✅ Done |
| [sprint-05](./sprint-backlogs/sprint-05.md) | 2026-08-05 → 2026-08-19 | **Device Discovery** (auto-detect `sd*` from `/sys/block/`, config override) | ✅ Done |
| [sprint-06](./sprint-backlogs/sprint-06.md) | 2026-08-19 → 2026-09-02 | **Graph View Improvements** (temp legend, Read/Write split, conditional multi-array RAID graph) | 🟡 Planned |

---

## 🚀 Sprint Details

### ✅ [Sprint 01: Core Data Collectors](./sprint-backlogs/sprint-01.md)
- **จุดมุ่งหมายหลัก:** พัฒนา async data collector สำหรับทุก data source ที่จำเป็น (`/proc/mdstat`, `smartctl`, `iostat`) พร้อมโครงสร้าง TUI application พื้นฐานที่รัน event loop, keyboard handling และ shared state ได้ครบถ้วน
- **ระยะเวลา:** 2 สัปดาห์ (14 วัน)
- **การประเมินผล:** สามารถรัน binary และเห็น raw data (แม้ยังไม่สวยงาม) จาก RAID, SMART และ iostat บน terminal ได้ถูกต้อง

---

### ✅ [Sprint 02: Dashboard UI](./sprint-backlogs/sprint-02.md)
- **จุดมุ่งหมายหลัก:** สร้าง ratatui UI ครบทั้ง 3 panels (RAID panel, Disk table, SMART details) และเชื่อม auto-refresh loop เพื่อให้ได้ dashboard ที่ใช้งานได้จริง ตามแบบที่กำหนดใน [System Design](../software/01-system-design.md)
- **ระยะเวลา:** 2 สัปดาห์ (14 วัน)
- **การประเมินผล:** เปิด terminal, รัน `sudo ./hdd-monitor` แล้วเห็น dashboard ครบ 3 panels อัปเดตทุก 2 วินาที และ keyboard `q`/`r` ทำงานถูกต้อง

---

### ✅ [Sprint 03: Alerts & Notifications](./sprint-backlogs/sprint-03.md)
- **จุดมุ่งหมายหลัก:** เพิ่มระบบแจ้งเตือนให้ dashboard สามารถแจ้งเตือนปัญหาได้ทั้งบน UI (color coding, banner) และ out-of-band (Discord webhook) เพื่อให้ผู้ดูแลระบบรับรู้เหตุการณ์สำคัญแม้ไม่ได้มองหน้าจออยู่
- **ระยะเวลา:** 2 สัปดาห์ (14 วัน)
- **การประเมินผล:** disk ที่ร้อนเกิน 55°C แสดง WARN บน UI, Discord ได้รับ message เมื่อ RAID degraded

---

### ✅ [Sprint 04: Cross-Distribution Support](./sprint-backlogs/sprint-04.md)
- **จุดมุ่งหมายหลัก:** ให้ VaultWatch รันได้บน Ubuntu/Debian, Fedora, openSUSE, Arch Linux, Alpine Linux และ Docker โดยไม่ต้อง patch code — แก้ `sudo` hardcode, เพิ่ม dependency check, musl static build และ installation docs ครบทุก distro
- **ระยะเวลา:** 2 สัปดาห์ (14 วัน)
- **การประเมินผล:** รัน `make build-static` บน Ubuntu แล้วนำ binary ไปรันบน Alpine Docker ได้ทันที, ผู้ใช้ใหม่ที่ไม่มี smartmontools เห็น error screen พร้อม install command ถูกต้องตาม distro

---

### ✅ [Sprint 05: Device Discovery](./sprint-backlogs/sprint-05.md)
- **จุดมุ่งหมายหลัก:** ลบ `const DISK_DEVICES` hardcode ออกจาก source code และแทนที่ด้วย auto-detect จาก `/sys/block/sd*` พร้อม config override ผ่าน `[system] devices = [...]`
- **ระยะเวลา:** 2 สัปดาห์ (14 วัน)
- **การประเมินผล:** รัน VaultWatch บน machine ที่มี disk เป็น `sda`, `sdb` (ไม่ใช่ `sdc`) แล้วเห็น device ปรากฏถูกต้องโดยไม่ต้องแก้ code

---

### 🟡 [Sprint 06: Graph View Improvements](./sprint-backlogs/sprint-06.md)
- **จุดมุ่งหมายหลัก:** ทำให้ Graph view อ่านได้จริงเมื่อมีหลาย disk / หลาย array — เพิ่ม legend ให้ Temperature graph, แยก Throughput เป็นช่อง Read/Write เพื่อแยกสีต่อ device ได้, และเปลี่ยน RAID Rebuild graph ให้แสดงเฉพาะตอนมี rebuild พร้อมรองรับหลาย array แยกสีเส้นต่อ array
- **ระยะเวลา:** 2 สัปดาห์ (14 วัน)
- **การประเมินผล:** ดู Graph view แล้วบอกได้ทันทีว่าเส้นไหนคือ disk/array ตัวไหนทุก chart; ไม่มี rebuild → ไม่เห็น panel ว่าง; มี rebuild 2 arrays พร้อมกัน → เห็นสองเส้นแยกสีพร้อมชื่อ
