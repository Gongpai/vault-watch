# Documentation Changelog

ประวัติการปรับปรุงและการเปลี่ยนแปลงเอกสารทั้งหมดในโปรเจค HDD Monitor

---

## [0.18.0] - 2026-07-11

> **MINOR bump:** เริ่ม native SAS/SCSI protocol foundation ของ Sprint 10D ต่อจาก `0.17.4`

### Added

- pure typed read-only SCSI commands สำหรับ TEST UNIT READY, standard/selected VPD INQUIRY และ selected LOG SENSE
- bounds-checked parsers สำหรับ standard INQUIRY, supported VPD pages, temperature LOG SENSE และ fixed/descriptor sense
- identity-free fixtures ครอบ valid, truncated, malformed parameter และ unavailable-temperature sentinel

### Security

- command surface ไม่มี arbitrary opcode/page, data-to-device direction, path, file descriptor หรือ ioctl
- module ยังไม่เชื่อม runtime probing/TUI และรอ privilege broker gate ของ US-MON-37

### Validated

- SCSI targeted tests 6/6, full suite 81/81 และ clippy ผ่าน

## [0.17.4] - 2026-07-11

> **PATCH bump:** บันทึก hardware retest/handoff โดยไม่มี runtime behavior ใหม่ต่อจาก `0.17.3`

### Changed

- Sprint 10B implementation gate ปิดหลัง native diskstats/MD sysfs cutover และ live single-array rebuild verification
- multi-array qualification ถูกส่งต่ออย่างชัดเจนไป US-MON-38/10H โดย US-MON-31 ยังไม่ถูกประกาศ fully qualified

### Known limitations

- BUG-13: ordinary Tab/hold behavior ใช้งานได้ แต่การกดเร็วมากอาจยังพบ terminal-specific focus skipping; ยอมรับ edge case นี้ชั่วคราว

### Security

- เอกสาร hardware retest เก็บเฉพาะ pass/fail behavior ไม่เก็บ array/device identity หรือ raw terminal events

## [0.17.3] - 2026-07-11

> **PATCH bump:** request typed keyboard events เพื่อหยุด repeated Press focus loops ต่อจาก `0.17.2`

### Fixed

- terminal ที่รองรับ keyboard enhancement ถูก request ให้รายงาน Press/Repeat/Release และ encode plain keys อย่างไม่กำกวม
- enhancement mode ถูก pop คืนเมื่อออกจากโปรแกรมทั้งเส้นทางปกติและ panic cleanup
- terminal ที่ไม่รองรับยังคงใช้ compatibility path โดยไม่เปิด protocol flags

### Validated

- fixture ยืนยันว่า requested flags มี event types, all-key encoding และ escape disambiguation ครบ
- edge-triggered focus และ repeat-only scrolling regressions ยังคงผ่าน

## [0.17.2] - 2026-07-11

> **PATCH bump:** แก้ held/early Repeat ทำให้ Tab focus ข้าม panel ต่อจาก `0.17.1`

### Fixed

- Tab/Shift+Tab, view toggles, refresh และ quit เป็น edge-triggered และตอบสนองเฉพาะ Press ครั้งแรก
- Repeat ยังคงเปิดเฉพาะปุ่ม continuous scrolling (`↑`, `↓`, `j`, `k`, Page Up/Down)

### Validated

- regression fixture จำลอง Press ตามด้วย Repeat หลายครั้งและ Release โดย focus ขยับเพียงหนึ่ง panel
- fixture แยกยืนยันว่าการกดปุ่ม scroll ค้างยังเลื่อนต่อเนื่องได้

## [0.17.1] - 2026-07-11

> **PATCH bump:** แก้ Graph focus ขยับซ้ำจาก enhanced-keyboard Release event ต่อจาก `0.17.0`

### Fixed

- keyboard handler ทำงานเฉพาะ Press/Repeat และไม่เปลี่ยน state เมื่อได้รับ Release
- Tab หนึ่งครั้งจึงเลื่อน focus เพียงหนึ่ง panel ตามลำดับ Temperature → Read → Write เมื่อไม่มี RAID graph

### Validated

- regression fixture จำลอง Press+Release sequence และยืนยันว่า focus ขยับเพียงครั้งเดียว
- RAID focus cycle ใช้ fixture state จึงไม่ต้องเริ่ม rebuild จริงเพื่อทดสอบ regression

## [0.17.0] - 2026-07-11

> **MINOR bump:** เพิ่ม Graph metric-scope disclosure และปิด US-MON-32/Sprint 10C ต่อจาก `0.16.1`

### Added

- Graph view แสดง `source=diskstats`, `scope=direct whole-device` และเตือนว่า counters ข้าม stacked layers ไม่สามารถนำมาบวกกันได้
- fixture UI tests ครอบ minimum-size fallback, compact security disclosure, per-frame hitbox reset และ visible-panel focus cycle

