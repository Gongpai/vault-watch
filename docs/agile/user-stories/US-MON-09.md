# User Story: US-MON-09 — Temperature Color Coding

**Status:** 🚧 Sprint 03
**Sprint:** [Sprint 03](../sprint-backlogs/sprint-03.md)
**Epic:** [Should Have — Future Enhancements](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่มองหน้าจอจากระยะไกล
**ฉันต้องการ** ให้ค่าอุณหภูมิ disk มีสีที่สื่อความหมาย
**เพื่อให้** เห็นสถานะอันตรายได้ทันทีโดยไม่ต้องอ่านตัวเลข

---

## ✅ Acceptance Criteria

1. [ ] < 45°C → สีเขียว (Normal)
2. [ ] 45–55°C → สีเหลือง (Warm)
3. [ ] > 55°C → สีแดง + text `WARN` ต่อท้าย
4. [ ] `N/A` (ไม่มีข้อมูล) → สีปกติ ไม่ highlight

---

## 🛠 Technical Tasks

- [ ] อัปเดต `disk_table.rs` — เพิ่ม `" WARN"` suffix ใน temperature value cell เมื่อ > 55°C
- [ ] ตรวจสอบว่า sparkline สี และ value สีใช้ threshold เดียวกัน (Green/Yellow/Red)
- [ ] อัปเดต Graph View temp chart — เพิ่ม Y-axis threshold lines หรือ labels ที่ 45°C และ 55°C

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- US-MON-06 (Disk Table): [US-MON-06.md](./US-MON-06.md)
- Sprint: [../sprint-backlogs/sprint-03.md](../sprint-backlogs/sprint-03.md)
