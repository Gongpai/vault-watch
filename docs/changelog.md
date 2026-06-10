# Documentation Changelog

ประวัติการปรับปรุงและการเปลี่ยนแปลงเอกสารทั้งหมดในโปรเจค HDD Monitor

---

## [0.4.1] - 2026-06-11

### Added (Sprint 04 Planning)

- **[docs/agile/sprint-backlogs/sprint-04.md](agile/sprint-backlogs/sprint-04.md)**: สร้าง Sprint 04 backlog — Cross-Distribution Support (US-MON-14/15/16/17), timeline 2026-07-22 → 2026-08-05, Gantt chart, DoD, recommended implementation order และ known risks
- **[docs/agile/user-stories/US-MON-14.md](agile/user-stories/US-MON-14.md)**: Configurable smartctl Privilege Escalation — auto-detect root via `/proc/self/status`, shared `config.rs`, override สำหรับ `doas`/`setcap`
- **[docs/agile/user-stories/US-MON-15.md](agile/user-stories/US-MON-15.md)**: Startup Dependency Check — `check_dependencies()`, `detect_distro()` จาก `/etc/os-release`, error screen widget, degraded mode
- **[docs/agile/user-stories/US-MON-16.md](agile/user-stories/US-MON-16.md)**: Static Binary & Alpine/Docker Support — Makefile targets, `.cargo/config.toml` musl, systemd + OpenRC service files
- **[docs/agile/user-stories/US-MON-17.md](agile/user-stories/US-MON-17.md)**: Cross-Distribution Installation Guide — README.md per-distro, annotated config.example.toml, privilege setup + troubleshooting

### Changed

- **[docs/agile/01-product-backlog.md](agile/01-product-backlog.md)** v1.4 → v1.5: เพิ่ม section "🔵 Platform Support" พร้อม US-MON-14/15/16/17
- **[docs/agile/02-sprint-planning.md](agile/02-sprint-planning.md)** v1.1 → v1.2: เพิ่ม Sprint 04 row + section details
- **[docs/index.md](index.md)**: status → Sprint 04 Planned, target platform อัปเดตเป็น 5 distros, เพิ่ม Sprint 04 US entries

---

## [0.4.0] - 2026-06-11

### Implemented (Sprint 03 — Alerts & Notifications)

- **`src/app.rs`** (US-MON-10): เพิ่ม `Alert` enum (`HighTemperature`, `DiskFail`, `GrownDefects`, `RaidDegraded`) พร้อม `.message()` และ `.is_critical()` methods; `pub fn collect_alerts(state: &AppState) -> Vec<Alert>`; เพิ่ม `alerts: Vec<Alert>` และ `alert_cooldowns: HashMap<String, Instant>` ใน `AppState`
- **`src/notifier.rs`** (US-MON-11): Discord webhook sender — อ่าน `~/.config/hdd-monitor/config.toml` สำหรับ `[discord] webhook_url`; `process_alerts(alerts, cooldowns) -> HashMap` — Discord threshold คือ 60°C (ต่างจาก UI 55°C); cooldown 1 ชั่วโมงต่อ condition key; mutex ถูก release ก่อน HTTP call; graceful when no config
- **`src/widgets/disk_table.rs`** (US-MON-09): `COL_TEMP` 18→23 — เพิ่ม bold red `" WARN"` span เมื่อ `temp > 55°C`; เพิ่ม alert-based border — unfocused: Red เมื่อมี `DiskFail`, Yellow เมื่อมี `GrownDefects`/`HighTemperature`
- **`src/widgets/smart_details.rs`** (US-MON-10): เพิ่ม alert-based border เหมือน disk_table.rs
- **`src/widgets/raid_panel.rs`** (US-MON-10): เพิ่ม Red border เมื่อ `RaidDegraded` alert อยู่ใน `state.alerts`; เปลี่ยน `.style()` → `.border_style()` สำหรับ consistency
- **`src/widgets/graph_view.rs`** (US-MON-09): เพิ่ม threshold reference lines ที่ 45°C (Yellow) และ 55°C (Red) ใน TempGraph เป็น separate Dataset; Y-axis labels มีสี Yellow=`45°` / Red=`55°`
- **`src/ui.rs`** (US-MON-10): เพิ่ม `render_alert_banner()` — Red-bordered panel แสดงสูงสุด 2 alerts; height = 0 เมื่อ no alerts, 3 เมื่อ 1 alert, 4 เมื่อ 2+ alerts; render_table_view สร้าง constraints vector แบบ dynamic
- **`src/main.rs`** (US-MON-10/11): เพิ่ม `mod notifier`; collector_loop เรียก `collect_alerts` → อัปเดต `state.alerts` → `notifier::process_alerts` (no lock held) → อัปเดต `state.alert_cooldowns`
- **`Cargo.toml`**: เพิ่ม `reqwest = "0.12"` (rustls-tls, no OpenSSL) + `toml = "0.8"`

