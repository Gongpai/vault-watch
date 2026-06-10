# HDD Monitor — System Design

**Version:** 0.1.3 | **Last Updated:** 2026-06-10

---

## 1. Data Structures

### 1.1 RaidStatus — จาก `/proc/mdstat`

```rust
pub struct RaidStatus {
    pub name: String,           // "md0"
    pub state: RaidState,       // Active | Rebuilding | Degraded
    pub rebuild_pct: Option<f64>, // 9.3 (เป็น %)
    pub rebuild_speed_mb: Option<u64>, // 178 (MB/s)
    pub eta_minutes: Option<u64>,     // 496 (นาที)
    pub active_disks: u8,       // 3
    pub total_disks: u8,        // 3
}

pub enum RaidState {
    Active,
    Rebuilding,
    Degraded,
    Unknown,
}
```

### 1.2 DiskInfo — จาก `smartctl -a -d scsi`

```rust
pub struct DiskInfo {
    pub device: String,            // "sdc"
    pub serial: Option<String>,     // "XXXX000000" — None เมื่อ smartctl ไม่ตอบสนอง
    pub temperature_c: Option<u8>, // 53
    pub health_ok: bool,           // true = PASSED; default false เมื่อ smartctl ไม่ตอบสนอง (safe default)
    pub power_on_hours: Option<u64>,
    pub grown_defects: Option<u64>,
    pub non_medium_errors: Option<u64>,
    pub read_errors: Option<u64>,
    pub write_errors: Option<u64>,
}
```

### 1.3 IoStats — จาก `iostat -d -k -y 1 1`

```rust
pub struct IoStats {
    pub device: String,      // "sdc"
    pub read_mb_s: f64,      // 178.0
    pub write_mb_s: f64,     // 0.0
}
```

### 1.4 AppState — Shared State

```rust
use std::collections::{HashMap, VecDeque};

// จำนวน sample สูงสุดที่เก็บใน history buffer (60 samples × 2s = 2 นาที)
pub const HISTORY_SIZE: usize = 60;

pub struct AppState {
    pub raid: Option<RaidStatus>,
    pub disks: Vec<DiskInfo>,    // one DiskInfo per device; matched to disk_devices by DiskInfo.device
    pub io_stats: Vec<IoStats>,  // one IoStats per device; matched to disk_devices by IoStats.device
    pub last_updated: std::time::Instant,
    pub disk_devices: Vec<String>, // master device list ["sdc", "sdd", "sde"] — key ที่ใช้ link ทุก collection

    // History ring buffers สำหรับ graph — key = device name (ตรงกับ disk_devices)
    // ใช้ VecDeque เพื่อ push_back O(1) และ pop_front O(1)
    pub temp_history: HashMap<String, VecDeque<u64>>,   // °C per device
    pub read_history: HashMap<String, VecDeque<u64>>,   // MB/s × 10 per device (เก็บ 1 decimal)
    pub write_history: HashMap<String, VecDeque<u64>>,  // MB/s × 10 per device
    pub raid_speed_history: VecDeque<u64>,              // MB/s (RAID-level ไม่ใช่ per device)

    // View & navigation state
    pub view_mode: ViewMode,
    pub focused_panel: FocusedPanel,
    pub disk_table_scroll: usize,    // index ของ disk แถวแรกที่แสดง
    pub smart_details_scroll: usize, // index ของ disk แถวแรกที่แสดงใน SMART panel
    pub graph_scroll: usize,         // scroll ใน Graph View (ถ้า disk มากกว่าที่ chart รองรับ)

    // Rendered panel bounds — อัปเดตทุก frame เพื่อใช้ตรวจจับ mouse click/scroll
    pub panel_rects: HashMap<FocusedPanel, ratatui::layout::Rect>,
}

pub enum ViewMode {
    Table,  // default — disk table พร้อม inline sparklines
    Graph,  // expanded chart panels (toggle ด้วย g)
}

#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub enum FocusedPanel {
    // Table View
    DiskTable,
    SmartDetails,
    // Graph View
    TempGraph,
    ThroughputGraph,
    RaidGraph,
}
```

