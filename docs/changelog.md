# Documentation Changelog

ประวัติการปรับปรุงและการเปลี่ยนแปลงเอกสารทั้งหมดในโปรเจค HDD Monitor

---

## Unreleased — 2026-07-11

### Fixed (Sprint 10 real-hardware regression)

- Privacy bar แยกจำนวน whole block, NVMe whole devices, partitions และ virtual nodes; partition ของ P4618 ไม่ถูกนับเป็น NVMe drive อีกต่อไป
- Disk Summary ใช้ graph inventory เติม whole devices ที่ legacy collector ยังไม่รองรับ โดยแสดง health เป็น `N/A` แทนการซ่อน NVMe
- SMART status เปลี่ยนจาก boolean เป็น `Healthy`/`Failed`/`Unavailable`; missing tool/status, ambiguous output และอุณหภูมิ sentinel `0°C` ไม่สร้าง false critical alert
- เพิ่ม synthetic Intel DC P4618 fixture: 2 NVMe whole devices, 6 partitions และ 32 loop devices
- เพิ่ม periodic sysfs topology reconciliation แบบ atomic; failed-empty snapshot รักษา last-known graph เป็น partial และ device incarnation ใหม่แทนรุ่นเดิมผ่าน `diskseq`/`dev_t`
- hardware verification: removable whole device และ partitions เพิ่ม/ลดจาก inventory ภายใน polling cycle โดยไม่ restart/crash; หลักฐานบันทึกแบบ sanitized
- เริ่ม Sprint 10B native counters: defensive `/proc/diskstats` batch parser สำหรับ base/discard/flush layouts และ reset-safe metric calculator ที่ใช้ sector 512 bytes พร้อม unavailable latency เมื่อ idle
- cut over throughput runtime จาก `iostat` เป็น generation-keyed `/proc/diskstats` sampler; IO table/graphs รวม NVMe whole devices และตัด partition/virtual/stacked layers เพื่อไม่ double count
- hardware verification: native NVMe/removable throughput แสดงใน table/graph และ removable add/read/remove ทำงานโดยไม่ restart/crash
- fix responsive TUI regressions: ขยาย device column เป็น 12, compact privacy counts บน terminal <150 columns และแก้ native throughput label จาก MB/s เป็น MiB/s
- hardware verification: common NVMe names, compact storage counts และ MiB/s table/graph labels แสดงถูกต้องที่ terminal width จริง
- เริ่ม native MD sysfs shadow backend: enumerate โดยตรวจ `md/` ไม่ assume ชื่อ, typed array/action/member state, external metadata, progress/speed/ETA, malformed-to-partial และ bounded consistency retry พร้อม fixtures
- operator verification: targeted native MD sysfs fixture suite ผ่านครบโดยไม่มี MD hardware dependency
- เพิ่ม MD operation sampler: delta rebuild speed/ETA, generation reset บน action/total/metadata/topology change และ semantic fixture comparison กับ legacy `/proc/mdstat`
- cut over MD runtime เป็น read-only sysfs primary; `/proc/mdstat` เหลือ test oracle และ partial/unavailable snapshots รักษา last-known arrays พร้อม UI availability label
- operator verification หลัง MD cutover: native MD fixture suite 6/6, full regression 52/52 และ no-array runtime path เริ่มทำงานได้บน openSUSE; ยังไม่อ้างว่า live MD/rebuild ผ่าน
- ปิด US-MON-30: Device Details แสดง native IOPS, utilization, read/write latency, average queue depth และ in-flight I/O พร้อม source/scope `diskstats/whole`; idle latency เป็น `N/A` และ baseline/reset/device absent เป็น unavailable

## [0.9.0] - 2026-06-17

### Implemented (Sprint 09 — Tunable Y-Axis Label Offset)

