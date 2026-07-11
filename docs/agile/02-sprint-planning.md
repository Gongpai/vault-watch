# Sprint Planning & Roadmap

**Last Updated:** 2026-07-11 | **Version:** 2.0

ยินดีต้อนรับสู่แผนการดำเนินงาน HDD Monitor สำหรับขั้นตอนการพัฒนาต้นแบบ (MVP Development)

## 📅 Sprint Schedule Overview

| Sprint | Timeline | Focus | Status |
|:---|:---|:---|:---|
| [sprint-01](./sprint-backlogs/sprint-01.md) | 2026-06-10 → 2026-06-24 | **Core Data Collectors** (RAID parser, SMART parser, iostat parser, TUI foundation) | ✅ Done |
| [sprint-02](./sprint-backlogs/sprint-02.md) | 2026-06-24 → 2026-07-08 | **Dashboard UI** (RAID panel, Disk table, SMART details, Auto-refresh loop) | ✅ Done |
| [sprint-03](./sprint-backlogs/sprint-03.md) | 2026-07-08 → 2026-07-22 | **Alerts & Notifications** (Temp color coding, SMART warnings banner, Discord webhook) | ✅ Done |
| [sprint-04](./sprint-backlogs/sprint-04.md) | 2026-07-22 → 2026-08-05 | **Cross-Distribution Support** (sudo config, dependency check, musl build, README) | ✅ Done |
| [sprint-05](./sprint-backlogs/sprint-05.md) | 2026-08-05 → 2026-08-19 | **Device Discovery** (auto-detect `sd*` from `/sys/block/`, config override) | ✅ Done |
| [sprint-06](./sprint-backlogs/sprint-06.md) | 2026-08-19 → 2026-09-02 | **Graph View Improvements** (temp legend, Read/Write split, conditional multi-array RAID graph) | ✅ Done |
| [sprint-07](./sprint-backlogs/sprint-07.md) | 2026-09-02 → 2026-09-16 | **Canvas Graph Redesign** (temperature zone backgrounds, dark theme I/O graphs, unified style) | ✅ Done |
| [sprint-08](./sprint-backlogs/sprint-08.md) | 2026-09-16 → 2026-09-30 | **Graph Layout & Color Tuning** (math-based Y-axis positioning, เส้นสว่างขึ้น, zone bg มืดลง 10%) | ✅ Done |
| [sprint-09](./sprint-backlogs/sprint-09.md) | 2026-09-30 → 2026-10-14 | **Tunable Y-Axis Label Offset** (ตัวแปร `Y_LABEL_OFFSET` ปรับตำแหน่งตัวเลขแกน Y ให้ตรงเส้นแบ่ง zone) | ✅ Done |
| [sprint-10](./sprint-backlogs/sprint-10.md) | Started 2026-07-11 · sub-sprints 10A–10H | **Universal Storage Architecture & Security Hardening** | 🚧 In Progress |

---

## 🚀 Sprint Details

### 🚧 [Sprint 10: Universal Storage Architecture & Security Hardening](./sprint-backlogs/sprint-10.md)

- **เป้าหมาย:** restructure เป็น graph-first storage monitor, ย้าย external collectors ไป native backends, redesign UI และวาง privacy/privilege boundary ก่อน raw protocol access
- **โครงสร้าง:** 10A Security/Foundation → 10B Counters/MD → 10C UI → 10D SCSI → 10E SATA → 10F NVMe → 10G USB/MMC → 10H Broker/Qualification
- **Carry-over:** งาน verify/config/theme/static binary ที่ยังไม่จบจาก Sprint 05/06/09 ถูกย้ายเข้า US-MON-38
- **สถานะปัจจุบัน:** เริ่ม US-MON-28/29; protocol backends ยังไม่เปิดใช้งาน

---

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