**Force refresh:** ไม่มี `force_refresh` field ใน AppState — ใช้ `Arc<tokio::sync::Notify>` แยกต่างหาก ส่งเป็น parameter ไปยัง collector task เมื่อกด `r` event loop เรียก `notify.notify_one()` → collector ถูกปลุกออกจาก `sleep` ทันที

```rust
// ใน main.rs
let refresh_notify = Arc::new(tokio::sync::Notify::new());
// ใน collector loop
tokio::select! {
    _ = tokio::time::sleep(Duration::from_secs(2)) => {}
    _ = refresh_notify.notified() => {}
}
// collect data...
```

**Per-device data model:** `disk_devices` คือ master list — ทุก collection ใช้ device name เดียวกันเป็น key:

```
disk_devices = ["sdc", "sdd", "sde"]
     │
     ├── disks[i].device        → temperature_c, health_ok, serial, …  (per device)
     ├── io_stats[i].device     → read_mb_s, write_mb_s                (per device)
     ├── temp_history["sdc"]    → VecDeque<u64> อุณหภูมิย้อนหลัง     (per device)
     ├── read_history["sdc"]    → VecDeque<u64> read speed ย้อนหลัง   (per device)
     └── write_history["sdc"]   → VecDeque<u64> write speed ย้อนหลัง  (per device)
```

UI merge ข้อมูลโดยวน loop `disk_devices` แล้ว lookup `disks`, `io_stats`, และ history maps ด้วย device name เดียวกัน

**History buffer logic:** ทุกครั้งที่ collector อัปเดต ให้ `push_back` ค่าใหม่ และ `pop_front` ถ้า `len() > HISTORY_SIZE`

**Scroll logic:** `disk_table_scroll` คือ index แถวแรกที่แสดง — จำกัดให้ `scroll <= max(0, disk_count - visible_rows)`

---

## 2. Parser Specifications

### 2.1 /proc/mdstat Parser

**ตัวอย่าง Input:**

```text
Personalities : [raid10]
md0 : active raid10 sdc[0] sdd[1] sde[2]
      11718504448 blocks super 1.2 512K chunks 2 near-copies [3/3] [UUU]
      [==>..................]  resync =  9.3% (1090263040/11718504448) finish=60.5min speed=178031K/sec

unused devices: <none>
```

**Regex Patterns:**

```rust
// สถานะ array
r"^(\w+)\s*:\s*(active|inactive)\s+(\w+)"

// rebuild progress
r"\[([=>\.]+)\]\s+(?:resync|recovery|check|repair)\s*=\s*([\d.]+)%"

// speed & ETA
r"speed=(\d+)K/sec"
r"finish=([\d.]+)min"

// disk count
r"\[(\d+)/(\d+)\]"
```

**Logic:**
- หากไม่มีบรรทัด rebuild → `state = Active`
- หากมีบรรทัด rebuild → `state = Rebuilding`, คำนวณ `eta_minutes`
- `speed` หน่วยเป็น K/sec → หาร 1024 เพื่อได้ MB/s

---

### 2.2 smartctl Parser

**Command:** `sudo smartctl -a -d scsi /dev/sdc`

**ตัวอย่าง Output ที่ต้องการ:**

```text
Serial number:        XXXX000000
Current Drive Temperature:  53 C
SMART Health Status: OK
Power_On_Hours:       12345
Elements in grown defect list: 7
Non-medium error count:  16373
```

**Regex Patterns:**

```rust
r"Serial number:\s+(\S+)"
r"Current Drive Temperature:\s+(\d+) C"
r"SMART Health Status:\s+(\w+)"
r"Power_On_Hours:\s+(\d+)"
r"Elements in grown defect list:\s+(\d+)"
r"Non-medium error count:\s+(\d+)"
r"read:\s+\S+\s+\S+\s+\S+\s+\S+\s+(\d+)"   // read errors
r"write:\s+\S+\s+\S+\s+\S+\s+\S+\s+(\d+)"  // write errors
```

**หมายเหตุ:** ต้องรัน `smartctl` ด้วย `sudo` — ระบบต้องมี sudoers entry หรือรัน binary ด้วย setuid

---

### 2.3 iostat Parser

**Command:** `iostat -d -k -y 1 1 sdc sdd sde`

