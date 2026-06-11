# User Story: US-MON-17 — Cross-Distribution Installation Guide

**Status:** ✅ Done
**Sprint:** [Sprint 04](../sprint-backlogs/sprint-04.md)
**Epic:** [Platform Support](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่ใช้ distro ต่างๆ
**ฉันต้องการ** คู่มือติดตั้งที่บอกทุกอย่างตั้งแต่ต้น
**เพื่อให้** ติดตั้งและตั้งค่า VaultWatch ได้ภายใน 5 นาทีโดยไม่ต้องค้นหาข้อมูลเพิ่มเติม

---

## ✅ Acceptance Criteria

1. [x] `README.md` ที่ root ของ project มี section สำหรับ Ubuntu/Debian, Fedora/RHEL, openSUSE, Arch Linux, Alpine Linux
2. [x] แต่ละ section มี: install prerequisites, build/install binary, config file setup, run command
3. [x] มี annotated `config.toml` example ครบทุก option พร้อม comment อธิบาย
4. [x] มี section อธิบาย privilege setup: sudo, doas, setcap, root
5. [x] มี section อธิบาย systemd service setup (auto-start on boot)
6. [ ] มี Troubleshooting section: อาการทั่วไปและวิธีแก้

> **หมายเหตุ (2026-06-11):** per-distro guide ถูก implement ใน [MANUAL.md](../../../MANUAL.md) (README.md เป็น quick start Ubuntu/Debian + link ไป MANUAL) — ข้อ 6 Troubleshooting section ยังไม่ได้เขียน

---

## 🛠 Technical Tasks

- [x] สร้าง `README.md` ที่ root พร้อม sections:
  - **Overview** — screenshot + feature list
  - **Prerequisites** — per-distro install table
  - **Build** — `make build` / `make build-static`
  - **Configuration** — annotated `config.toml` example
  - **Privilege Setup** — sudo / doas / setcap guide
  - **Run** — command line + systemd service
  - **Troubleshooting** — common issues per distro
- [x] สร้าง `contrib/config.example.toml` — annotated config ครบทุก option
- [x] อัปเดต `docs/index.md` ให้ link ไป README.md

---

## 📋 Per-Distro Prerequisites Table (สำหรับ README)

| Distro | smartmontools | sysstat | sudo |
|:---|:---|:---|:---|
| Ubuntu/Debian | `apt install smartmontools` | `apt install sysstat` | pre-installed |
| Fedora/RHEL | `dnf install smartmontools` | `dnf install sysstat` | pre-installed |
| openSUSE | `zypper install smartmontools` | `zypper install sysstat` | pre-installed |
| Arch Linux | `pacman -S smartmontools` | `pacman -S sysstat` | `pacman -S sudo` |
| Alpine Linux | `apk add smartmontools` | `apk add sysstat` | `apk add sudo` หรือใช้ `doas` |

---

## 📝 Annotated config.toml Example

```toml
# ~/.config/hdd-monitor/config.toml

[system]
# Prefix used before smartctl command.
# Auto-detect: omit this field — root→no prefix, non-root→"sudo"
# Alpine/doas: smartctl_prefix = "doas"
# setcap / run as root: smartctl_prefix = ""
# smartctl_prefix = "sudo"

# Custom path to smartctl binary (default: uses PATH)
# smartctl_path = "/usr/sbin/smartctl"

# Custom path to iostat binary (default: uses PATH)
# iostat_path = "/usr/bin/iostat"

[discord]
# Discord webhook URL for out-of-band alerts
# Get from: Server Settings → Integrations → Webhooks
# webhook_url = "https://discord.com/api/webhooks/..."
```

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- Sprint: [../sprint-backlogs/sprint-04.md](../sprint-backlogs/sprint-04.md)
- Related: [US-MON-14](./US-MON-14.md) (config options ที่ต้อง document)
- Related: [US-MON-16](./US-MON-16.md) (build instructions)
