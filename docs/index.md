# HDD Monitor — Project Index

**Project:** HDD Monitor (Rust TUI)
**Status:** Sprint 05 Complete — Device Discovery & Key Hint Bar (Sprints 01–05 ✅ Done)
**Current Architecture:** Async TUI application (`tokio` runtime + `ratatui` renderer)
**Tech Stack:** Rust, ratatui, crossterm, tokio, serde, reqwest, toml, regex
**Last Updated:** 2026-06-11 | **Target Platform:** Ubuntu/Debian · Fedora · Arch · openSUSE · Alpine

---

## Current and Target Status

| Area | Status | Source |
| :--- | :--- | :--- |
| RAID Status Parser (`/proc/mdstat`) | ✅ Done | [US-MON-01](./agile/user-stories/US-MON-01.md), [System Design](./software/01-system-design.md) |
| SMART Data Collector (`smartctl`) | ✅ Done | [US-MON-02](./agile/user-stories/US-MON-02.md), [System Design](./software/01-system-design.md) |
| Disk Throughput Collector (`iostat`) | ✅ Done | [US-MON-03](./agile/user-stories/US-MON-03.md), [System Design](./software/01-system-design.md) |
| TUI Application Foundation | ✅ Done | [US-MON-04](./agile/user-stories/US-MON-04.md), [Architecture](./software/00-architecture.md) |
| RAID Status Panel UI | ✅ Done | [US-MON-05](./agile/user-stories/US-MON-05.md) |
| Disk Summary Table Panel UI | ✅ Done | [US-MON-06](./agile/user-stories/US-MON-06.md) |
| SMART Details Panel UI | ✅ Done | [US-MON-07](./agile/user-stories/US-MON-07.md) |
| Auto-Refresh Async Loop | ✅ Done | [US-MON-08](./agile/user-stories/US-MON-08.md) |
| History Buffer & Graph UI | ✅ Done | [US-MON-12](./agile/user-stories/US-MON-12.md) |
| Panel Focus & Scroll | ✅ Done | [US-MON-13](./agile/user-stories/US-MON-13.md) |
| Temperature Color Coding | ✅ Done | [US-MON-09](./agile/user-stories/US-MON-09.md) |
| SMART Threshold Warnings | ✅ Done | [US-MON-10](./agile/user-stories/US-MON-10.md) |
| Discord Webhook Notifications | ✅ Done | [US-MON-11](./agile/user-stories/US-MON-11.md) |
| Configurable smartctl Privilege | ✅ Done | [US-MON-14](./agile/user-stories/US-MON-14.md) |
| Startup Dependency Check | ✅ Done | [US-MON-15](./agile/user-stories/US-MON-15.md) |
| Static Binary (Alpine/musl) | ✅ Done | [US-MON-16](./agile/user-stories/US-MON-16.md) |
| Cross-Distro Installation Guide | ✅ Done | [US-MON-17](./agile/user-stories/US-MON-17.md) |
| Auto-detect Disk Devices | ✅ Done | [US-MON-18](./agile/user-stories/US-MON-18.md) |
| Key Hint Bar (nano-style) | ✅ Done | [US-MON-19](./agile/user-stories/US-MON-19.md) |

---

## Software Design

- [00-architecture.md](./software/00-architecture.md) - สถาปัตยกรรมระบบ module breakdown และ async data flow
- [01-system-design.md](./software/01-system-design.md) - รายละเอียด data structures, parser specs และ UI layout

---

## Agile Management

- [01-product-backlog.md](./agile/01-product-backlog.md) - Product backlog และสถานะ user stories ทั้งหมด
- [02-sprint-planning.md](./agile/02-sprint-planning.md) - Sprint roadmap และแผนการดำเนินงาน
- [sprint-01.md](./agile/sprint-backlogs/sprint-01.md) - Sprint 01 details (Core Data Collectors + TUI Foundation)
- [sprint-02.md](./agile/sprint-backlogs/sprint-02.md) - Sprint 02 details (Dashboard Panels + Auto-Refresh)
- [sprint-03.md](./agile/sprint-backlogs/sprint-03.md) - Sprint 03 details (Alerts & Notifications)
- [sprint-04.md](./agile/sprint-backlogs/sprint-04.md) - Sprint 04 details (Cross-Distribution Support)
- [sprint-05.md](./agile/sprint-backlogs/sprint-05.md) - Sprint 05 details (Device Discovery)

---

## Resources

- [README.md](../README.md) - Quick start (Ubuntu/Debian) และ overview
- [MANUAL.md](../MANUAL.md) - Per-distro installation manual + privilege setup + service config
- [brief.md](../brief.md) - Project brief และ background ของระบบ
- [changelog.md](./changelog.md) - Documentation and project change history
