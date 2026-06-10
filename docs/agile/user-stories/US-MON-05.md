# User Story: US-MON-05 — RAID Status Panel

**Status:** 🔵 Planned
**Sprint:** [Sprint 02](../sprint-backlogs/sprint-02.md)
**Epic:** [Must Have — MVP Scope](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่กำลังติดตาม RAID rebuild
**ฉันต้องการ** panel ที่แสดงสถานะ RAID อย่างชัดเจน
**เพื่อให้** เห็น array name, rebuild progress, speed และ ETA โดยไม่ต้องรัน `cat /proc/mdstat` เอง

---

## ✅ Acceptance Criteria

1. [ ] แสดง array name (เช่น `md0`) และ RAID level
2. [ ] แสดง state: `ACTIVE` (สีเขียว), `REBUILDING` (สีเหลือง), `DEGRADED` (สีแดง)
3. [ ] แสดง progress bar rebuild (filled blocks)
4. [ ] แสดง rebuild % ถัดจาก progress bar
5. [ ] แสดง rebuild speed (MB/s) และ ETA (เช่น `8h 16m`)
6. [ ] แสดง disk count (เช่น `3/3`)
7. [ ] เมื่อ no array detected → แสดง "No RAID array detected"
8. [ ] เมื่อ RAID active ไม่มี rebuild → ไม่แสดง progress bar, แสดง "Healthy" แทน

---

## 🛠 Technical Tasks

- [ ] สร้าง `src/widgets/raid_panel.rs`
- [ ] Implement `fn render_raid_panel(f: &mut Frame, area: Rect, state: &AppState)`
- [ ] ใช้ `ratatui::widgets::{Block, Paragraph, Gauge}` สำหรับ progress bar
- [ ] Color mapping: `RaidState::Active` → Green, `Rebuilding` → Yellow, `Degraded` → Red
- [ ] Format ETA: แปลงนาที → `Xh Ym` string

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- System Design (Layout): [../../software/01-system-design.md](../../software/01-system-design.md)
- Sprint: [../sprint-backlogs/sprint-02.md](../sprint-backlogs/sprint-02.md)
