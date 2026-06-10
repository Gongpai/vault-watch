# Sprint 03: Alerts & Notifications

**Goal:** เพิ่มระบบแจ้งเตือนครบวงจร — Temperature color coding บน UI, Warning banner เมื่อ SMART threshold ถูกละเมิด และ Discord webhook สำหรับ out-of-band notification เมื่อเหตุการณ์สำคัญเกิดขึ้น
**Timeline:** 2026-07-08 → 2026-07-22

## 📅 Internal Timeline

```mermaid
gantt
    title Sprint 03 Tasks
    dateFormat  YYYY-MM-DD
    section UI Alerts
    Temp Color Coding (US-MON-09)          :t1, 2026-07-08, 2d
    SMART Threshold Warnings UI (US-MON-10) :t2, after t1, 4d
    section Notifications
    Discord Webhook (US-MON-11)            :t3, after t2, 5d
    section Integration
    Integration & Manual Testing           :t4, after t3, 3d
```

---

## 📋 Committed Stories & Tasks

| ID | Story / Task | Owner | Estimate (Hrs) | Status |
|:---|:---|:---|:---|:---|
| [US-MON-09](../user-stories/US-MON-09.md) | **Temperature Color Coding**<br>- เพิ่ม `WARN` suffix เมื่อ temp > 55°C ใน disk table<br>- ตรวจสอบ sparkline + value ใช้สีตาม threshold เดียวกัน<br>- อัปเดต Graph View temp chart Y-axis ที่ 45°C / 55°C | kong | 3 | 🔵 Planned |
| [US-MON-10](../user-stories/US-MON-10.md) | **SMART Threshold Warnings**<br>- สร้าง `Alert` struct และ `collect_alerts()` function<br>- เพิ่ม alert banner (1–2 rows) ใต้ header บน UI<br>- เพิ่ม `alerts` field ใน `AppState` อัปเดตทุก collector cycle<br>- Highlight border สีแดงบน panel ที่มี disk มีปัญหา | kong | 8 | 🔵 Planned |
| [US-MON-11](../user-stories/US-MON-11.md) | **Discord Webhook Notifications**<br>- เพิ่ม `reqwest` dependency<br>- สร้าง `src/notifier.rs` — `send_discord_alert()`<br>- สร้าง config struct + TOML parser (`~/.config/hdd-monitor/config.toml`)<br>- Alert cooldown tracker (1 ชั่วโมง per condition)<br>- Graceful fallback เมื่อไม่มี config | kong | 10 | 🔵 Planned |

---

## 🛠 Sprint Specifics

### Definition of Done (DoD)

- Temperature ใน disk table แสดงสีตาม threshold ถูกต้อง (Green/Yellow/Red+WARN)
- Alert banner ปรากฏบน UI เมื่อ disk มี `grown_defects > 0`, `health_ok == false` หรือ `temp > 55°C`
- Discord notification ส่งถึงเมื่อ RAID degraded, temp > 60°C หรือ health fail (ทดสอบด้วย webhook จริง)
- Cooldown ทำงาน — ไม่ส่ง alert ซ้ำภายใน 1 ชั่วโมง
- โปรแกรมทำงานได้ปกติเมื่อไม่มี config file
- `cargo clippy` และ `cargo test` ผ่านสะอาด

### Known Risks

| ความเสี่ยง | แนวทางแก้ไข |
|:---|:---|
| `reqwest` เพิ่ม compile time และ binary size | ใช้ `reqwest` features เฉพาะที่จำเป็น (`json`, `rustls-tls`) |
| Discord rate limit เมื่อ alert บ่อย | cooldown 1 ชั่วโมงต่อ condition จัดการได้ |
| Alert banner ดัน layout ทำให้ min terminal height เพิ่ม | ออกแบบให้ banner ใช้ `Constraint::Max(2)` แสดงเฉพาะเมื่อมี alert จริง |
