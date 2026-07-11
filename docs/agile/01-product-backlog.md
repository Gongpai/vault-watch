# HDD Monitor — Product Backlog

**Last Updated:** 2026-07-11 | **Version:** 3.0

นี่คือรายการ User Story ทั้งหมดของโปรเจค HDD Monitor แบ่งตามลำดับความสำคัญ

---

## 🔴 Must Have (MVP Scope)

| ID | User Story | Acceptance Criteria | Estimate | Status |
|:---|:---|:---|:---|:---|
| [US-MON-01](./user-stories/US-MON-01.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** อ่านและ parse สถานะ RAID จาก `/proc/mdstat`<br>**เพื่อให้** ได้ข้อมูล rebuild %, speed และ ETA พร้อมใช้งาน | 1. Parse array name, state, disk count<br>2. Parse rebuild %, speed (MB/s), ETA (นาที)<br>3. Handle กรณี no rebuild, degraded, active | **M** | ✅ Done |
| [US-MON-02](./user-stories/US-MON-02.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** รัน `smartctl` และ parse ผลลัพธ์สำหรับแต่ละ SAS disk<br>**เพื่อให้** ได้อุณหภูมิ, health status, serial และ error counts | 1. รัน `smartctl -a -d scsi /dev/sdX` ด้วย async process<br>2. Parse temperature, health, serial, power-on hours<br>3. Parse grown defects, non-medium errors, read/write errors<br>4. Handle disk ที่ไม่ตอบสนอง | **M** | ✅ Done |
| [US-MON-03](./user-stories/US-MON-03.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** รัน `iostat` และ parse throughput ของแต่ละ disk<br>**เพื่อให้** เห็น Read MB/s และ Write MB/s per disk แบบ realtime | 1. รัน `iostat -d -k` ด้วย async process<br>2. Parse Read MB/s และ Write MB/s ต่อ device<br>3. Handle กรณี iostat ไม่ติดตั้ง | **S** | ✅ Done |
| [US-MON-04](./user-stories/US-MON-04.md) | **ในฐานะ** นักพัฒนา<br>**ฉันต้องการ** โครงสร้าง TUI application พื้นฐาน<br>**เพื่อให้** มี main loop, terminal setup, keyboard handling และ shared state พร้อมสำหรับต่อยอด | 1. Terminal raw mode เปิด/ปิดได้สะอาด<br>2. AppState share ระหว่าง collector และ render task<br>3. Keyboard: `q` = quit, `r` = force refresh<br>4. Error handling ไม่ leave terminal ค้าง | **M** | ✅ Done |
| [US-MON-05](./user-stories/US-MON-05.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** panel แสดงสถานะ RAID<br>**เพื่อให้** เห็น array name, state, rebuild progress, speed และ ETA บนหน้าจอ | 1. แสดง array name และ state (Active/Rebuilding/Degraded)<br>2. แสดง progress bar rebuild<br>3. แสดง speed (MB/s) และ ETA<br>4. แสดง disk count (active/total) | **M** | ✅ Done |
| [US-MON-06](./user-stories/US-MON-06.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** ตารางสรุปข้อมูล disk แต่ละตัว<br>**เพื่อให้** เห็น Temp, Health, Read MB/s, Write MB/s และ Defects ในแถวเดียว | 1. แสดง table ที่มีคอลัมน์ Disk, Temp, Health, Read, Write, Defects<br>2. Color highlight: WARN สำหรับ temp > 55°C, ERROR สำหรับ health != OK<br>3. Alignment ถูกต้องและไม่ล้น terminal | **M** | ✅ Done |
| [US-MON-07](./user-stories/US-MON-07.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** panel แสดง SMART details แต่ละ disk<br>**เพื่อให้** เห็น serial, power-on hours, non-medium errors และ grown defects อย่างละเอียด | 1. แสดง serial number ต่อ disk<br>2. แสดง power-on hours<br>3. แสดง non-medium errors และ grown defects<br>4. Highlight ค่าที่ไม่ปกติ (defects > 0) | **S** | ✅ Done |
| [US-MON-08](./user-stories/US-MON-08.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** ให้หน้าจอ refresh อัตโนมัติทุก 2 วินาที<br>**เพื่อให้** ข้อมูล RAID, SMART และ throughput อัปเดตต่อเนื่องโดยไม่ต้องกด manual | 1. Collector loop ทำงานทุก 2 วินาที<br>2. Render loop ทำงานทุก 250ms (smooth UI)<br>3. แสดง last updated timestamp<br>4. `r` key force refresh ทันที | **M** | ✅ Done |
| [US-MON-12](./user-stories/US-MON-12.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** ให้ค่าตัวเลข (Temperature, Read/Write MB/s, RAID speed) แสดงเป็น graph<br>**เพื่อให้** เห็น trend และ pattern ได้ทันที ไม่ใช่แค่ค่า snapshot ปัจจุบัน | 1. History buffer เก็บ 60 sample ต่อ metric<br>2. Inline Sparkline ในทุกคอลัมน์ตัวเลขของ disk table<br>3. Full line chart ใน Graph View (`g` toggle)<br>4. Temperature, Throughput, RAID speed charts | **M** | ✅ Done |
| [US-MON-13](./user-stories/US-MON-13.md) | **ในฐานะ** ผู้ดูแลระบบที่ติดตั้ง HDD มากกว่า 3–5 ลูก<br>**ฉันต้องการ** scroll ภายใน panel ด้วย mouse wheel หรือ keyboard และสลับ focus ระหว่าง panel ด้วย Tab<br>**เพื่อให้** ดูข้อมูล disk ทุกลูกได้แม้หน้าจอ terminal มีพื้นที่จำกัด | 1. `Tab`/`Shift+Tab` สลับ focus panel<br>2. `↑↓`/`jk`/`PgUp`/`PgDn` scroll focused panel<br>3. Mouse wheel scroll panel ที่เมาส์อยู่<br>4. Mouse click โฟกัส panel<br>5. Focused panel แสดง double border<br>6. Scrollbar widget ทุก panel ที่ scroll ได้ | **M** | ✅ Done |

---

## 🟡 Should Have (Future Enhancements)

| ID | User Story | Acceptance Criteria | Estimate | Status |
|:---|:---|:---|:---|:---|
| [US-MON-09](./user-stories/US-MON-09.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** ให้อุณหภูมิ disk มีสีบ่งบอกระดับความร้อน<br>**เพื่อให้** เห็นสถานะอันตรายได้ทันทีโดยไม่ต้องอ่านตัวเลข | 1. < 45°C = สีเขียว<br>2. 45–55°C = สีเหลือง<br>3. > 55°C = สีแดง + ข้อความ WARN | **S** | ✅ Done |
| [US-MON-10](./user-stories/US-MON-10.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** การแจ้งเตือนเมื่อ SMART threshold ถูกละเมิด<br>**เพื่อให้** รู้ทันทีเมื่อ disk มีปัญหาที่อาจนำไปสู่ความเสียหาย | 1. Warning เมื่อ grown defects > 0<br>2. Alert เมื่อ health != OK<br>3. แสดง notification ชัดเจนบน UI | **M** | ✅ Done |
| [US-MON-11](./user-stories/US-MON-11.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** ส่ง alert ไป Discord webhook เมื่อเกิดเหตุการณ์สำคัญ<br>**เพื่อให้** รับการแจ้งเตือนแม้ไม่ได้นั่งดูหน้าจออยู่ | 1. Config webhook URL ผ่าน config file<br>2. Alert เมื่อ RAID degraded<br>3. Alert เมื่อ temp > 60°C<br>4. Alert เมื่อ SMART health != OK | **L** | ✅ Done |

---

## 🟣 Device Discovery (Sprint 05)

| ID | User Story | Acceptance Criteria | Estimate | Status |
|:---|:---|:---|:---|:---|
| [US-MON-18](./user-stories/US-MON-18.md) | **ในฐานะ** ผู้ดูแลระบบที่มี disk setup ต่างจาก `sdc/sdd/sde`<br>**ฉันต้องการ** ให้ VaultWatch ค้นหา disk device อัตโนมัติ<br>**เพื่อให้** รันได้ทันทีโดยไม่ต้อง hardcode ชื่อ device | 1. Auto-detect `sd*` จาก `/sys/block/`<br>2. Config override `devices = [...]` ใน `[system]`<br>3. Filter: ตัด loop, ram, dm-*, md* ออก<br>4. Empty fallback — แสดง warning แทน crash<br>5. แสดง device list ที่ใช้งานใน UI | **S** | ✅ Done |
| [US-MON-19](./user-stories/US-MON-19.md) | **ในฐานะ** ผู้ใช้ที่ไม่คุ้นเคยกับ keyboard shortcuts<br>**ฉันต้องการ** แถบแสดง keyboard shortcuts ที่ด้านล่างสุดของหน้าจอ<br>**เพื่อให้** รู้ว่ากดปุ่มไหนได้บ้างโดยไม่ต้องจำหรืออ่าน README | 1. Key bar ด้านล่างสุดตลอดเวลา<br>2. Context-aware ตาม view/panel<br>3. nano-style: key invert bg, action gray<br>4. ไม่ล้น terminal แคบ<br>5. ลบ shortcuts ซ้ำออกจาก header | **S** | ✅ Done |

---

## 🟠 Graph Label Centering (Sprint 09)

| ID | User Story | Acceptance Criteria | Estimate | Status |
|:---|:---|:---|:---|:---|
| [US-MON-27](./user-stories/US-MON-27.md) | **ในฐานะ** ผู้ดูแลระบบที่อ่านค่าอุณหภูมิจาก Temperature graph<br>**ฉันต้องการ** ให้มีตัวแปร offset ปรับตำแหน่งตัวเลขแกน Y ให้ตรงเส้นแบ่ง zone (ปัจจุบันลอยใต้เส้นครึ่งบรรทัด)<br>**เพื่อให้** จูนตำแหน่งตัวเลขได้จากตัวแปรจุดเดียว | 1. มี `Y_LABEL_OFFSET` named constant ในกลุ่ม theme block<br>2. ปรับค่าที่เดียวมีผลตัวเลขทุก graph<br>3. Default `-0.5` → label center บนเส้นแบ่ง<br>4. เส้นแบ่ง zone ไม่ขยับ (ไม่ regression US-MON-24/25)<br>5. Edge labels (`90`/`0`) clamp ที่ขอบ<br>6. เตรียม config override (`[graph] label_offset`)<br>7. build/clippy/test สะอาด | **S** | ✅ Done |

---

## 🟠 Graph Layout & Color Tuning (Sprint 08)

| ID | User Story | Acceptance Criteria | Estimate | Status |
|:---|:---|:---|:---|:---|
| [US-MON-24](./user-stories/US-MON-24.md) | **ในฐานะ** ผู้ดูแลระบบที่อ่านค่าอุณหภูมิจาก Temperature graph<br>**ฉันต้องการ** ให้ zone background และตัวเลขแกน Y อยู่ตรงตำแหน่งตามสัดส่วนอุณหภูมิจริง<br>**เพื่อให้** เส้นกราฟ, เส้นแบ่ง zone และตัวเลขกำกับ ตรงกันทุกจุด | 1. ใช้สูตร `value/max` คำนวณทุกตำแหน่ง — ไม่จับวาง<br>2. Zone boundary ตรงสัดส่วน (`60/90 = 66.67%`)<br>3. Zone span เท่ากัน → สูงเท่ากัน<br>4. Label ตรงแถวเดียวกับ boundary<br>5. ใช้กับ Read/Write/RAID ด้วย<br>6. ไม่มี hardcoded offset | **S** | ✅ Done |
| [US-MON-25](./user-stories/US-MON-25.md) | **ในฐานะ** ผู้ดูแลระบบที่จ้อง Graph view นานๆ<br>**ฉันต้องการ** ให้เส้นกราฟทุกหน้าใช้สีสว่างขึ้น และพื้นหลัง zone ของ Temperature graph มืดลง 10%<br>**เพื่อให้** เส้นกราฟเด่นและอ่านง่ายขึ้น (contrast สูงขึ้น) | 1. เส้นสว่างขึ้นทุก graph (bright RGB palette)<br>2. Zone bg มืดลง 10% (× 0.9)<br>3. IO/RAID bg `#0A0D14` คงเดิม<br>4. Contrast เพิ่มจริง<br>5. Legend สียังตรงกับเส้น | **S** | ✅ Done |
| [US-MON-26](./user-stories/US-MON-26.md) **Part A** | **ในฐานะ** ผู้พัฒนา/ผู้ดูแลที่อยากปรับ theme<br>**ฉันต้องการ** ให้ค่าสีเส้น/zone/Y-max รวมเป็นตัวแปร block เดียวที่ชื่อชัดเจน<br>**เพื่อให้** เปลี่ยน theme ได้ง่ายโดยแก้จุดเดียว | 1. theme constants อยู่ block เดียวที่หัวไฟล์ + doc comment<br>2. ไม่มี magic number สี/Y-max กระจาย<br>3. แก้สี 1 ค่า มีผลทั้ง graph + legend<br>4. ไม่มี regression | **S** | ✅ Done |

---

## 🟠 Canvas Graph Redesign (Sprint 07)

| ID | User Story | Acceptance Criteria | Estimate | Status |
|:---|:---|:---|:---|:---|
| [US-MON-23](./user-stories/US-MON-23.md) | **ในฐานะ** ผู้ดูแลระบบที่ดู Graph view เป็นประจำ<br>**ฉันต้องการ** ให้ graph ทุกช่องมี background สีตาม theme — Temperature ใช้สีโซนตามระดับความร้อน, Read/Write/RAID ใช้ dark background เดียวกัน<br>**เพื่อให้** อ่านค่าได้ง่ายขึ้นและ Graph view มี visual style ที่สอดคล้องกันทั้งหมด | 1. Temperature: 5 zone colors (0°/30°/40°/50°/60°/90°)<br>2. Read/Write/RAID: dark bg `#0A0D14`<br>3. Braille lines ทับ zone bg ได้ชัด<br>4. Threshold lines 45°/55° ยังแสดงอยู่<br>5. Y-axis labels + legend overlay ครบทุก panel<br>6. Focus/Tab/RAID conditional ไม่ regression | **M** | ✅ Done |

---

## 🟠 Graph View Improvements (Sprint 06)

| ID | User Story | Acceptance Criteria | Estimate | Status |
|:---|:---|:---|:---|:---|
| [US-MON-20](./user-stories/US-MON-20.md) | **ในฐานะ** ผู้ดูแลระบบที่ดู Graph view<br>**ฉันต้องการ** legend บอกว่าเส้น temperature แต่ละสีคือ disk ไหน (แบบเดียวกับ Throughput)<br>**เพื่อให้** รู้ว่า disk ไหนร้อนเท่าไหร่โดยไม่ต้องเดาจากสี | 1. Legend device ที่มุมขวาบนของ Temp graph<br>2. แสดงครบทุก device (≥ 5 disks)<br>3. เส้น threshold 45°/55° ไม่อยู่ใน legend<br>4. ไม่บังเส้น graph ที่จอ 110×30 | **S** | ✅ Done |
| [US-MON-21](./user-stories/US-MON-21.md) | **ในฐานะ** ผู้ดูแลระบบที่ดู throughput หลาย disk<br>**ฉันต้องการ** แยก Throughput เป็นช่อง Read และ Write โดยแยกสีต่อ device<br>**เพื่อให้** รู้ว่าเส้น Write เป็นของ disk ไหน (ตอนนี้ Write ทุกตัวสีเทาเหมือนกัน) | 1. คอลัมน์ขวาแยกเป็น Read panel + Write panel<br>2. สีต่อ device ตรงกันทั้งสองช่อง<br>3. Legend ต่อช่อง<br>4. Tab/mouse focus ครอบ panel ใหม่<br>5. Y-axis สองช่อง scale เดียวกัน | **S** | ✅ Done |
| [US-MON-22](./user-stories/US-MON-22.md) | **ในฐานะ** ผู้ดูแลระบบที่มี mdadm array หลายชุด<br>**ฉันต้องการ** ช่อง RAID Rebuild แสดงเฉพาะตอนมี rebuild และแยกเส้นสีต่อ array<br>**เพื่อให้** ไม่เสียพื้นที่จอ และเห็นความเร็ว rebuild แต่ละ array แยกกัน | 1. Panel แสดงเฉพาะเมื่อมี rebuild (Temp ขยายเต็มเมื่อไม่มี)<br>2. Parse ทุก `mdN` ใน `/proc/mdstat`<br>3. เส้นแยกสีต่อ array + legend ชื่อ array<br>4. History แยก key ต่อ array<br>5. Table view RAID panel ไม่พัง<br>6. Hide delay กัน layout กระพริบ | **M** | ✅ Done |

---

## 🔵 Platform Support (Sprint 04)

| ID | User Story | Acceptance Criteria | Estimate | Status |
|:---|:---|:---|:---|:---|
| [US-MON-14](./user-stories/US-MON-14.md) | **ในฐานะ** ผู้ใช้ Alpine/root/doas<br>**ฉันต้องการ** ให้ smartctl ทำงานโดยไม่ต้องใช้ `sudo` แบบ hardcode<br>**เพื่อให้** VaultWatch รันได้บน setup ที่ไม่มี `sudo` | 1. Auto-detect root → ไม่ใช้ prefix<br>2. Config `smartctl_prefix` สำหรับ doas/custom<br>3. Default behavior เดิมบน non-root | **M** | ✅ Done |
| [US-MON-15](./user-stories/US-MON-15.md) | **ในฐานะ** ผู้ใช้ใหม่<br>**ฉันต้องการ** ให้โปรแกรมแจ้งเตือนเมื่อ external tool ขาด<br>**เพื่อให้** รู้ว่าต้อง install อะไรทันที แทนที่จะเห็น `"--"` ทุกช่อง | 1. ตรวจสอบ smartctl + iostat ตอน startup<br>2. Error screen พร้อม install command ตาม distro<br>3. Degraded mode เมื่อ tool ขาดบางส่วน | **S** | ✅ Done |
| [US-MON-16](./user-stories/US-MON-16.md) | **ในฐานะ** ผู้ใช้ Alpine/Docker<br>**ฉันต้องการ** static binary ที่รันได้โดยไม่ต้องพึ่ง glibc<br>**เพื่อให้** ใช้งานบน Alpine หรือ minimal container ได้ | 1. `make build-static` → musl binary<br>2. รันบน Alpine 3.19+ ได้จริง<br>3. Systemd + OpenRC service files | **S** | ✅ Done |
| [US-MON-17](./user-stories/US-MON-17.md) | **ในฐานะ** ผู้ใช้ทุก distro<br>**ฉันต้องการ** คู่มือติดตั้ง per-distro ที่ครบถ้วน<br>**เพื่อให้** ติดตั้งได้ภายใน 5 นาทีโดยไม่ต้องค้นหาเพิ่ม | 1. README.md ครบ 5 distro<br>2. Annotated config.toml example<br>3. Privilege + systemd setup guide | **M** | ✅ Done |

---

## 🔐 Sprint 10 — Universal Storage & Security Hardening

| ID | User Story | Skill / Scope | Estimate | Status |
|:---|:---|:---|:---|:---|
| [US-MON-28](./user-stories/US-MON-28.md) | Privacy, consent and security baseline | universal/security | **M** | 🚧 In Progress |
| [US-MON-29](./user-stories/US-MON-29.md) | Universal storage inventory graph | universal discovery | **L** | 🚧 In Progress |
| [US-MON-30](./user-stories/US-MON-30.md) | Native block throughput backend | diskstats/sysfs | **M** | 🚧 In Progress |
| [US-MON-31](./user-stories/US-MON-31.md) | Native Linux MD RAID backend | md sysfs | **L** | 🚧 In Progress |
| [US-MON-32](./user-stories/US-MON-32.md) | Storage-first TUI and scoped metrics | UI/config | **L** | 📋 Planned |
| [US-MON-33](./user-stories/US-MON-33.md) | Native SAS/SCSI health | SG_IO/SCSI | **XL** | 📋 Planned |
| [US-MON-34](./user-stories/US-MON-34.md) | Native SATA/ATA health | SAT/ATA SMART | **XL** | 📋 Planned |
| [US-MON-35](./user-stories/US-MON-35.md) | Native NVMe health | NVMe ioctl | **XL** | 📋 Planned |
| [US-MON-36](./user-stories/US-MON-36.md) | USB/removable/SD/eMMC | USB/MMC | **XL** | 📋 Planned |
| [US-MON-37](./user-stories/US-MON-37.md) | Privileged read-only command broker | security boundary | **XL** | 📋 Planned |
| [US-MON-38](./user-stories/US-MON-38.md) | Carry-over and hardware qualification | release gate | **L** | 📋 Planned |

รายละเอียดและลำดับทำงานอยู่ใน [Sprint 10](./sprint-backlogs/sprint-10.md) งานที่ต้องใช้ raw protocol access ห้ามเริ่มก่อน security gate ของ US-MON-28/37

---

## 🟢 Nice to Have (Long-term Vision)

| Feature | Description | Status |
|:---|:---|:---|
| [US-MON-26](./user-stories/US-MON-26.md) Part B — Config-Driven Theme | ย้ายเข้า US-MON-32/38 เพื่อทำพร้อม validated config และ UI redesign | ↪ Sprint 10 |
| Prometheus Exporter | Export metrics ไปยัง Prometheus/Grafana | 🔵 Planned |
| JSON API Export | HTTP endpoint สำหรับ external tooling | 🔵 Planned |
| Cockpit Integration | Plugin สำหรับ RHEL/Ubuntu Cockpit web console | 🔵 Planned |
| Audible Alerts | เสียงเตือนเมื่อ SMART critical | 🔵 Planned |
| Web Dashboard | Lightweight web UI แทน TUI | 🔵 Planned |

---

## 🔗 Related Documents

- Architecture: [../software/00-architecture.md](../software/00-architecture.md)
- System Design: [../software/01-system-design.md](../software/01-system-design.md)
- Sprint Planning: [02-sprint-planning.md](./02-sprint-planning.md)
