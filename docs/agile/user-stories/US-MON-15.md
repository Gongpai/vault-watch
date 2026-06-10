# User Story: US-MON-15 — Startup Dependency Check

**Status:** 🔵 Sprint 04
**Sprint:** [Sprint 04](../sprint-backlogs/sprint-04.md)
**Epic:** [Platform Support](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่เพิ่ง install VaultWatch บนเครื่องใหม่
**ฉันต้องการ** ให้โปรแกรมแจ้งเตือนชัดเจนเมื่อ external tool ที่จำเป็นยังไม่ได้ติดตั้ง
**เพื่อให้** รู้ทันทีว่าต้องติดตั้งอะไรและใช้คำสั่งอะไร แทนที่จะเห็น `"--"` ทุกช่องโดยไม่รู้สาเหตุ

---

## ✅ Acceptance Criteria

1. [ ] ตรวจสอบ `smartctl` และ `iostat` ก่อนเข้า collector loop (ตอน startup)
2. [ ] ถ้าไม่พบ tool ใดก็ตาม → แสดง error panel แทน normal UI พร้อมระบุว่าขาด tool อะไร
3. [ ] Error panel แสดง install command ที่ถูกต้องตาม distro ที่ detect ได้จาก `/etc/os-release`
4. [ ] Detect ได้อย่างน้อย: Ubuntu/Debian, Fedora/RHEL, Arch, openSUSE, Alpine
5. [ ] ถ้า detect distro ไม่ได้ → แสดง generic install hint (`smartmontools`, `sysstat`)
6. [ ] `smartctl` ขาดแต่ `iostat` มี → Degraded mode (SMART columns แสดง `N/A`, throughput ยังทำงาน)
7. [ ] โปรแกรม exit gracefully เมื่อ user กด `q` จาก error panel

---

## 🛠 Technical Tasks

- [ ] เพิ่ม `fn check_dependencies(config: &Config) -> Vec<DependencyError>` ใน `src/collectors/mod.rs`
  - ทดสอบด้วย `Command::new(smartctl_cmd).arg("--version").output()`
  - ทดสอบด้วย `Command::new("iostat").arg("-V").output()`
- [ ] เพิ่ม `fn detect_distro() -> Distro` — parse `/etc/os-release` field `ID`
  ```
  ubuntu/debian → "apt install smartmontools sysstat"
  fedora/rhel/centos → "dnf install smartmontools sysstat"
  arch/manjaro → "pacman -S smartmontools sysstat"
  opensuse → "zypper install smartmontools sysstat"
  alpine → "apk add smartmontools sysstat"
  _ → generic package names
  ```
- [ ] อัปเดต `main.rs` — run dependency check ก่อน loop, ถ้ามี hard error → render error screen แล้วรอ `q`
- [ ] สร้าง `src/widgets/error_screen.rs` — full-screen error panel แสดง tool ที่ขาด + install command
- [ ] Unit test: `detect_distro()` กับ sample `/etc/os-release` content

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- Sprint: [../sprint-backlogs/sprint-04.md](../sprint-backlogs/sprint-04.md)
- Related: [US-MON-14](./US-MON-14.md) (ใช้ config เดียวกันสำหรับ smartctl path)
- Related: [US-MON-17](./US-MON-17.md) (README มี per-distro install guide)
