# User Story: US-MON-09 — Temperature Color Coding

**Status:** 🔵 Planned
**Sprint:** TBD
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

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- US-MON-06 (Disk Table): [US-MON-06.md](./US-MON-06.md)