> **หมายเหตุสำคัญ:** ต้องใช้ `-y 1 1` เสมอ
> - `-y` = ข้าม first report ซึ่งเป็นค่าเฉลี่ย **ตั้งแต่ boot** ไม่ใช่ real-time
> - `1 1` = sample interval 1 วินาที, จำนวน 1 ครั้ง → ได้ค่า **throughput ณ ขณะนั้นจริง**
>
> `iostat -d -k sdc sdd sde` (ไม่มี interval) ให้ค่าเฉลี่ย since-boot ซึ่งไม่เหมาะสำหรับ realtime monitoring

**ตัวอย่าง Output:**

```text
Device             tps    kB_read/s    kB_wrtn/s    kB_dscd/s    kB_read    kB_wrtn    kB_dscd
sdc               0.00         0.00         0.00         0.00          0          0          0
sdd             178.00    182304.00         0.00         0.00    1090263      0          0
sde               0.00         0.00       182304.00     0.00          0    1090263      0
```

**Parsing Logic:**
- Skip บรรทัด header (`Device`, `Linux`, blank) และบรรทัดว่าง
- แต่ละแถวข้อมูล: field[0]=device, field[2]=kB_read/s, field[3]=kB_wrtn/s
- หาร 1024 เพื่อแปลง kB/s → MB/s
- เนื่องจากใช้ `-y 1 1` output จะมี 2 blocks — ใช้ **block ที่สอง** เสมอ (block แรกอาจยังเป็น since-boot บน kernel เก่า)

---

## 3. UI Layout Specification

มี 2 view modes สลับด้วยปุ่ม `g`

---

### 3.1 Table View (default — `ViewMode::Table`)

ค่าตัวเลขทุกตัว (Temperature, Read MB/s, Write MB/s, RAID speed) แสดงเป็น **inline Sparkline** แสดงประวัติ 2 นาทีที่ผ่านมา ควบคู่กับค่าปัจจุบัน รองรับ **scroll** และ **panel focus** สำหรับ disk จำนวนมาก

**Scenario: 8 disks, terminal แสดง disk table ได้ 5 แถว, focus อยู่ที่ DiskTable:**