### Changed

- Tab/BackTab ใช้ focus-transition helper เดียวกันเพื่อให้ behavior ที่ทดสอบตรงกับ runtime
- feature count เป็น 31 delivered, 1 in progress และ 6 planned

### Security

- scope banner เปิดเผยเฉพาะชนิด kernel source และ metric scope; ไม่แสดง identity, path, serial หรือ user content

### Validated

- scoped graph subjects ยังคงจำกัดเฉพาะ direct whole-device จาก graph inventory; partition/virtual/stacked nodes ไม่ถูกนับซ้ำ
- responsive/focus/scroll tests และ full suite ผ่าน พร้อม live verification ที่บันทึกแบบ sanitized ก่อนหน้า

## [0.16.1] - 2026-07-11

> **PATCH bump:** บันทึก live graph-theme override qualification ต่อจาก `0.16.0` โดยไม่มี runtime behavior ใหม่

### Validated

- custom line palette ถูกใช้ร่วมกันใน legend และ graph series พร้อมวนซ้ำอย่างถูกต้องเมื่อจำนวน subject มากกว่าจำนวนสี
- custom I/O background ถูกนำไปใช้หลัง restart และการ render ไม่ crash
- หลักฐานถูกบันทึกแบบ sanitized โดยไม่เก็บ device identifier หรือค่า identity จากหน้าจอ

## [0.16.0] - 2026-07-11

> **MINOR bump:** เพิ่ม validated graph-theme configuration ต่อจาก `0.15.1` และปิด US-MON-26 Part B

### Added

- `[graph]` supports optional `line_colors`, `temp_zones`, `io_background` และ `label_offset`
- runtime Graph canvases, temperature zones, line legends และ label positioning ใช้ resolved theme เดียวกัน

### Security

- รับเฉพาะ `#RRGGBB`; palette/zone count จำกัด 1..=16, zone max ต้อง finite/เพิ่มขึ้น/ไม่เกิน 200 และ label offset ต้อง finite ใน `-2..=2`
- invalid theme config ถูก reject และแสดง startup error; ไม่มี path, command, escape sequence หรือ arbitrary style payload

### Validated

- config tests ครอบ valid resolution และ invalid color/unordered zones โดยไม่ fallback เงียบ

## [0.15.1] - 2026-07-11

> **PATCH bump:** บันทึก live typed-availability qualification ต่อจาก `0.15.0` โดยไม่มี runtime behavior ใหม่

### Validated

- ตรวจทุก visible device แล้ว availability reasons ไม่สร้าง false `FAIL` alert
- หลักฐานถูกบันทึกแบบ sanitized โดยไม่เก็บ device identifier หรือ diagnostic output

## [0.15.0] - 2026-07-11

> **MINOR bump:** เพิ่ม typed health availability taxonomy ต่อจาก `0.14.0`; alert semantics ยังต้องมี explicit health failure เท่านั้น

### Added

- `MetricAvailability`: Available, Unsupported, Hidden, PermissionDenied, Asleep, TemporarilyUnavailable, Stale, Malformed และ DeviceGone
- legacy SMART boundary จำแนก process I/O error และ sanitized diagnostic text โดยไม่เปลี่ยน unavailable เป็น FAIL
- topology details ใช้ Stale สำหรับ retained partial graph, Hidden สำหรับ stacked physical health และ typed legacy collector reason สำหรับ whole devices

### Security

- availability classification ไม่เปิดไฟล์/device เพิ่มและไม่ log stderr, command arguments หรือ identifiers; UI แสดงเฉพาะ reason label

### Validated

- SMART taxonomy tests 7/7 และ topology scope/availability tests 2/2
- live Topology selection/detail/scroll แสดงถูกต้องและไม่เปิดเผย persistent identity values

## [0.14.0] - 2026-07-11

> **MINOR bump:** เพิ่ม privacy-safe selected-node detail view ต่อจาก `0.13.2`; full availability taxonomy ของ US-MON-32 ยังไม่ครบ

### Added

- Topology แบ่งพื้นที่เป็น scrollable overview และ Selected Node details
- detail แสดง health availability, source, scope, topology confidence, generation presence และ relation counts
- row selection ติดตาม keyboard/mouse scrolling และ clamp อย่างปลอดภัยเมื่อ hot-remove ทำให้รายการสั้นลง

### Security

- detail panel ระบุ `Identity values: REDACTED by UI policy` และไม่ render identity claim, raw `dev_t` หรือ `diskseq`

### Validated

- fixtures ยืนยัน partition เป็น `Unsupported/topology-only/partition` และ partial MD เป็น `TemporarilyUnavailable/md-sysfs/array`

