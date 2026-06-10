# User Story: US-MON-14 — Configurable smartctl Privilege Escalation

**Status:** 🔵 Sprint 04
**Sprint:** [Sprint 04](../sprint-backlogs/sprint-04.md)
**Epic:** [Platform Support](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่รัน VaultWatch บน Alpine Linux, Void Linux หรือในฐานะ root user
**ฉันต้องการ** ให้ smartctl ทำงานโดยไม่ต้องพึ่งพา `sudo` แบบ hardcode
**เพื่อให้** VaultWatch รันได้บน setup ที่ไม่มี `sudo` (เช่น `doas`, root, setcap)

---

## ✅ Acceptance Criteria

1. [ ] ถ้า process รันในฐานะ root (`uid == 0`) → ใช้ `smartctl` โดยตรงโดยไม่มี prefix
2. [ ] ถ้าไม่ใช่ root → ใช้ `sudo` เป็น default prefix (behavior เดิม)
3. [ ] Config `[system] smartctl_prefix = "doas"` → ใช้ `doas smartctl` (Alpine/Void)
4. [ ] Config `[system] smartctl_prefix = ""` → ไม่มี prefix (สำหรับ setcap หรือ root)
5. [ ] Config `[system] smartctl_path = "/usr/sbin/smartctl"` → ใช้ path แบบ explicit
6. [ ] ไม่มี `[system]` section ใน config → auto-detect ทำงานได้โดยไม่ error

---

## 🛠 Technical Tasks

- [ ] สร้าง `src/config.rs` — shared `Config` struct รวม `[system]` + `[discord]` section
  ```toml
  [system]
  smartctl_prefix = "doas"   # optional — auto-detect ถ้าไม่ระบุ
  smartctl_path = "smartctl" # optional — default ใช้ PATH
  iostat_path = "iostat"     # optional — default ใช้ PATH
  ```
- [ ] ย้าย config loading จาก `notifier.rs` → `config.rs` (refactor ไม่เปลี่ยน behavior)
- [ ] เพิ่ม `fn detect_smartctl_cmd(config: &Config) -> (String, Vec<String>)`:
  - อ่าน `/proc/self/status` line `Uid:` เพื่อ detect root (ไม่ต้องการ crate ใหม่)
  - คืน `("smartctl", ["-a", "-d", "scsi", path])` เมื่อเป็น root
  - คืน `(prefix, ["smartctl", "-a", "-d", "scsi", path])` เมื่อไม่ใช่ root
- [ ] อัปเดต `smart.rs` — รับ `smartctl_cmd` และ `smartctl_args` จาก config แทน hardcode
- [ ] Pass config ผ่าน `AppState` หรือ parameter ไปยัง `collectors::smart::collect_all()`
- [ ] Unit test: test `detect_smartctl_cmd()` กับ config ทุก case

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- Sprint: [../sprint-backlogs/sprint-04.md](../sprint-backlogs/sprint-04.md)
- Related: [US-MON-15](./US-MON-15.md) (dependency check ใช้ config เดียวกัน)
- Related: [US-MON-11](./US-MON-11.md) (Discord config จะถูก merge เข้า config.rs)