### Changed

- **`docs/software/00-architecture.md`** v0.1.0 → v0.1.1: เพิ่ม `notifier.rs` ใน module structure; เพิ่ม Notifier+Alert nodes ใน architecture diagram; อัปเดต sequence diagram ใน async data flow
- **`docs/agile/user-stories/US-MON-09/10/11.md`**: status → ✅ Done, checkboxes ทั้ง AC และ Tech Tasks
- **`docs/agile/sprint-backlogs/sprint-03.md`**: status ทุก story → ✅ Done
- **`docs/agile/01-product-backlog.md`** v1.3 → v1.4: US-MON-09/10/11 → ✅ Done
- **`docs/agile/02-sprint-planning.md`**: Sprint 03 → ✅ Done
- **`docs/index.md`**: status → Sprint 03 Complete, tech stack เพิ่ม reqwest/toml

---

## [0.3.1] - 2026-06-11

### Added (Sprint 03 Planning)

- **[docs/agile/sprint-backlogs/sprint-03.md](agile/sprint-backlogs/sprint-03.md)**: สร้าง Sprint 03 backlog — Alerts & Notifications (US-MON-09/10/11), timeline 2026-07-08 → 2026-07-22, Gantt chart, DoD และ known risks
- **[docs/agile/user-stories/US-MON-09.md](agile/user-stories/US-MON-09.md)**: เพิ่ม Technical Tasks section และ sprint reference
- **[docs/agile/user-stories/US-MON-10.md](agile/user-stories/US-MON-10.md)**: เพิ่ม Technical Tasks section (Alert struct, collect_alerts(), banner, AppState field) และ sprint reference
- **[docs/agile/user-stories/US-MON-11.md](agile/user-stories/US-MON-11.md)**: เพิ่ม sprint reference

### Changed

- **[docs/agile/02-sprint-planning.md](agile/02-sprint-planning.md)** v1.0 → v1.1: Sprint 01/02 → ✅ Done, เพิ่ม Sprint 03 Active
- **[docs/agile/01-product-backlog.md](agile/01-product-backlog.md)** v1.2 → v1.3: US-MON-09/10/11 → 🚧 Sprint 03
- **[docs/index.md](index.md)**: status → Sprint 03 Active, อัปเดต status matrix และลิงก์ sprint-03

---

## [0.3.0] - 2026-06-11

### Implemented (Sprint 02 — Dashboard UI)