- **`src/widgets/graph_view.rs`** (US-MON-27): เพิ่ม `Y_LABEL_OFFSET: f64 = -0.5` ในกลุ่ม theme constants (ใต้ `IO_Y_MAX`) + `row_for_label()` (`round(row_pos(v) + Y_LABEL_OFFSET)`); `render_y_labels` เรียก `row_for_label` แทน `row_for_value` → ตัวเลขแกน Y center บนเส้นแบ่งแทน top-align (ครอบทุก graph เพราะใช้ `render_y_labels` ร่วมกัน); ลบ `row_for_value()` ที่กลายเป็น dead code (ผู้เรียกรายเดียวคือ `render_y_labels`; `ZoneBackground` คำนวณ `row_pos().round()` inline อยู่แล้ว → เส้นแบ่ง zone ไม่ขยับ)
- **`contrib/config.example.toml`**: เพิ่ม commented `label_offset = -0.5` ใน `[graph]` (planned US-MON-26 Part B)

### Verified

- `cargo clippy` clean (ไม่มี warning — รวม dead_code หลังลบ `row_for_value`), `cargo test` 16 passed
- ยังต้อง verify บนเครื่องจริง (HPE server): ตัวเลข `30/40/50/60` center บนเส้นแบ่ง zone ด้วยตา + ลองจูน `Y_LABEL_OFFSET` ตามฟอนต์/terminal

### Added (Sprint 09 Planning — Tunable Y-Axis Label Offset)

จาก feedback การใช้งานจริง (screenshot 2026-06-17 10:51): หลัง Sprint 08 layout เขต temperature ตรงสัดส่วนแล้ว แต่ตัวเลขแกน Y (`30/40/50/60`) ยังลอยอยู่ **ใต้** เส้นแบ่ง zone ราวครึ่ง cell — ผู้ใช้ชี้แจงเพิ่มว่าไม่ได้ให้ hardcode แต่ต้องการ **ตัวแปร offset** สำหรับปรับตำแหน่งตัวเลขให้ตรงขึ้น

- **[docs/agile/user-stories/US-MON-27.md](agile/user-stories/US-MON-27.md)**: Tunable Y-Axis Label Offset — เพิ่ม named constant `Y_LABEL_OFFSET` (default `-0.5`) ในกลุ่ม theme block + `row_for_label()` (`round(row_pos + Y_LABEL_OFFSET)`) แยกจาก `row_for_value()` (boundary, ไม่เปลี่ยน); `-0.5` = ครึ่งความสูง text cell แต่เป็น **ตัวแปร** ปรับที่เดียวมีผลทุก label; root cause = label top-aligned ทำให้ glyph center (`row + 0.5`) อยู่ใต้ boundary
- **[docs/agile/sprint-backlogs/sprint-09.md](agile/sprint-backlogs/sprint-09.md)**: สร้าง Sprint 09 backlog — timeline 2026-09-30 → 2026-10-14, implementation plan, target visual (before/after), DoD, known risks
- **[docs/software/01-system-design.md](software/01-system-design.md)**: §3.4 เพิ่มหัวข้อ "Tunable Label Offset" — กฎแยก boundary row vs label row + `Y_LABEL_OFFSET`
- **[contrib/config.example.toml](../contrib/config.example.toml)**: เพิ่ม commented `label_offset` ใน `[graph]` (planned US-MON-26 Part B)

### Changed

- **[docs/agile/01-product-backlog.md](agile/01-product-backlog.md)** v2.2 → v2.3: เพิ่ม section "🟠 Graph Label Centering (Sprint 09)" + US-MON-27
- **[docs/agile/02-sprint-planning.md](agile/02-sprint-planning.md)** v1.8 → v1.9: เพิ่ม Sprint 09 row + section details
- **[docs/index.md](index.md)**: sync สถานะที่ค้าง — status → Sprint 08 Complete + Sprint 09 Planned, เพิ่ม US-MON-23/24/25/26/27 ใน status matrix, เพิ่มลิงก์ sprint-07/08/09

---

## [0.7.0] - 2026-06-11

### Implemented (Sprint 06 — Graph View Improvements)

