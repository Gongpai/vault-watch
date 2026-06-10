# User Story: US-MON-11 — Discord Webhook Notifications

**Status:** ✅ Done (Sprint 03)
**Sprint:** [Sprint 03](../sprint-backlogs/sprint-03.md)
**Epic:** [Should Have — Future Enhancements](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่ไม่ได้นั่งดูหน้าจออยู่ตลอดเวลา
**ฉันต้องการ** ให้โปรแกรมส่ง alert ไปยัง Discord webhook อัตโนมัติเมื่อเกิดเหตุการณ์สำคัญ
**เพื่อให้** รับการแจ้งเตือนบนมือถือทันทีโดยไม่ต้อง SSH เข้าไปตรวจสอบ

---

## ✅ Acceptance Criteria

1. [x] อ่าน webhook URL จาก config file (เช่น `~/.config/hdd-monitor/config.toml`)
2. [x] ส่ง alert เมื่อ RAID state เปลี่ยนเป็น `Degraded`
3. [x] ส่ง alert เมื่อ temperature > 60°C
4. [x] ส่ง alert เมื่อ `health_ok == false`
5. [x] ไม่ส่ง alert ซ้ำภายใน 1 ชั่วโมงสำหรับ condition เดิม (cooldown)
6. [x] โปรแกรมทำงานได้ปกติถ้าไม่มี config file (Discord เป็น optional)

---

## 🛠 Technical Tasks

- [x] เพิ่ม dependency `reqwest = "0.12"` (rustls-tls, no OpenSSL) + `toml = "0.8"`
- [x] สร้าง `src/notifier.rs` — `async fn send_discord_alert(webhook_url: &str, message: &str)`
- [x] สร้าง config struct (`Config`, `DiscordConfig`) และ TOML parser
- [x] สร้าง alert cooldown tracker (`alert_cooldowns: HashMap<String, Instant>`) ใน `AppState`

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- US-MON-10 (Threshold Warnings): [US-MON-10.md](./US-MON-10.md)
- Sprint: [../sprint-backlogs/sprint-03.md](../sprint-backlogs/sprint-03.md)