## [0.13.2] - 2026-07-11

> **PATCH bump:** บันทึก live BUG-12 qualification ต่อจาก `0.13.1` โดยไม่มี runtime behavior ใหม่

### Validated

- mouse wheel เลื่อน Topology ได้จริงหลังสลับ view
- scrollbar ของ Topology, Disk Table และ Device Details ถึงปลาย track ถูกต้อง
- หลักฐานถูกบันทึกแบบ sanitized โดยไม่เก็บ device identifier

## [0.13.1] - 2026-07-11

> **PATCH bump:** แก้ shared scrolling regressions ต่อจาก `0.13.0` โดยไม่มี view หรือ metric ใหม่

### Fixed

- BUG-12: ล้าง stale panel rectangles ทุก frame เพื่อให้ mouse wheel ไม่ถูก invisible panel จาก view ก่อนหน้าจับ event
- Disk Table, Device Details และ Topology ใช้จำนวน viewport positions เป็น scrollbar range ทำให้ final offset อยู่ปลาย track จริง

### Validated

- shared scrollbar boundary fixtures 2/2 และ topology privacy fixture ผ่าน; live mouse/endpoint retest ยัง pending

## [0.13.0] - 2026-07-11

> **MINOR bump:** เพิ่ม Storage Topology Overview และ keyboard interaction ใหม่ต่อจาก `0.12.3`; US-MON-32 ยัง In Progress

### Added

- ปุ่ม `t` สลับ Topology Overview/Table และ `g` เปิด Graph จากทุก view
- scrollable topology table จาก graph inventory: node locator, layer, protocol, removable state, confidence, generation presence และ relation counts
- topology availability/source banner พร้อมคำเตือนว่า counters ข้าม stacked layers ไม่ additive

### Security

- topology UI ไม่ render identity claim values, raw `dev_t` หรือ `diskseq`; แสดงเพียง locator ที่ UI ใช้อยู่และ presence ของ generation discriminator

### Validated

- topology privacy fixture ผ่าน; BUG-11 live server retest ยืนยัน speed/ETA คงที่และไม่มี startup/refetch spike

## [0.12.3] - 2026-07-11

> **PATCH bump:** แก้ one-frame MD rebuild speed spike ต่อจาก `0.12.2` โดยไม่เปลี่ยน metric scope หรือเพิ่ม feature

### Fixed

- event hint ที่ทำให้ sample ระยะสั้นหลัง startup ไม่ถูก extrapolate เป็น delta speed หลาย GiB/s
- delta speed ต้องมี observation window อย่างน้อย 2 วินาที; sample ที่สั้นกว่านั้นใช้ kernel `sync_speed`/ETA และรักษา baseline เดิม

### Validated

- MD sysfs fixtures 8/8 รวม 150 ms event-driven spike regression; live server observation ยัง pending

## [0.12.2] - 2026-07-11

> **PATCH bump:** แก้ native MD presentation regression ต่อจาก `0.12.1` โดยไม่มี feature/API ใหม่

### Fixed

- BUG-11: sample ที่ `sync_completed` ยังไม่ขยับไม่ลบ kernel `sync_speed` และ ETA หลัง refresh
- delta sampler เก็บ baseline เดิมระหว่าง unchanged samples เพื่อคำนวณความเร็วจากช่วงที่มี progress จริง

### Validated

- MD sysfs fixtures 7/7 รวม unchanged-progress regression; live server rebuild retest ยัง pending

## [0.12.1] - 2026-07-11

> **PATCH bump:** บันทึก live qualification และปิด Sprint 10A โดยไม่มี runtime behavior ใหม่ต่อจาก `0.12.0`

### Changed

- ปิด US-MON-29 และ Sprint 10A; feature count เป็น 30 delivered, 1 in progress, 7 planned

### Validated

- live removable-storage add: whole/removable/node counts และ UI row ปรากฏทันทีผ่าน event-assisted reconciliation
- live remove: counts/row ลดทันทีโดยไม่ restart หรือ crash
- หลักฐานถูกบันทึกแบบ sanitized โดยไม่เก็บ block name, serial หรือ device identifier

## [0.12.0] - 2026-07-11

> **MINOR bump:** เพิ่ม unprivileged Linux block-event plane ต่อจาก `0.11.0`; event เป็นเพียง hint และไม่แทน periodic transactional sysfs reconciliation

### Added

- read-only `NETLINK_KOBJECT_UEVENT` listener กรองเฉพาะ `SUBSYSTEM=block` โดยไม่เปิด device node หรืออ่าน user content
- bounded event channel และ 150 ms debounce/coalescing; event burst ปลุก collector เพียงหนึ่ง reconciliation cycle
- parser/coalescing tests และ graceful fallback เมื่อเปิดหรืออ่าน netlink ไม่ได้