- **`src/widgets/graph_view.rs`** (US-MON-20): Temp graph legend แสดงได้แล้ว — threshold lines 45°/55° เป็น unnamed datasets (ratatui นับเฉพาะ dataset ที่มี name เข้า legend) + `hidden_legend_constraints` ขยายจาก default ¼ → ½ ของ panel (`LEGEND_CONSTRAINTS` const ใช้ร่วมทุก chart); root cause: 7 ชื่อ dataset (5 disks + 2 thresholds) เกิน ¼ height ทำให้ ratatui ซ่อน legend ทั้งอัน
- **`src/widgets/graph_view.rs`** (US-MON-21): แตก `render_throughput_graph()` → `render_io_graph()` shared renderer — คอลัมน์ขวาแยกเป็น Read (บน) / Write (ล่าง) 50/50, สีต่อ device จาก `DISK_COLORS` ตรงกันทั้งสองช่อง, legend ชื่อ device (ไม่มี R/W suffix), Y-axis 0–200 เท่ากันทั้งคู่
- **`src/widgets/graph_view.rs`** (US-MON-22): RAID graph แสดงเฉพาะเมื่อ `raid_graph_visible()` — ไม่มี rebuild → Temperature เต็มคอลัมน์ซ้าย; เส้นแยกสีต่อ array (sort ชื่อให้สีคงที่) + legend ชื่อ array; focus ที่ค้างบน RaidGraph ตอนซ่อนถูกย้ายไป TempGraph + ลบ panel_rect กัน mouse hit-test ค้าง
- **`src/collectors/raid.rs`** (US-MON-22): `collect()/parse_mdstat()` → `Vec<RaidStatus>` — parse ทุก `mdN` block ไม่ใช่แค่ตัวแรก; เพิ่ม `test_two_arrays` (md0 rebuilding + md1 active), อัปเดต tests เดิมทั้ง 5 ตัว
- **`src/app.rs`** (US-MON-22): `raid: Option<RaidStatus>` → `raids: Vec<RaidStatus>`; `raid_speed_history` → `HashMap<String, VecDeque<u64>>` per array; เพิ่ม `raid_graph_visible()` (rebuilding หรือ history ยังมีค่า non-zero = hide delay กัน layout กระพริบ); `Alert::RaidDegraded` → `{ array: String }` ระบุชื่อ array ใน message; `FocusedPanel::ThroughputGraph` → `ReadGraph` + `WriteGraph`
- **`src/main.rs`**: Tab/BackTab cycle ใน Graph view ครอบ 4 panels — RaidGraph เข้า cycle เฉพาะตอน visible; collector_loop push speed ต่อ array (array ไม่ rebuild push 0 รักษา time axis, array ที่หายจาก mdstat ไหล 0 จน history ว่างแล้วถูก drop)
- **`src/widgets/raid_panel.rs`** (US-MON-22): เลือก array ที่ rebuilding ก่อน → degraded → ตัวแรก; title แสดง `(+N more)` เมื่อมีหลาย array; sparkline ใช้ history ของ array ที่แสดง
- **`src/notifier.rs`**: cooldown key `raid_degraded_{array}` แยกต่อ array

### Fixed (carry-over + clippy)

- **`src/widgets/smart_details.rs`**: แสดง `RdErr:` / `WrErr:` ต่อ disk (Yellow เมื่อ > 0) — ลบ dead_code warning ของ `read_errors`/`write_errors` (carry-over จาก Sprint 05)
- **`src/app.rs`, `src/config.rs`**: เก็บ clippy warnings เก่าทั้งหมด (collapsible_if ×3 → let-chains, redundant_guards → `Some("")`) — `cargo clippy` สะอาด 100%

### Verified

- `cargo test` 16 passed (รวม `test_two_arrays`), `cargo clippy` ไม่มี warning
- Smoke test ใน pty (120×35): เปิด app → toggle Graph view → Tab → quit ออกสะอาด; frame capture ยืนยัน Temperature เต็มคอลัมน์ซ้าย (ไม่มี rebuild) + Read/Write แยกสองช่อง
- ยังต้อง verify บนเครื่องจริง (HPE server): legend device names บนจอที่มี `sd*` disks และ RAID graph ตอน rebuild จริง

---

## [0.6.1] - 2026-06-11

### Added (Sprint 06 Planning — Graph View Improvements)

จาก feedback การใช้งาน Graph view จริง: temp graph ไม่มี legend บอกว่าเส้นไหนคือ disk ไหน, เส้น Write ทุก disk เป็นสีเทาแยกไม่ออก, และช่อง RAID Rebuild ว่างเปล่ากินพื้นที่ตอนไม่มี rebuild

