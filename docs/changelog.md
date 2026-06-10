# Documentation Changelog

ประวัติการปรับปรุงและการเปลี่ยนแปลงเอกสารทั้งหมดในโปรเจค HDD Monitor

---

## [1.4.0] - 2026-06-10

### Fixed (contradictions & ambiguities audit)

- **[docs/software/01-system-design.md](software/01-system-design.md)** v1.3.0 → v1.4.0:
  - `DiskInfo.serial`: เปลี่ยนจาก `String` → `Option<String>` (None เมื่อ smartctl ไม่ตอบสนอง สอดคล้องกับ optional fields อื่น)
  - `DiskInfo.health_ok`: เพิ่ม comment ว่า default `false` เมื่อ error คือ safe default
  - Section 1.3 header: แก้จาก `iostat -d -k` → `iostat -d -k -y 1 1`
  - เพิ่ม `force_refresh` design note: ใช้ `Arc<tokio::sync::Notify>` แทน field ใน AppState พร้อม code snippet `tokio::select!`
- **[docs/software/00-architecture.md](software/00-architecture.md)**:
  - Mermaid diagram: แก้ `iostat -d -k sdc sdd sde` → `iostat -d -k -y 1 1 sdc sdd sde`
  - Module structure: เพิ่ม `graph_view.rs` และ `sparkline_cell.rs` ใน `src/widgets/`
- **[docs/agile/user-stories/US-MON-04.md](agile/user-stories/US-MON-04.md)**:
  - criterion #3: อัปเดต AppState fields ให้ครบทุก field รวม history maps, scroll state, view mode, panel_rects
  - criterion #6: เปลี่ยนจาก `force_refresh = true` → `refresh_notify.notify_one()` บน `Arc<Notify>`
  - Technical tasks: เพิ่ม task สร้าง `Arc<Notify>`
- **[docs/agile/user-stories/US-MON-07.md](agile/user-stories/US-MON-07.md)**:
  - criterion #6: เปลี่ยนจาก "Panel height ยืดตามจำนวน disk" → fixed height + scroll (สอดคล้องกับ US-MON-13)
- **[docs/agile/user-stories/US-MON-08.md](agile/user-stories/US-MON-08.md)**:
  - Technical tasks: เปลี่ยนจาก `watch`/`Mutex` flag → `Arc<tokio::sync::Notify>`
- **[docs/agile/user-stories/US-MON-12.md](agile/user-stories/US-MON-12.md)**:
  - แก้ text corruption 3 จุด: "gle View" → "Table View", "Disk gle" → "Disk Table" (เกิดจากการ replace Tab→g ที่ทำลายคำว่า "Table")
- **[docs/agile/sprint-backlogs/sprint-01.md](agile/sprint-backlogs/sprint-01.md)**:
  - แก้ iostat command จาก `iostat -d -k` → `iostat -d -k -y 1 1`
- **[docs/agile/sprint-backlogs/sprint-02.md](agile/sprint-backlogs/sprint-02.md)**:
  - DoD terminal size: แก้จาก `80×24` → `100×28 (Table) / 110×30 (Graph)` ให้ตรงกับ system-design
- **[docs/index.md](index.md)**:
  - เพิ่ม US-MON-12 (History Buffer & Graph UI) และ US-MON-13 (Panel Focus & Scroll) ในตาราง status

---

## [1.3.0] - 2026-06-10

### Added

- **[docs/agile/user-stories/US-MON-13.md](agile/user-stories/US-MON-13.md)**: เพิ่ม User Story ใหม่สำหรับ Panel Focus & Scroll — ครอบคลุม `Tab`/`Shift+Tab` panel cycling, keyboard scroll (`↑↓`, `jk`, `PgUp`, `PgDn`, `Home`, `End`), mouse wheel scroll, mouse click focus, double border สำหรับ focused panel, `Scrollbar` widget, status bar focus indicator, mouse hit-testing ผ่าน `panel_rects`

### Changed

- **[docs/software/01-system-design.md](software/01-system-design.md)** v1.2.0 → v1.3.0:
  - Section 3 header: แก้ไข "สลับด้วยปุ่ม `Tab`" → `g` (Tab สงวนไว้สำหรับ panel focus)
  - Section 3.1 Table View: ออกแบบใหม่ mockup แสดง 8 disks พร้อม focused panel (double border), scrollbar (`▲ █ ░ ▼`), status bar (`● DiskTable [5/8 — ↑↓:scroll] ○ SmartDetails`), overflow hint (`↓ N more`)
  - Section 3.2 Graph View: อัปเดต title bar แสดง `Tab:panel`, เพิ่มหมายเหตุ `graph_scroll`
  - Section 3.5 เพิ่มใหม่: Keyboard & Mouse Interaction — ตารางทุก shortcut รวม `Tab`, scroll keys, mouse events
  - Section 3.6 เพิ่มใหม่: Scroll State Logic — pseudocode clamp, slice pattern, mouse hit-testing function
- **[docs/agile/01-product-backlog.md](agile/01-product-backlog.md)** v1.1 → v1.2: เพิ่ม US-MON-13 ใน Must Have section; อัปเดต US-MON-12 toggle key จาก `Tab` เป็น `g`
- **[docs/agile/sprint-backlogs/sprint-02.md](agile/sprint-backlogs/sprint-02.md)**: เพิ่ม US-MON-13 ใน committed stories (8 hrs); อัปเดต US-MON-12 toggle key จาก `Tab` เป็น `g`

### Fixed

- **[docs/software/01-system-design.md](software/01-system-design.md)**: แก้ไข `Tab:graph` / `Tab:table` ใน title bar mockups → `g:graph` / `g:table` เพื่อหลีกเลี่ยง key conflict กับ panel focus

---

## [1.2.0] - 2026-06-10

