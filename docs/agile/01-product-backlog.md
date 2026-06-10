# HDD Monitor — Product Backlog

**Last Updated:** 2026-06-11 | **Version:** 1.3

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
| [US-MON-09](./user-stories/US-MON-09.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** ให้อุณหภูมิ disk มีสีบ่งบอกระดับความร้อน<br>**เพื่อให้** เห็นสถานะอันตรายได้ทันทีโดยไม่ต้องอ่านตัวเลข | 1. < 45°C = สีเขียว<br>2. 45–55°C = สีเหลือง<br>3. > 55°C = สีแดง + ข้อความ WARN | **S** | 🚧 Sprint 03 |
| [US-MON-10](./user-stories/US-MON-10.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** การแจ้งเตือนเมื่อ SMART threshold ถูกละเมิด<br>**เพื่อให้** รู้ทันทีเมื่อ disk มีปัญหาที่อาจนำไปสู่ความเสียหาย | 1. Warning เมื่อ grown defects > 0<br>2. Alert เมื่อ health != OK<br>3. แสดง notification ชัดเจนบน UI | **M** | 🚧 Sprint 03 |
| [US-MON-11](./user-stories/US-MON-11.md) | **ในฐานะ** ผู้ดูแลระบบ<br>**ฉันต้องการ** ส่ง alert ไป Discord webhook เมื่อเกิดเหตุการณ์สำคัญ<br>**เพื่อให้** รับการแจ้งเตือนแม้ไม่ได้นั่งดูหน้าจออยู่ | 1. Config webhook URL ผ่าน config file<br>2. Alert เมื่อ RAID degraded<br>3. Alert เมื่อ temp > 60°C<br>4. Alert เมื่อ SMART health != OK | **L** | 🚧 Sprint 03 |

---

## 🟢 Nice to Have (Long-term Vision)

| Feature | Description | Status |
|:---|:---|:---|
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