- **[docs/agile/sprint-backlogs/sprint-06.md](agile/sprint-backlogs/sprint-06.md)**: สร้าง Sprint 06 backlog — Graph View Improvements (US-MON-20/21/22), timeline 2026-08-19 → 2026-09-02, target layout, implementation order, DoD, carry-over จาก Sprint 05 known gaps และ known risks
- **[docs/agile/user-stories/US-MON-20.md](agile/user-stories/US-MON-20.md)**: Temperature Graph Per-Device Legend — แก้ ratatui auto-hide legend (`hidden_legend_constraints`) + เอา threshold lines ออกจาก legend
- **[docs/agile/user-stories/US-MON-21.md](agile/user-stories/US-MON-21.md)**: Split Throughput เป็น Read/Write สองช่อง — แยกสีต่อ device ได้ทั้งสองทิศ, `FocusedPanel::ReadGraph`/`WriteGraph`
- **[docs/agile/user-stories/US-MON-22.md](agile/user-stories/US-MON-22.md)**: RAID Rebuild Graph แสดงเฉพาะตอน rebuild + multi-array — parse ทุก `mdN`, history ต่อ array, เส้นแยกสี + legend ชื่อ array, hide delay

### Changed

- **[docs/agile/01-product-backlog.md](agile/01-product-backlog.md)** v1.8 → v1.9: เพิ่ม section "🟠 Graph View Improvements (Sprint 06)" พร้อม US-MON-20/21/22
- **[docs/agile/02-sprint-planning.md](agile/02-sprint-planning.md)** v1.4 → v1.5: เพิ่ม Sprint 06 row + section details
- **[docs/index.md](index.md)**: status → Sprint 06 Planned, เพิ่ม US-MON-20/21/22 ใน status matrix + sprint-06 link

---

## [0.6.0] - 2026-06-11

### Implemented (Sprint 05 — Device Discovery & UX)

- **`src/config.rs`** (US-MON-18): เพิ่ม `detect_disk_devices()` — อ่าน `/sys/block` กรองเฉพาะ `sd*` entries (partition ไม่ปรากฏที่ระดับ `/sys/block` จึงไม่ต้อง filter เพิ่ม) + sort; เพิ่ม `resolve_devices(config)` — `[system] devices = [...]` override มาก่อน auto-detect; เพิ่ม `devices: Option<Vec<String>>` ใน `SystemConfig`
- **`src/main.rs`** (US-MON-18): ลบ device list hardcode — ใช้ `config::resolve_devices(&cfg)` ตอน startup
- **`src/ui.rs`** (US-MON-19): เพิ่ม `render_key_bar(f, area, state)` — nano-style hint bar (key = `fg(Black) bg(Cyan)` invert, action = `fg(DarkGray)`); context-aware: `g` label สลับ Graph/Table ตาม view mode, `Home/End Jump` เฉพาะ Table view; เพิ่ม `Constraint::Length(1)` ท้าย layout ทั้ง Table และ Graph view; header เหลือแค่ title + last update (ไม่มี shortcut ซ้ำ)
- **`contrib/config.example.toml`**: เพิ่ม `devices` option พร้อม comment

### Known Gaps (ยกมา Sprint หน้า)

- US-MON-18 AC4/AC5: ยังไม่มีข้อความ "No disk devices found" บน UI เมื่อ detect ไม่เจอ device (ไม่ crash แต่เงียบ) และยังไม่แสดง active device list ใน header/status bar
- US-MON-17 AC6: MANUAL.md ยังไม่มี Troubleshooting section
- US-MON-16: static binary ยังไม่ได้ทดสอบจริงบน Alpine/Docker
- `config.rs` ยังไม่มี unit tests (`smartctl_base_cmd`, `detect_distro`)
- `DiskInfo.read_errors`/`write_errors` ถูก parse แต่ไม่แสดงบน UI (dead_code warning)

### Changed (doc status sync)