- **`src/widgets/sparkline_cell.rs`** (US-MON-12): Unicode block char helper `sparkline(history, width)` — normalises history slice to `▁▂▃▄▅▆▇█` string of given width, right-aligned with space padding. 4 unit tests (empty, all-zero, padding, max-block)
- **`src/widgets/raid_panel.rs`** (US-MON-05): `render(f, area, state)` — left col shows array name/state badge (Green/Yellow/Red/DarkGray)/disk count; right col shows `Gauge` progress bar + ETA label when rebuilding, inline sparkline of `raid_speed_history` on bottom row
- **`src/widgets/disk_table.rs`** (US-MON-06/12/13): Scrollable `Table` with columns Disk(5)+Temp(22)+Read(22)+Write(22)+Health(10) — each numeric column contains Unicode sparkline + value; temperature color-coded (Green/<45, Yellow/45–55, Red/>55); `D:N` defect indicator in Health column; `Scrollbar` widget when content overflows; `↓ N more` overflow hint; `BorderType::Double`+Cyan when focused; stores `panel_rects` each frame for mouse hit-testing
- **`src/widgets/smart_details.rs`** (US-MON-07/13): Scrollable `Paragraph` with header + one row per disk showing serial(20)/hours(7)/NME(6 — Yellow when >1000)/Defects(7 — Red+⚠ when >0); `Scrollbar` widget; double border when focused; stores `panel_rects`
- **`src/widgets/graph_view.rs`** (US-MON-12): Graph View with 3 `Chart` widgets — left column 65% TempGraph + 35% RaidGraph, right column ThroughputGraph; Braille marker line graphs; per-disk color array `[Cyan,Yellow,Green,Magenta,Blue,Red]`; X-axis `-(n-1)*2s` to `0`
- **`src/widgets/mod.rs`**: เปิด 5 modules (disk_table, graph_view, raid_panel, smart_details, sparkline_cell)
- **`src/ui.rs`** (US-MON-05/06/07/08): Replaced placeholder `draw()` with full orchestration — Table View layout (header 1 + RAID 4 + DiskTable fill + status_bar 1 + SmartDetails 7); Graph View layout (header 1 + graphs fill); terminal size guard shows resize message when below 100×28 (Table) or 110×30 (Graph); `render_status_bar` shows `●/○ DiskTable [N/total — ↑↓:scroll]` / `○/● SmartDetails`
- **`src/main.rs`** (US-MON-08/13): Added `EnableMouseCapture`/`DisableMouseCapture`; `draw(f, &mut state_guard)`; `PgUp`/`PgDn` (±5 rows), `Home`/`End` key handlers; `handle_mouse` for `ScrollUp`/`ScrollDown` (±3 rows on panel under cursor) and `Left Click` (focus panel under cursor); `panel_at()` hit-test helper; `chrono::Local::now().format("%H:%M:%S")` written to `last_updated_str` in `collector_loop`
- **`src/app.rs`**: เพิ่ม `last_updated_str: String` field (initialized `"--:--:--"`)
- **`Cargo.toml`**: เพิ่ม `chrono = "0.4"`

### Fixed (clippy findings)

- **`src/collectors/iostat.rs`**: `.filter(..).last()` on `DoubleEndedIterator` → `.rfind(..)` (clippy::filter_next)
- **`src/main.rs`**: collapsible match for key event guard; collapsible if for `temp_history` push (clippy::collapsible_match / collapsible_if)

---

## [0.2.0] - 2026-06-10

### Implemented (Sprint 01 — Core Data Collectors)

- **`src/collectors/raid.rs`** (US-MON-01): อ่าน `/proc/mdstat` ด้วย `tokio::fs::read_to_string`, parse ด้วย `LazyLock<Regex>` — ตรวจจับ array name, state (Active/Rebuilding/Degraded/Unknown), disk count `[N/M]`, rebuild %, speed K/sec÷1024→MB/s, ETA `ceil(min)`. 5 unit tests (active, rebuilding, degraded, no-array, inactive)
- **`src/collectors/smart.rs`** (US-MON-02): รัน `sudo smartctl -a -d scsi /dev/sdX` ด้วย `tokio::process::Command`, parse 8 fields ด้วย `LazyLock<Regex>`. ใช้ `futures::future::join_all` สำหรับ concurrent collection — รักษา device order. Error fallback คืน `DiskInfo` ที่มีค่า `None`/`false` ทุก optional field. 3 unit tests
- **`src/collectors/iostat.rs`** (US-MON-03): รัน `iostat -d -k -y 1 1 <devices>`, parse แบบ line-splitting (ไม่ใช้ regex). Block-detection logic ใช้ block สุดท้ายที่มี `Device` header — รองรับ kernel เก่าที่ ignore `-y`. 3 unit tests
- **`src/main.rs`** — `collector_loop` (US-MON-08 partial): แทนที่ placeholder ด้วย `tokio::join!` รัน 3 collectors พร้อมกัน, push ผลลัพธ์ลง `AppState` history buffers (`temp_history`, `read_history`×10, `write_history`×10, `raid_speed_history`) พร้อม pop_front เมื่อ len > `HISTORY_SIZE`
- **`Cargo.toml`**: เพิ่ม `futures = "0.3"`
- **`src/collectors/mod.rs`**: เปิด `pub mod raid; pub mod smart; pub mod iostat;`