```
┌──────────────────────────────────────────────────────────────────────────────────────┐
│ HDD Monitor  q:quit  g:graph  Tab:panel  r:refresh              Last: 14:32:05      │
├──────────────────────────────────────────────────────────────────────────────────────┤
│ RAID  md0  REBUILDING  [████░░░░░░░░░░░░░░░░░]  9.3%  Disks 8/8                    │
│            Speed  ▁▂▃▄▅▄▃▄▅▆▅▄▅▄▃▄▅▄▅▃  178 MB/s    ETA 8h 16m                    │
╔══════╦══════════════════════════╦═════════════════════════╦══════════╦═══════════════╗
║ Disk ║ Temperature              ║ Read MB/s               ║Write MB/s║    Health     ║▲
╠══════╬══════════════════════════╬═════════════════════════╬══════════╬═══════════════╣█
║ sdc  ║ ▃▄▄▅▄▄▃▄▅▄▃▄  50°C      ║ ▁▁▁▁▁▁▁▁▁▁▁▁   0.0     ║▁▁▁▁  0.0║     OK        ║█
║ sdd  ║ ▅▆▆▅▆▅▆▅▅▆▅▆  53°C      ║ ████████████ 178.2      ║▁▁▁▁  0.0║     OK        ║░
║ sde  ║ ▃▄▃▄▄▃▃▄▃▄▃▄  48°C      ║ ▁▁▁▁▁▁▁▁▁▁▁▁   0.0     ║████ 178.1║  OK  ⚠D:7   ║░
║ sdf  ║ ▃▄▄▅▄▃▃▄▄▅▃▄  47°C      ║ ▁▁▁▁▁▁▁▁▁▁▁▁   0.0     ║▁▁▁▁  0.0║     OK        ║░
║ sdg  ║ ▄▄▃▄▃▄▄▅▄▃▄▃  49°C      ║ ▁▁▁▁▁▁▁▁▁▁▁▁   0.0     ║▁▁▁▁  0.0║     OK        ║▼
╚══════╩══════════════════════════╩═════════════════════════╩══════════╩═══════════════╝
│ ● DiskTable [5/8 — ↑↓:scroll]   ○ SmartDetails                                     │
├──────────────────────────────────────────────────────────────────────────────────────┤
│ SMART DETAILS                                                                      ▲ │
│ sdc  Serial: XXXX000   Power-on: 12345h   NME: 16373                              █ │
│ sdd  Serial: YYYY000   Power-on: 12340h   NME: 32025                              ░ │
│ sde  Serial: ZZZZ000   Power-on: 12338h   Grown defects: 7 ⚠                     ▼ │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

**Visual indicators:**
- **Focused panel**: double border `╔╦╗╠╬╣╚╩╝` สีสว่าง; unfocused ใช้ single border `┌┬┐├┼┤└┴┘` สีปกติ
- **Scrollbar**: `ratatui::widgets::Scrollbar` (VerticalRight) ทางขวาของทุก panel ที่ scroll ได้ แสดง `▲ █ ░ ▼`
- **Status bar**: บรรทัดระหว่าง DiskTable กับ SmartDetails — `● Panel [N/total — hint]` = focused, `○ Panel` = unfocused
- **Overflow hint**: เมื่อมีแถวซ่อนอยู่ด้านล่างสุด แสดง `↓ 3 more` ที่แถวสุดท้ายของ panel
- **Mouse support**: scroll wheel บน panel ใดก็ scroll panel นั้น; click โฟกัส panel นั้น

**Sparkline widget:** `ratatui::widgets::Sparkline` ใช้ Unicode block chars `▁▂▃▄▅▆▇█`
- Temperature column: แสดง 12 sample ล่าสุด (24 วินาที) + ค่าปัจจุบัน + สี
- Read/Write column: แสดง 12 sample ล่าสุด + ค่าปัจจุบัน (MB/s)
- RAID speed row: แสดง 20 sample ล่าสุด ใต้ progress bar

---

### 3.2 Graph View (`ViewMode::Graph`)

แสดง line chart แบบ full-screen โดยใช้ `ratatui::widgets::Chart` พร้อม axis labels สลับด้วย `g` รองรับ scroll ผ่าน `graph_scroll` เมื่อ disk มีมากกว่าที่ chart แสดงได้

```
┌──────────────────────────────────────────────────────────────────────────────────────┐
│ HDD Monitor    q:quit  r:refresh  g:table  Tab:panel            Last: 14:32:05      │
├────────────────────────────────────────┬─────────────────────────────────────────────┤
│ TEMPERATURE (°C) — last 2 min          │ THROUGHPUT (MB/s) — last 2 min              │
│                                        │                                             │
│ 60┤                                    │ 200┤            ████  ██████               │
│   │       ·  ·  · ·  · · ·  ·  · sdd  │    │         ████████████████  sdd R        │
│ 55┤  ···· · ·· · · ·· ·  ·· ·         │ 150┤      ████████████████████              │
│   │                                    │    │                                        │
│ 50┤  ──────────────────────────  sdc   │ 100┤                                        │
│   │  ···  ··· ··· ··· ···  ···  sde   │    │                                        │
│ 45┤                                    │  50┤                                        │
│   │                                    │    │  ───────────────────────── sde W       │
│ 40└────────────────────────── 2min     │   0└──────────────────────────── 2min      │
├────────────────────────────────────────┤                                             │
│ RAID REBUILD SPEED (MB/s) — last 2 min │                                             │
│                                        │                                             │
│ 200┤  ──────────────────────── 178 MB/s│                                             │
│    │                                    │                                             │
│   0└────────────────────────── 2min    │                                             │
└────────────────────────────────────────┴─────────────────────────────────────────────┘
```

**Chart widget:** `ratatui::widgets::Chart` + `Dataset` per disk per metric
- X axis: เวลา (วินาทีย้อนหลัง `0` = ปัจจุบัน, `-120` = 2 นาทีที่แล้ว)
- Y axis Temperature: range dynamic (min-10 ถึง max+5 °C)
- Y axis Throughput: range 0 ถึง max+20 MB/s
- แต่ละ disk ใช้คนละสี: sdc=Cyan, sdd=Yellow, sde=Green
- Read=solid line, Write=dashed (ต่างกันด้วย marker style)

---

### 3.3 Color Scheme

| ค่า / สถานะ | สี |
| :--- | :--- |
| Temperature < 45°C | Green |
| Temperature 45–55°C | Yellow |
| Temperature > 55°C | Red |
| SMART Health OK | Green |
| SMART Health FAIL | Red |
| RAID Active | Green |
| RAID Rebuilding | Yellow |
| RAID Degraded | Red |
| Defects = 0 | White (default) |
| Defects > 0 | Yellow + ⚠ |

---

### 3.4 Layout Constraints

- Terminal minimum size: **100×28** (Table view), **110×30** (Graph view)
- รองรับ dynamic disk count (ไม่ hardcode 3 disks)
- Disk Table: column widths ยืดหยุ่นตาม terminal width — Sparkline ยืดเต็มช่องที่เหลือ
- Graph View: left/right panels แบ่ง 50/50 horizontal

---

### 3.5 Keyboard & Mouse Interaction

| Input | Action |
|:---|:---|
| `q` | ออกจากโปรแกรม (cleanup terminal) |
| `r` | Force refresh ทันที |
| `g` | Toggle Table View ↔ Graph View |
| `Tab` | ย้าย focus ไปยัง panel ถัดไป (cycle forward) |
| `Shift+Tab` | ย้าย focus ไปยัง panel ก่อนหน้า (cycle backward) |
| `↑` / `k` | Scroll focused panel ขึ้น 1 แถว |
| `↓` / `j` | Scroll focused panel ลง 1 แถว |
| `PgUp` | Scroll focused panel ขึ้น `visible_rows - 1` แถว |
| `PgDn` | Scroll focused panel ลง `visible_rows - 1` แถว |
| `Home` | Scroll focused panel กลับไปแถวแรก |
| `End` | Scroll focused panel ไปยังแถวสุดท้าย |
| Mouse wheel up | Scroll panel ที่เมาส์อยู่ขึ้น 3 แถว |
| Mouse wheel down | Scroll panel ที่เมาส์อยู่ลง 3 แถว |
| Mouse click | โฟกัส panel ที่ click |

**Panel cycle order (Table View):** `DiskTable` → `SmartDetails` → `DiskTable` → …

**Panel cycle order (Graph View):** `TempGraph` → `ThroughputGraph` → `RaidGraph` → `TempGraph` → …

---

### 3.6 Scroll State Logic

```
// จำนวนแถวที่แสดงได้ = panel height - header rows - border rows
let visible_rows = panel_rect.height as usize - 2;