### Added

- **[docs/agile/user-stories/US-MON-12.md](agile/user-stories/US-MON-12.md)**: เพิ่ม User Story ใหม่สำหรับ History Buffer & Graph UI — ครอบคลุม ring buffers ใน AppState, inline Sparkline ในทุกคอลัมน์ตัวเลขของ disk table, full line Chart ใน Graph View และ `Tab` toggle

### Changed

- **[docs/software/01-system-design.md](software/01-system-design.md)** v1.0.0 → v1.2.0:
  - Section 1.4 AppState: เพิ่ม history ring buffers (`temp_history`, `read_history`, `write_history`, `raid_speed_history`) และ `ViewMode` enum พร้อม `view_mode` field
  - Section 3 UI Layout: ออกแบบใหม่ทั้งหมด แบ่งเป็น Table View (inline Sparklines) และ Graph View (full Chart) พร้อม ASCII mockup และ color scheme table
- **[docs/agile/01-product-backlog.md](agile/01-product-backlog.md)** v1.0 → v1.1: เพิ่ม US-MON-12 ใน Must Have section
- **[docs/agile/sprint-backlogs/sprint-02.md](agile/sprint-backlogs/sprint-02.md)**: เพิ่ม US-MON-12 ใน committed stories table

---

## [1.1.0] - 2026-06-10

### Fixed

- **[docs/software/01-system-design.md](software/01-system-design.md)** Section 2.3: แก้ไข iostat command จาก `iostat -d -k` เป็น `iostat -d -k -y 1 1` — command เดิมให้ค่าเฉลี่ย since-boot ไม่ใช่ real-time throughput; `-y 1 1` บังคับให้ได้ค่า throughput ณ 1 วินาทีที่ผ่านมา
- **[docs/agile/user-stories/US-MON-03.md](agile/user-stories/US-MON-03.md)**: อัปเดต acceptance criteria และ technical tasks ให้ระบุ `-y 1 1` flag อย่างชัดเจน และเพิ่ม criterion ที่ระบุว่าค่าต้องสะท้อน real-time ไม่ใช่ since-boot average

---

## [1.0.0] - 2026-06-10

### Added

- **[docs/index.md](index.md)**: สร้าง project index พร้อม status matrix ของ features ทั้งหมดและลิงก์ไปยังเอกสารทุกส่วน
- **[docs/software/00-architecture.md](software/00-architecture.md)**: สร้างเอกสารสถาปัตยกรรมระบบ ประกอบด้วย Mermaid data flow diagram, module breakdown, async data flow sequence diagram, target disk configuration และ design constraints
- **[docs/software/01-system-design.md](software/01-system-design.md)**: สร้างเอกสาร system design ครอบคลุม data structures (RaidStatus, DiskInfo, IoStats, AppState), parser specifications พร้อม regex patterns (/proc/mdstat, smartctl, iostat), UI layout specification, error handling table และ dependencies
- **[docs/agile/01-product-backlog.md](agile/01-product-backlog.md)** v1.0: สร้าง product backlog ครบถ้วนแบ่งเป็น Must Have (US-MON-01 ถึง US-MON-08), Should Have (US-MON-09 ถึง US-MON-11) และ Nice to Have (Prometheus, JSON API, Cockpit, Web Dashboard)
- **[docs/agile/02-sprint-planning.md](agile/02-sprint-planning.md)** v1.0: สร้าง sprint roadmap 2 sprints — Sprint 01 (Core Data Collectors) และ Sprint 02 (Dashboard UI)
- **[docs/agile/sprint-backlogs/sprint-01.md](agile/sprint-backlogs/sprint-01.md)**: สร้าง Sprint 01 backlog พร้อม Gantt chart, committed stories table, Definition of Done และ known risks
- **[docs/agile/sprint-backlogs/sprint-02.md](agile/sprint-backlogs/sprint-02.md)**: สร้าง Sprint 02 backlog พร้อม Gantt chart และ committed stories
- **[docs/agile/user-stories/US-MON-01.md](agile/user-stories/US-MON-01.md)**: User Story สำหรับ RAID Status Parser (`/proc/mdstat`)
- **[docs/agile/user-stories/US-MON-02.md](agile/user-stories/US-MON-02.md)**: User Story สำหรับ SMART Data Collector (`smartctl`)
- **[docs/agile/user-stories/US-MON-03.md](agile/user-stories/US-MON-03.md)**: User Story สำหรับ Disk Throughput Collector (`iostat`)
- **[docs/agile/user-stories/US-MON-04.md](agile/user-stories/US-MON-04.md)**: User Story สำหรับ TUI Application Foundation
- **[docs/agile/user-stories/US-MON-05.md](agile/user-stories/US-MON-05.md)**: User Story สำหรับ RAID Status Panel UI
- **[docs/agile/user-stories/US-MON-06.md](agile/user-stories/US-MON-06.md)**: User Story สำหรับ Disk Summary Table UI
- **[docs/agile/user-stories/US-MON-07.md](agile/user-stories/US-MON-07.md)**: User Story สำหรับ SMART Details Panel UI
- **[docs/agile/user-stories/US-MON-08.md](agile/user-stories/US-MON-08.md)**: User Story สำหรับ Auto-Refresh Async Loop
- **[docs/agile/user-stories/US-MON-09.md](agile/user-stories/US-MON-09.md)**: User Story สำหรับ Temperature Color Coding (Should Have)
- **[docs/agile/user-stories/US-MON-10.md](agile/user-stories/US-MON-10.md)**: User Story สำหรับ SMART Threshold Warnings (Should Have)
- **[docs/agile/user-stories/US-MON-11.md](agile/user-stories/US-MON-11.md)**: User Story สำหรับ Discord Webhook Notifications (Should Have)