### Fixed (code-review findings)

- **`src/collectors/raid.rs`**: `inactive` array เดิมรายงาน `Active` เพราะ `0 < 0 = false` — แก้โดย capture `(active|inactive)` และ return `RaidState::Unknown` เมื่อ inactive
- **`src/main.rs`**: `raid_speed_history` เดิม skip push เมื่อไม่มี rebuild ทำให้ graph time-axis ผิด — แก้โดย push `0` เสมอเมื่อ `rebuild_speed_mb = None`

---

## [0.1.4] - 2026-06-10

### Fixed (contradictions & ambiguities audit)

- **[docs/software/01-system-design.md](software/01-system-design.md)** v0.1.3 → v0.1.4:
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

## [0.1.3] - 2026-06-10

### Added

- **[docs/agile/user-stories/US-MON-13.md](agile/user-stories/US-MON-13.md)**: เพิ่ม User Story ใหม่สำหรับ Panel Focus & Scroll — ครอบคลุม `Tab`/`Shift+Tab` panel cycling, keyboard scroll (`↑↓`, `jk`, `PgUp`, `PgDn`, `Home`, `End`), mouse wheel scroll, mouse click focus, double border สำหรับ focused panel, `Scrollbar` widget, status bar focus indicator, mouse hit-testing ผ่าน `panel_rects`

### Changed

- **[docs/software/01-system-design.md](software/01-system-design.md)** v0.1.2 → v0.1.3:
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

## [0.1.2] - 2026-06-10

### Added

- **[docs/agile/user-stories/US-MON-12.md](agile/user-stories/US-MON-12.md)**: เพิ่ม User Story ใหม่สำหรับ History Buffer & Graph UI — ครอบคลุม ring buffers ใน AppState, inline Sparkline ในทุกคอลัมน์ตัวเลขของ disk table, full line Chart ใน Graph View และ `Tab` toggle

### Changed

- **[docs/software/01-system-design.md](software/01-system-design.md)** v0.1.0 → v0.1.2:
  - Section 1.4 AppState: เพิ่ม history ring buffers (`temp_history`, `read_history`, `write_history`, `raid_speed_history`) และ `ViewMode` enum พร้อม `view_mode` field
  - Section 3 UI Layout: ออกแบบใหม่ทั้งหมด แบ่งเป็น Table View (inline Sparklines) และ Graph View (full Chart) พร้อม ASCII mockup และ color scheme table
- **[docs/agile/01-product-backlog.md](agile/01-product-backlog.md)** v1.0 → v1.1: เพิ่ม US-MON-12 ใน Must Have section
- **[docs/agile/sprint-backlogs/sprint-02.md](agile/sprint-backlogs/sprint-02.md)**: เพิ่ม US-MON-12 ใน committed stories table

---

## [0.1.1] - 2026-06-10

### Fixed

- **[docs/software/01-system-design.md](software/01-system-design.md)** Section 2.3: แก้ไข iostat command จาก `iostat -d -k` เป็น `iostat -d -k -y 1 1` — command เดิมให้ค่าเฉลี่ย since-boot ไม่ใช่ real-time throughput; `-y 1 1` บังคับให้ได้ค่า throughput ณ 1 วินาทีที่ผ่านมา
- **[docs/agile/user-stories/US-MON-03.md](agile/user-stories/US-MON-03.md)**: อัปเดต acceptance criteria และ technical tasks ให้ระบุ `-y 1 1` flag อย่างชัดเจน และเพิ่ม criterion ที่ระบุว่าค่าต้องสะท้อน real-time ไม่ใช่ since-boot average

---

## [0.1.0] - 2026-06-10

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
