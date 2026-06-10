# HDD Monitor — Project Index

**Project:** HDD Monitor (Rust TUI)
**Status:** Sprint 01 Complete — Sprint 02 Active
**Current Architecture:** Async TUI application (`tokio` runtime + `ratatui` renderer)
**Tech Stack:** Rust, ratatui, crossterm, tokio, serde, regex
**Last Updated:** 2026-06-10 | **Target Platform:** Ubuntu Server 24.04

---

## Current and Target Status

| Area | Status | Source |
| :--- | :--- | :--- |
| RAID Status Parser (`/proc/mdstat`) | ✅ Done | [US-MON-01](./agile/user-stories/US-MON-01.md), [System Design](./software/01-system-design.md) |
| SMART Data Collector (`smartctl`) | ✅ Done | [US-MON-02](./agile/user-stories/US-MON-02.md), [System Design](./software/01-system-design.md) |
| Disk Throughput Collector (`iostat`) | ✅ Done | [US-MON-03](./agile/user-stories/US-MON-03.md), [System Design](./software/01-system-design.md) |
| TUI Application Foundation | ✅ Done | [US-MON-04](./agile/user-stories/US-MON-04.md), [Architecture](./software/00-architecture.md) |
| RAID Status Panel UI | 🚧 Sprint 02 | [US-MON-05](./agile/user-stories/US-MON-05.md) |
| Disk Summary Table Panel UI | 🚧 Sprint 02 | [US-MON-06](./agile/user-stories/US-MON-06.md) |
| SMART Details Panel UI | 🚧 Sprint 02 | [US-MON-07](./agile/user-stories/US-MON-07.md) |
| Auto-Refresh Async Loop | 🚧 Sprint 02 | [US-MON-08](./agile/user-stories/US-MON-08.md) |
| History Buffer & Graph UI | 🔵 Sprint 02 | [US-MON-12](./agile/user-stories/US-MON-12.md) |
| Panel Focus & Scroll | 🔵 Sprint 02 | [US-MON-13](./agile/user-stories/US-MON-13.md) |
| Temperature Color Coding | 🔵 Planned | [Product Backlog](./agile/01-product-backlog.md) |
| SMART Threshold Warnings | 🔵 Planned | [Product Backlog](./agile/01-product-backlog.md) |
| Discord Webhook Notifications | 🔵 Planned | [Product Backlog](./agile/01-product-backlog.md) |

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

---

## Resources

- [brief.md](../brief.md) - Project brief และ background ของระบบ
- [changelog.md](./changelog.md) - Documentation and project change history