// จำกัด scroll ไม่ให้เกิน
let max_scroll = disk_count.saturating_sub(visible_rows);
state.disk_table_scroll = state.disk_table_scroll.min(max_scroll);

// แถวที่แสดง = slice ของ disks[scroll .. scroll + visible_rows]
```

**Mouse hit-testing:** ทุก frame ที่ render ให้บันทึก `Rect` ของแต่ละ panel ลงใน `panel_rects` จากนั้นเมื่อรับ `MouseEvent` ให้วน loop หา panel ที่ครอบ coordinate ของ cursor

```rust
fn panel_at(rects: &HashMap<FocusedPanel, Rect>, col: u16, row: u16) -> Option<FocusedPanel> {
    rects.iter()
        .find(|(_, r)| col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height)
        .map(|(panel, _)| *panel)
}
```

---

## 4. Error Handling

| Scenario | Behavior |
| :--- | :--- |
| `smartctl` ไม่ติดตั้ง / permission denied | แสดง `N/A` ในทุก SMART field, log error |
| `/proc/mdstat` ไม่พบ RAID array | แสดง "No RAID array detected" ใน RAID panel |
| `iostat` ไม่ติดตั้ง | แสดง `--` ใน throughput columns |
| Disk ไม่ response | แสดง `TIMEOUT` ใน health column |
| Parse error (unexpected format) | ใช้ `None` สำหรับ optional fields, ไม่ panic |

---

## 5. Dependencies

```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
regex = "1"
```

---

## Related Documents

- ภาพรวมสถาปัตยกรรม: [00-architecture.md](./00-architecture.md)
- User stories: [agile/user-stories/](../agile/user-stories/)