### ✅ [Sprint 06: Graph View Improvements](./sprint-backlogs/sprint-06.md)
- **จุดมุ่งหมายหลัก:** ทำให้ Graph view อ่านได้จริงเมื่อมีหลาย disk / หลาย array — เพิ่ม legend ให้ Temperature graph, แยก Throughput เป็นช่อง Read/Write เพื่อแยกสีต่อ device ได้, และเปลี่ยน RAID Rebuild graph ให้แสดงเฉพาะตอนมี rebuild พร้อมรองรับหลาย array แยกสีเส้นต่อ array
- **ระยะเวลา:** 2 สัปดาห์ (14 วัน)
- **การประเมินผล:** ดู Graph view แล้วบอกได้ทันทีว่าเส้นไหนคือ disk/array ตัวไหนทุก chart; ไม่มี rebuild → ไม่เห็น panel ว่าง; มี rebuild 2 arrays พร้อมกัน → เห็นสองเส้นแยกสีพร้อมชื่อ

---

### ✅ [Sprint 07: Canvas Graph Redesign](./sprint-backlogs/sprint-07.md)
- **จุดมุ่งหมายหลัก:** แทนที่ `ratatui::Chart` ทุกช่องใน Graph view ด้วย `Canvas` เพื่อให้ Temperature graph แสดง zone background สี 5 ระดับตามช่วงอุณหภูมิ และ Read/Write/RAID graphs แสดง dark background แบบ unified — ทุก panel มี visual style เดียวกัน
- **ระยะเวลา:** 2 สัปดาห์ (14 วัน)
- **การประเมินผล:** ดู Temperature graph แล้วเห็น background เปลี่ยนสีตามโซน (teal → green → amber → red → purple) อย่างชัดเจน; Read/Write/RAID graph มี dark background `#0A0D14` เหมือนกัน; เส้น braille ทับ background ได้ไม่ถูกบัง

---

### ✅ [Sprint 08: Graph Layout & Color Tuning](./sprint-backlogs/sprint-08.md)
- **จุดมุ่งหมายหลัก:** (1) แก้การจัดตำแหน่งบน Graph view ให้คำนวณจากสูตรสัดส่วนเดียว (`value / max_value`) แทนการจับวาง เพื่อให้ zone background, เส้นแบ่ง zone และตัวเลขแกน Y ตรงกัน; (2) ปรับสีเส้นกราฟให้สว่างขึ้นและพื้นหลัง zone มืดลง 10% เพื่อเพิ่ม contrast
- **ระยะเวลา:** 2 สัปดาห์ (14 วัน)
- **การประเมินผล:** เส้นแบ่ง zone `60-90°C` เริ่มที่ 66.67% ของความสูงพอดี; ตัวเลข `60` อยู่แถวเดียวกับเส้นแบ่ง; zone ที่ห่างกัน 10°C สูงเท่ากัน; เส้นกราฟสว่างเด่นบนพื้นหลังที่เข้มขึ้น; ไม่มี hardcoded offset ในโค้ด

---

### ✅ [Sprint 09: Tunable Y-Axis Label Offset](./sprint-backlogs/sprint-09.md)
- **จุดมุ่งหมายหลัก:** เพิ่มตัวแปร `Y_LABEL_OFFSET` (named constant ในกลุ่ม theme, default `-0.5`) สำหรับปรับตำแหน่งตัวเลขแกน Y ให้ตรงเส้นแบ่ง zone (Sprint 08 ทำ layout zone ตรงแล้ว แต่ตัวเลขยังลอยใต้เส้นราวครึ่ง cell) — ปรับที่เดียวมีผลทุก graph โดยไม่ขยับ zone background
- **ระยะเวลา:** 2 สัปดาห์ (14 วัน)
- **การประเมินผล:** ดู Temperature graph แล้วตัวเลข `30/40/50/60` อยู่กึ่งกลางเส้นแบ่งสีพอดี ไม่ลอยใต้เส้น; ปรับค่า `Y_LABEL_OFFSET` แล้วจูนตำแหน่งได้; เส้นแบ่ง zone ยังอยู่ที่เดิมจาก Sprint 08