- **[docs/agile/01-product-backlog.md](agile/01-product-backlog.md)** v1.7 → v1.8: US-MON-14/15/16/17/18/19 → ✅ Done
- **[docs/agile/02-sprint-planning.md](agile/02-sprint-planning.md)** v1.3 → v1.4: Sprint 04/05 → ✅ Done
- **[docs/agile/sprint-backlogs/sprint-04.md](agile/sprint-backlogs/sprint-04.md)**: story statuses → ✅ Done
- **[docs/agile/sprint-backlogs/sprint-05.md](agile/sprint-backlogs/sprint-05.md)**: status → ✅ Done, DoD checkboxes ติ๊กตามที่ verify ได้จริง (เหลือ runtime test ต่าง machine + empty-device warning)
- **[docs/agile/user-stories/US-MON-14…19.md](agile/user-stories/)**: status → ✅ Done, AC/Tech Task checkboxes ติ๊กตาม code จริง พร้อมหมายเหตุข้อที่ยังไม่ครบ
- **[docs/index.md](index.md)**: status → Sprint 05 Complete, status matrix → ✅ Done ทั้งหมด, เพิ่มลิงก์ README.md/MANUAL.md ใน Resources
- **MANUAL.md**: เพิ่ม Reinstall/Update + Uninstall sections; แก้ install path สำหรับ openSUSE (`sudo` secure_path → แนะนำ `/usr/bin`)

---

## [0.5.1] - 2026-06-11

### Added (Sprint 05 Planning)

- **[docs/agile/sprint-backlogs/sprint-05.md](agile/sprint-backlogs/sprint-05.md)**: สร้าง Sprint 05 backlog — Device Discovery & UX (US-MON-18/19), timeline 2026-08-05 → 2026-08-19, implementation plan, auto-detect logic, DoD และ known risks
- **[docs/agile/user-stories/US-MON-18.md](agile/user-stories/US-MON-18.md)**: Auto-detect Disk Devices — `/sys/block` scan, config override, empty fallback
- **[docs/agile/user-stories/US-MON-19.md](agile/user-stories/US-MON-19.md)**: Key Hint Bar (nano-style) — context-aware shortcuts bar
- **[docs/agile/01-product-backlog.md](agile/01-product-backlog.md)** → v1.7: เพิ่ม section "🟣 Device Discovery (Sprint 05)"

---

## [0.5.0] - 2026-06-11

### Implemented (Sprint 04 — Cross-Distribution Support)

- **`src/config.rs`** (US-MON-14/15): shared `Config` struct (`[system]` + `[discord]`); `smartctl_base_cmd()` — root auto-detect via `/proc/self/status` Uid (root → no prefix, non-root → `sudo`), override ด้วย `smartctl_prefix` (`"doas"` / `""` สำหรับ setcap), `smartctl_path`/`iostat_path` overrides; `detect_distro()` parse `/etc/os-release` (Ubuntu/Debian, Fedora/RHEL+clones, Arch+derivatives, openSUSE, Alpine); `check_dependencies()` ทดสอบ `smartctl --version` + `iostat -V` → `Vec<DepError>` พร้อม per-distro install hint
- **`src/widgets/error_screen.rs`** (US-MON-15): banner แสดง tool ที่ขาด + install command ตาม distro — degraded mode (UI ยังทำงาน)
- **`src/collectors/smart.rs` / `iostat.rs`** (US-MON-14): รับ program/args จาก config แทน hardcode `sudo smartctl` / `iostat`
- **`src/notifier.rs`**: refactor → ใช้ shared `config.rs` (Discord config ย้ายไป `Config.discord`)
- **`src/main.rs`** (US-MON-15): `load_config()` → dependency check ก่อน collector loop → เก็บผลใน `AppState.dep_errors`
- **`Makefile`** (US-MON-16): targets `build` / `build-static` (musl) / `install` / `install-static` / `install-service` (systemd หรือ OpenRC ตามระบบ) / `uninstall` / `clean`; **`.cargo/config.toml`**: musl linker; **`contrib/vault-watch.service`** + **`contrib/vault-watch.openrc`**
- **`README.md`** + **`MANUAL.md`** (US-MON-17): README = quick start Ubuntu/Debian + shortcuts + config; MANUAL = per-distro install guide ครบ 5 distro + Privilege Setup (sudo/doas/setcap/root) + Running as a Service; **`contrib/config.example.toml`** annotated ครบทุก option
- **`LICENSE`**: MIT (Kongphai Wutthichaiya)

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