### Changed

- manual refresh, block events และ periodic timer ใช้ sysfs resnapshot/reconciliation correctness path เดียวกัน

### Security

- uevent payload ไม่ถูกเชื่อเป็น inventory หรือ identity; ใช้เป็น hint ให้สร้าง graph snapshot ใหม่เท่านั้น

### Validated

- event parser/coalescing fixtures ผ่าน; live event-assisted add/remove qualification ยัง pending ก่อนปิด US-MON-29

## [0.11.0] - 2026-07-11

> **MINOR bump:** เพิ่ม executable privacy/security policy และ operator-visible privileged posture ต่อจาก `0.10.0`; ยังเป็น pre-1.0 และ privileged protocol broker ยังคงปิด

### Added

- typed security capability/decision contract สำหรับ metadata, kernel counters, health metadata, outbound notifications, filesystem content, raw sectors และ arbitrary privileged commands
- privacy bar แสดง `privileged broker OFF` ควบคู่ content/network/legacy state
- compact privacy disclosure รักษา content/network/legacy/broker state ครบบน terminal แคบ
- unit tests ยืนยัน content/raw-sector/arbitrary-command default-deny และ network explicit-consent behavior

### Changed

- ปิด US-MON-28 หลังผูก threat-model controls เข้ากับ executable policy; feature count เป็น 29 delivered, 2 in progress, 7 planned

### Security

- TUI/config ไม่มีช่องทางเปิด raw content หรือ arbitrary privileged command; native raw protocol work ยังถูก gate ด้วย US-MON-37

### Validated

- full Rust tests และ clippy `-D warnings` ผ่าน; security disclosure ไม่มี device identifier หรือ secret

## [0.10.0] - 2026-07-11

> **MINOR bump:** เพิ่ม native Linux storage discovery, throughput และ MD RAID functionality หลัง `0.9.0`; โครงการยังเป็น pre-1.0 เพราะ native health/security broker และ hardware qualification ยังไม่ครบ

### Added (Sprint 10 native storage foundation)

- เพิ่ม periodic sysfs topology reconciliation แบบ atomic; failed-empty snapshot รักษา last-known graph เป็น partial และ device incarnation ใหม่แทนรุ่นเดิมผ่าน `diskseq`/`dev_t`
- เพิ่ม synthetic Intel DC P4618 fixture: 2 NVMe whole devices, 6 partitions และ 32 loop devices
- เพิ่ม native `/proc/diskstats` sampler: defensive base/discard/flush parsing, generation/reset-safe delta, 512-byte sector conversion และ scoped MiB/s/IOPS/utilization/latency/queue metrics
- เพิ่ม native MD sysfs backend: name-independent array/member discovery, typed unknown-safe state, progress/speed/ETA, operation-generation reset และ bounded consistency retry
- Device Details แสดง native IOPS, utilization, read/write latency, average queue depth และ in-flight I/O พร้อม source/scope `diskstats/whole`; idle latency เป็น `N/A`

### Changed

- cut over throughput runtime จาก `iostat` เป็น `/proc/diskstats`; iostat parser เหลือ test oracle
- cut over MD runtime จาก `/proc/mdstat` เป็น read-only sysfs primary; mdstat parser เหลือ test oracle
- Privacy bar แยก whole block, NVMe whole devices, partitions และ virtual nodes
- Disk Summary ใช้ graph inventory เติม whole devices และตัด partition/virtual/stacked layers ออกจาก presented throughput เพื่อไม่ double count
- SMART status ใช้ `Healthy`/`Failed`/`Unavailable`; missing/ambiguous data ไม่กลายเป็น failure หรือ zero

### Fixed (Sprint 10 real-hardware regression)

- fix responsive TUI regressions: ขยาย device column เป็น 12, compact privacy counts บน terminal <150 columns และแก้ native throughput label จาก MB/s เป็น MiB/s
- แก้ Intel DC P4618 partition ไม่ให้ถูกนับเป็น NVMe drive และเติม NVMe whole devices ที่เคยหายจากตาราง/กราฟ
- baseline/reset/device absent และ unavailable health ไม่ถูกแสดงเป็นค่าศูนย์หรือ critical failure

### Validated

- openSUSE: P4618 แบบสอง NVMe controllers, SATA disks และ removable storage แสดง inventory/native throughput/Device Details; add/remove ไม่ restart หรือ crash
- Ubuntu server: legacy SMART temperature/health และ MD rebuild panel แสดงผลได้ตามปกติ (หลักฐาน sanitized; ไม่บันทึก serial)
- native MD fixtures 6/6 และ full regression 53/53 ผ่าน; no-array runtime path เริ่มทำงานได้

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
