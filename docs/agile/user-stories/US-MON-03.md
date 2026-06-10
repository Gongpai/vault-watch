# User Story: US-MON-03 — Disk Throughput Collector

**Status:** ✅ Done
**Sprint:** [Sprint 01](../sprint-backlogs/sprint-01.md)
**Epic:** [Must Have — MVP Scope](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่ต้องการดู I/O ของแต่ละ disk ระหว่าง RAID rebuild
**ฉันต้องการ** ให้โปรแกรมรัน `iostat` และ parse Read/Write throughput ต่อ disk
**เพื่อให้** เห็นได้ชัดว่า disk ไหนกำลัง read/write และในอัตราเท่าไร

---

## ✅ Acceptance Criteria

1. [ ] รัน `iostat -d -k -y 1 1 <device list>` ด้วย `tokio::process::Command` (ต้องมี `-y 1 1` เสมอ เพื่อให้ได้ real-time throughput ไม่ใช่ค่าเฉลี่ย since-boot)
2. [ ] Parse `kB_read/s` ต่อ device และแปลงเป็น MB/s (หาร 1024)
3. [ ] Parse `kB_wrtn/s` ต่อ device และแปลงเป็น MB/s
4. [ ] Handle กรณี `iostat` ไม่ติดตั้ง (exit code != 0) → คืน `Vec::new()` และ log warning
5. [ ] Skip บรรทัด header และบรรทัดว่าง ไม่ panic
6. [ ] ค่าที่ได้สะท้อน throughput **ณ ช่วง 1 วินาทีที่ผ่านมา** ไม่ใช่ค่าเฉลี่ยตั้งแต่ boot

---

## 🛠 Technical Tasks

- [x] สร้าง `src/collectors/iostat.rs`
- [x] สร้าง `struct IoStats` ตาม spec ใน [System Design](../../software/01-system-design.md)
- [x] Implement `async fn collect(devices: &[String]) -> Vec<IoStats>`
- [x] ใช้ command args: `["iostat", "-d", "-k", "-y", "1", "1", ...devices]`
- [x] Parse output ด้วย line-by-line splitting (ไม่ต้องใช้ regex)
- [x] เขียน unit tests ด้วย mock iostat output (ทั้งกรณี `-y` ทำงาน และ fallback)

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- System Design: [../../software/01-system-design.md](../../software/01-system-design.md)
- Sprint: [../sprint-backlogs/sprint-01.md](../sprint-backlogs/sprint-01.md)
