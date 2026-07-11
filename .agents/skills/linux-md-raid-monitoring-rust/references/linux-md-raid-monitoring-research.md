# Linux MD Software RAID Monitoring ด้วย Rust: Native Sysfs Backend, Event Design และ Architecture

***

## A. Executive Summary

**สรุปหลัก**: สามารถสร้าง native MD RAID monitoring backend ใน Rust ได้อย่างสมบูรณ์โดยไม่ต้อง fork `mdadm` เนื่องจาก Linux kernel expose ข้อมูลทุกอย่างที่จำเป็นผ่าน sysfs ที่ `/sys/block/md*/md/` ซึ่งเป็น **stable, documented kernel ABI** ตั้งแต่ kernel 2.6.12[^1]

**ข้อสรุปสำคัญจากเอกสาร**:
- `/sys/block/md*/md/` sysfs attributes ทั้งหมดที่อธิบายในเอกสารนี้ **documented ใน `Documentation/admin-guide/md.rst`** ซึ่งเป็นส่วนหนึ่งของ official Linux kernel documentation[^2]
- `/proc/mdstat` ถูก designed สำหรับ human readability ไม่ใช่ machine parsing — ไม่มี documented stability guarantee สำหรับ text format[^3]
- `inotify` **ไม่รองรับ** sysfs/procfs — ต้องใช้ `select()`/`poll()` บน sysfs attributes โดยตรง หรือ udev netlink สำหรับ hot-plug events[^4]
- สำหรับ **external metadata arrays** (IMSM, DDF) kernel suspend IO แล้วรอ user-space (`mdmon`) — monitoring software ต้องรับทราบว่าตน **ต้องไม่ฆ่า mdmon** และต้องอ่านข้อมูลผ่าน sysfs เท่านั้น ไม่ใช่ผ่าน container ioctls[^5]

**สิ่งที่ต้องยืนยันด้วย kernel source / VM testing**:
- พฤติกรรมที่แน่นอนของ `sync_completed` เมื่อ `sync_action` เปลี่ยนสถานะระหว่างที่กำลัง read
- `consistency_policy` ค่า `journal` — kernel version ที่เพิ่มเข้ามาอย่างแน่นอน (ประมาณ 4.10–4.15)
- `uuid` attribute ใน sysfs — ไม่ได้ documented ใน v4.10 แต่ปรากฏใน v6.x docs[^2]

***

## B. Recommended Data-Source Priority

### B.1 Priority ของ Data Sources

| Priority | Source | ใช้เมื่อ | ความเสถียร |
|----------|--------|---------|-----------|
| **P1** | `/sys/block/md*/md/` (sysfs) | Primary source — ข้อมูลทั้งหมด | **Stable ABI** (docs.kernel.org)[^2] |
| **P2** | `poll()/select()` บน sysfs attributes | Event-driven: `array_state`, `sync_action`, `degraded`, member `state` | **Supported** (docs ระบุว่า file responds to select/poll)[^2] |
| **P3** | udev netlink (`SUBSYSTEM=="block"`, `KERNEL=="md*"`) | Hot-plug: array created/removed, member added/removed | **Stable** (udev rules docs)[^6] |
| **P4** | `/proc/mdstat` | Fallback validation; ข้อมูลบางอย่างที่ sysfs ไม่มี | **Unstable format** — ไม่มี format ABI guarantee[^7] |

### B.2 ข้อมูลที่มีเฉพาะใน `/proc/mdstat` (ไม่มีใน sysfs โดยตรง)

- `read_ahead` value (ไม่สำคัญสำหรับ monitoring)
- Human-readable device status bitmap เช่น `[UU_U]` — สามารถสร้างใหม่จาก sysfs member states ได้

### B.3 ข้อมูลที่มีใน sysfs แต่ไม่มีใน `/proc/mdstat`

- `consistency_policy` (resync/bitmap/ppl/journal/none)
- `uuid` (ใน newer kernels)
- Member-level attributes: `recovery_start`, `bad_blocks`, `errors`, `want_replacement`, `replacement`
- Bitmap attributes: `bitmap/location`, `bitmap/chunksize`, `bitmap/metadata`
- `reshape_position`, `sync_min`, `sync_max`
- `safe_mode_delay`

***

## C. ตาราง Sysfs Attributes

### C.1 Array-Level Attributes (`/sys/block/mdN/md/`)

| Attribute | Type | Possible Values | Poll-able | Min Kernel | Stability | Permissions |
|-----------|------|-----------------|-----------|-----------|-----------|-------------|
| `level` | string | `raid0`, `raid1`, `raid4`, `raid5`, `raid6`, `raid10`, `linear`, `multipath`, `faulty` | ❌ | 2.6.12 | **Stable** (documented)[^2] | `r--r--r--` |
| `raid_disks` | integer | N ≥ 1 (or empty if unknown) | ❌ | 2.6.12 | Stable | `rw-r--r--` |
| `array_state` | string | `clear`, `inactive`, `readonly`, `read-auto`, `clean`, `active`, `write-pending`, `active-idle` | ✅ | 2.6.12 | **Stable**[^2] | `rw-r--r--` |
| `metadata_version` | string | `0.90`, `1.0`, `1.1`, `1.2`, `none`, `external:<name>` | ❌ | 2.6.12 | Stable | `r--r--r--` |
| `chunk_size` | integer (bytes) | multiples of PAGE_SIZE | ❌ | 2.6.12 | Stable | `rw-r--r--` |
| `layout` | integer | level-specific number | ❌ | 2.6.12 | Stable | `rw-r--r--` |
| `component_size` | integer (sectors) | device-specific | ❌ | 2.6.12 | Stable | `rw-r--r--` |
| `array_size` | integer (KiB) or `default` | KiB or "default" | ❌ | 2.6.27 | Stable | `rw-r--r--` |
| `resync_start` | integer (sectors) or `none` | sector number, `none` (since 2.6.30-rc1) | ❌ | 2.6.12 | Stable[^2] | `rw-r--r--` |
| `degraded` | integer | 0, 1, 2, ... | ✅ | 2.6.12 | **Stable**[^2] | `r--r--r--` |
| `sync_action` | string | `idle`, `resync`, `recover`, `check`, `repair` | ✅ | 2.6.12 | **Stable**[^2] | `rw-r--r--` |
| `sync_completed` | string | `"N / M"` (sectors) | ✅ | 2.6.12 | Stable[^1] | `r--r--r--` |
| `sync_speed` | integer (KiB/s) | averaged over last 30s | ❌ | 2.6.12 | Stable[^2] | `r--r--r--` |
| `sync_speed_min` | string | integer + `(local)` or `(system)` | ❌ | 2.6.12 | Stable | `rw-r--r--` |
| `sync_speed_max` | string | integer + `(local)` or `(system)` | ❌ | 2.6.12 | Stable | `rw-r--r--` |
| `mismatch_cnt` | integer | sector count | ❌ | 2.6.12 | Stable[^2] | `r--r--r--` |
| `reshape_position` | string | `none` or sector number | ❌ | 2.6.18 | Stable[^2] | `rw-r--r--` |
| `consistency_policy` | string | `none`, `resync`, `bitmap`, `ppl`, `journal` | ❌ | ~4.10 (journal); ~4.15 (ppl)[^8] | Stable[^2] | `rw-r--r--` |
| `uuid` | string | `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` | ❌ | ~5.x | Stable (added later)[^2] | `r--r--r--` |
| `safe_mode_delay` | float (seconds) | default 0.200 | ❌ | 2.6.12 | Stable | `rw-r--r--` |
| `bitmap/location` | string | `none`, `file`, `[+-]N` | ❌ | 2.6.12 | Stable (when bitmap active) | `rw-r--r--` |
| `bitmap/metadata` | string | `internal`, `external` | ❌ | 2.6.12 | Stable | `rw-r--r--` |
| `last_sync_action` | string | `check`, `repair`, `resync`, `recover` | ❌ | ~4.x | Observed in production[^9] | `r--r--r--` |
| `stripe_cache_size` | integer | 17–32768, default 256 | ❌ | 2.6.12 | **Module-specific, may change**[^2] | `rw-r--r--` |

> **⚠️ ข้อควรระวัง [ต้องยืนยันด้วย kernel source]**: `consistency_policy` ค่า `journal` ถูกเพิ่มในช่วง kernel 4.10–4.15 และ `ppl` ถูกเพิ่มในช่วง 4.15–4.19 ค่าที่แน่นอนต้องตรวจสอบจาก git log ของ `drivers/md/`

### C.2 Member Device Attributes (`/sys/block/mdN/md/dev-XXX/`)

| Attribute | Type | Possible Values | Poll-able | Min Kernel | Notes |
|-----------|------|-----------------|-----------|-----------|-------|
| `state` | string (CSV) | `faulty`, `in_sync`, `writemostly`, `blocked`, `spare`, `write_error`, `want_replacement`, `replacement` | ✅ (faulty/blocked) | 2.6.12 | **Stable**[^2] |
| `slot` | string | `none` หรือ integer 0..raid_disks-1 | ❌ | 2.6.12 | Stable[^2] |
| `errors` | integer | approx read error count | ❌ | 2.6.12 | Stable |
| `recovery_start` | string | `none` หรือ sector number | ❌ | 2.6.12 | Stable[^2] |
| `offset` | integer (sectors) | device-specific | ❌ | 2.6.12 | Stable |
| `size` | integer (sectors) | ≤ component_size | ❌ | 2.6.12 | Stable |
| `bad_blocks` | multi-line | `start length` pairs | ❌ | ~3.5 | Stable (BBL feature) |
| `ppl_sector` | integer | sector location of PPL | ❌ | ~4.15 | Present when PPL active[^8] |
| `ppl_size` | integer | size in sectors of PPL | ❌ | ~4.15 | Present when PPL active |

### C.3 sync_completed Special Values

| ค่าที่อ่านได้ | ความหมาย | วิธี parse |
|---------------|---------|-----------|
| `"N / M"` | N sectors completed of M total | split by `/`, trim whitespace, parse u64 |
| `"none"` | ไม่มี sync operation กำลังเกิดขึ้น | ไม่มี progress |
| ค่าว่าง (empty) | array ไม่ support sync | อ่านไม่ได้ |

> **[จาก kernel docs + prometheus/procfs PR #509]** sync_completed และ sync_speed ถูก expose เฉพาะขณะที่ sync กำลังเกิดขึ้น เมื่อ `sync_action == "idle"` ให้ถือว่า completion = 100% (all blocks in sync)[^1]

***

## D. Snapshot Collection Algorithm

### D.1 ขั้นตอนทีละขั้น

```
START: Enumerate all MD arrays
│
├── STEP 1: Find all md devices
│   readdir("/sys/block/") → filter entries that start with "md"
│   Note: ไม่ assume ชื่อ "md0" — อาจเป็น "md_d0", "md127", "md/myarray"
│   Also check /dev/md/ via udev symlinks สำหรับ named arrays
│
├── STEP 2: สำหรับแต่ละ md device X:
│   a. open /sys/block/mdX/md/array_state — ถ้า ENOENT → ไม่ใช่ md device
│   b. อ่าน array_state → ถ้า "clear" หรือ "inactive" → skip (no active array)
│   c. อ่าน metadata_version → ถ้าขึ้นต้นด้วย "external:" → external metadata path
│
├── STEP 3: อ่าน array-level attributes (ใน order ที่ minimize TOCTOU):
│   Priority: level → raid_disks → array_state → degraded → sync_action
│             → sync_completed → sync_speed → mismatch_cnt
│             → consistency_policy → reshape_position → metadata_version
│             → chunk_size → component_size → uuid
│   Note: อ่าน sync_action และ sync_completed ในลำดับนี้เสมอ (ดู section I)
│
├── STEP 4: Enumerate member devices
│   readdir("/sys/block/mdX/md/") → filter directories starting with "dev-"
│   สำหรับแต่ละ dev-YYY:
│   a. อ่าน state (comma-separated flags)
│   b. อ่าน slot
│   c. อ่าน errors
│   d. อ่าน recovery_start (สำหรับ spare/recovery devices)
│   e. resolve symlink "block" → ได้ชื่อ block device จริง
│   f. resolve rdN symlinks → map slot number to dev-XXX
│
├── STEP 5: อ่าน bitmap state (conditional)
│   ถ้า consistency_policy == "bitmap" หรือ bitmap/location ≠ "none":
│   อ่าน bitmap/location, bitmap/chunksize, bitmap/metadata
│
└── STEP 6: Record snapshot timestamp (monotonic clock)
    ใช้ CLOCK_MONOTONIC สำหรับคำนวณ elapsed time ระหว่าง samples
```

### D.2 Snapshot Consistency

**[ข้อเสนอแนะผู้วิจัย]** sysfs ไม่มี atomic snapshot mechanism — แต่ละ attribute เป็น individual file read แยกกัน ให้อ่านตาม order ของ "ความเสี่ยงต่อ race condition":

1. อ่าน `array_state` ก่อนสุด — ถ้าเปลี่ยนเป็น `inactive` ระหว่างที่อ่านอยู่ ให้ discard snapshot ทั้งหมด
2. อ่าน `sync_action` ก่อน `sync_completed` — ถ้า `sync_action == "idle"` อย่าอ่าน `sync_completed` (หรืออ่านแล้วตีความว่า N/A)
3. หากพบ `ENOENT` หรือ `EIO` ระหว่าง read → อาจเป็น hot-remove → log event, mark array as disappeared

***

## E. Rebuild/Progress/ETA Formulas

### E.1 ค่าที่ kernel ให้โดยตรง vs ค่าที่ต้องคำนวณ

| Metric | Kernel provides? | วิธีได้มา |
|--------|-----------------|----------|
| Sectors completed | ✅ ใน `sync_completed` | parse `N / M` |
| Sectors total | ✅ ใน `sync_completed` | parse `N / M` |
| Current speed (KiB/s) | ✅ `sync_speed` (30s average) | อ่านโดยตรง |
| Percentage complete | ❌ ต้องคำนวณ | `N / M × 100` |
| Bytes remaining | ❌ ต้องคำนวณ | `(M - N) × 512` bytes |
| ETA (seconds) | ❌ ต้องคำนวณ | `(M - N) × 512 / (sync_speed × 1024)` |
| Instantaneous speed | ❌ (kernel gives 30s avg) | `Δsectors / Δtime` between snapshots |

### E.2 สูตรคำนวณ

**Percentage complete:**
\[ \text{pct} = \frac{N}{M} \times 100 \]

**Bytes remaining:**
\[ \text{bytes\_remaining} = (M - N) \times 512 \]

> หมายเหตุ: `sync_completed` units เป็น sectors และ **สำหรับ md, 1 sector = 512 bytes เสมอ** (kernel constant) ไม่ขึ้นกับ physical sector size ของ member drives[^1]

**ETA (วินาที):**
\[ \text{eta\_s} = \frac{(M - N) \times 512}{sync\_speed \times 1024} \]

> `sync_speed` unit เป็น **KiB/s**  — ต้องคูณด้วย 1024 เมื่อเปรียบเทียบกับ bytes remaining[^2]

**ตัวอย่างตัวเลขจริง:**

จาก bugzilla kernel.org (raid6, 8 disks):[^9]
```
sync_completed: 3566405120 / 15627790336
sync_speed: 126 (KiB/s)
```

\[ \text{pct} = \frac{3{,}566{,}405{,}120}{15{,}627{,}790{,}336} = 22.8\% \]

\[ \text{bytes\_remaining} = (15{,}627{,}790{,}336 - 3{,}566{,}405{,}120) \times 512 = 6{,}175{,}680{,}614{,}912 \approx 5.6 \text{ TiB} \]

\[ \text{eta} = \frac{6{,}175{,}680{,}614{,}912}{126 \times 1024} = 47{,}938{,}528 \text{ s} \approx 555 \text{ days} \]

> **[ข้อควรระวัง]** `sync_speed` = 126 KiB/s ใน example นั้นต่ำผิดปกติ (อาจ throttled หรือ I/O contention) — ใน production ควรแสดง real-time speed จาก `Δsync_completed / Δtime` ระหว่าง samples และใช้เป็น basis สำหรับ ETA แทน

**Instantaneous speed จาก delta (แนะนำ):**
\[ \text{speed\_kib} = \frac{(N_2 - N_1) \times 512}{(t_2 - t_1) \times 1024} \]

***

## F. Event-Monitoring Design

### F.1 ทำไม inotify ไม่ work

**[documented — inotify limitation]**: `inotify` ไม่รองรับ pseudo-filesystems เช่น `/proc`, `/sys` เนื่องจาก inotify รายงานเฉพาะ events ที่ triggered โดย user-space filesystem API — kernel pseudo-FS ไม่ trigger เหล่านี้[^4]

### F.2 Sysfs Poll/Select (สำหรับ state changes)

**[documented — kernel docs]** sysfs attributes บางตัวรองรับ `poll()`/`select()` อย่างแท้จริง เมื่อค่าเปลี่ยน kernel จะ notify ด้วย `POLLERR | POLLPRI`[^10]

Attributes ที่รองรับ poll/select ใน MD:
- `array_state` — เปลี่ยนทุกครั้งยกเว้น `active ↔ active-idle`[^2]
- `sync_action` — เปลี่ยนเมื่อ operation เริ่ม/หยุด[^2]
- `degraded` — เปลี่ยนเมื่อ device fail/recover[^2]
- `dev-XXX/state` — เปลี่ยนเมื่อ member device faulty/blocked[^2]
- `sync_completed` — เปลี่ยนเมื่อ sync completes หรือถึง sync_max[^2]

**⚠️ สำคัญ**: หลังจาก `poll()` return ต้อง **close และ re-open file** ก่อน read ใหม่ เพราะการ seek+read จะไม่ได้ค่าใหม่และจะไม่ reset poll state[^10]

### F.3 udev Netlink (สำหรับ hot-plug)

**[documented — udev rules]** ใช้สำหรับ:
- Array created/assembled: `ACTION=="add|change"`, `KERNEL=="md*"`, `SUBSYSTEM=="block"`
- Array removed: `ACTION=="remove"`
- Member device added/removed

จาก mdadm udev rules:[^6]
```
SUBSYSTEM!="block", GOTO="md_end"
ACTION!="add|change", GOTO="md_end"
KERNEL!="md*", GOTO="md_end"
ENV{DEVTYPE}=="partition", GOTO="md_ignore_state"
```

### F.4 Hybrid Design (แนะนำ)

```
┌─────────────────────────────────────────────────────────┐
│                   MdMonitor (Rust)                       │
│                                                          │
│  ┌─────────────────┐    ┌────────────────────────────┐  │
│  │  udev socket    │    │  sysfs poll thread         │  │
│  │  (netlink)      │    │                            │  │
│  │  - Array add    │    │  For each active array:    │  │
│  │  - Array remove │    │  - poll(array_state)       │  │
│  │  - Member add   │    │  - poll(sync_action)       │  │
│  │  - Member remove│    │  - poll(degraded)          │  │
│  └────────┬────────┘    │  - poll(member states)     │  │
│           │             └──────────────┬─────────────┘  │
│           ▼                            ▼                  │
│  ┌────────────────────────────────────────────────────┐  │
│  │              Event Channel (tokio::mpsc)            │  │
│  │  ArrayAppeared | ArrayDisappeared | StateChanged |  │  │
│  │  DegradedChanged | SyncStarted | SyncCompleted |   │  │
│  │  MemberFailed | MemberRecovered                    │  │
│  └────────────────────────────────────────────────────┘  │
│           │                                              │
│           ▼                                              │
│  ┌─────────────────────────────────────────────────┐    │
│  │        Periodic Reconciliation (every 30s)       │    │
│  │  Re-scan all arrays via sysfs full snapshot      │    │
│  │  Catch any missed events / validate state cache  │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### F.5 Polling Intervals ที่แนะนำ

| Metric | Interval | เหตุผล |
|--------|----------|--------|
| Full snapshot reconciliation | 30s | Catch missed events; sync_speed averaged over 30s anyway[^2] |
| sync_completed (during active sync) | 5s | UI update; อย่า poll เร็วกว่า sync_speed averaging window |
| sysfs poll() blocking | event-driven | No CPU cost; only wake on actual change |
| udev monitor | event-driven | Kernel push; hot-plug immediate |

***

## G. Rust Data Model และ Backend Trait

### G.1 Strongly-Typed Enums

```rust
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RaidLevel {
    Linear,
    Raid0,
    Raid1,
    Raid4,
    Raid5,
    Raid6,
    Raid10,
    Multipath,
    Faulty,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayState {
    Clear,
    Inactive,
    Readonly,
    ReadAuto,
    Clean,
    Active,
    WritePending,
    ActiveIdle,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncAction {
    Idle,
    Resync,
    Recover,
    Check,
    Repair,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsistencyPolicy {
    None,       // raid0, linear
    Resync,     // default full resync
    Bitmap,     // write-intent bitmap
    Journal,    // raid5 journal device
    Ppl,        // Partial Parity Log (raid5 only)
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemberState {
    pub faulty: bool,
    pub in_sync: bool,
    pub write_mostly: bool,
    pub blocked: bool,
    pub spare: bool,
    pub write_error: bool,
    pub want_replacement: bool,
    pub replacement: bool,
}

/// Parse comma-separated member state string
/// e.g. "in_sync,write_error" → MemberState { in_sync: true, write_error: true, ... }
impl MemberState {
    pub fn from_str(s: &str) -> Self {
        let mut state = Self::default();
        for flag in s.split(',') {
            match flag.trim() {
                "faulty"           => state.faulty = true,
                "in_sync"          => state.in_sync = true,
                "writemostly"      => state.write_mostly = true,
                "blocked"          => state.blocked = true,
                "spare"            => state.spare = true,
                "write_error"      => state.write_error = true,
                "want_replacement" => state.want_replacement = true,
                "replacement"      => state.replacement = true,
                _ => {}
            }
        }
        state
    }
}
```

### G.2 Core Data Structures

```rust
#[derive(Debug, Clone)]
pub struct MdArraySnapshot {
    /// Array name as kernel knows it, e.g. "md0", "md127", "md_ssd"
    pub name: String,
    /// sysfs path: /sys/block/mdX/md/
    pub sysfs_md_path: PathBuf,
    pub level: RaidLevel,
    pub array_state: ArrayState,
    pub metadata_version: String,
    /// UUID: only present in newer kernels (~5.x+)
    pub uuid: Option<String>,
    pub raid_disks: u32,
    pub chunk_size_bytes: Option<u64>,   // None for raid1
    pub component_size_sectors: u64,
    pub degraded: u32,
    pub sync_action: SyncAction,
    pub sync_progress: Option<SyncProgress>,  // None when idle
    pub mismatch_cnt: u64,
    pub consistency_policy: Option<ConsistencyPolicy>,  // None if attr missing
    pub reshape_position: Option<u64>,  // None = "none"
    pub members: Vec<MdMember>,
    pub bitmap: Option<BitmapInfo>,
    pub collected_at: Instant,  // CLOCK_MONOTONIC snapshot time
}

#[derive(Debug, Clone)]
pub struct SyncProgress {
    pub completed_sectors: u64,
    pub total_sectors: u64,
    /// KiB/s, averaged over last 30 seconds by kernel
    pub speed_kib_s: u64,
    pub sync_action: SyncAction,
}

impl SyncProgress {
    pub fn percent(&self) -> f64 {
        if self.total_sectors == 0 { return 100.0; }
        (self.completed_sectors as f64 / self.total_sectors as f64) * 100.0
    }

    pub fn bytes_remaining(&self) -> u64 {
        (self.total_sectors - self.completed_sectors) * 512
    }

    pub fn eta_seconds(&self) -> Option<u64> {
        if self.speed_kib_s == 0 { return None; }
        let remaining_kib = self.bytes_remaining() / 1024;
        Some(remaining_kib / self.speed_kib_s)
    }
}

#[derive(Debug, Clone)]
pub struct MdMember {
    /// kernel dev name, e.g. "sda1", "sdb", "nvme0n1p2"
    pub dev_name: String,
    /// /sys/block/mdX/md/dev-XXX/
    pub sysfs_dev_path: PathBuf,
    pub state: MemberState,
    /// None if spare or failed
    pub slot: Option<u32>,
    pub errors: u64,
    /// None = in_sync; "none" = in_sync; sector N = recovery checkpoint
    pub recovery_start: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct BitmapInfo {
    pub location: String,    // "none", "+128", "file:/path/to/file"
    pub metadata: String,    // "internal", "external"
    pub chunksize_bytes: Option<u64>,
}

/// Backend trait - เพื่อ inject fixture directory ใน tests
pub trait MdBackend: Send + Sync {
    /// Enumerate all active md array names (e.g. ["md0", "md127"])
    fn enumerate_arrays(&self) -> Result<Vec<String>, MdError>;
    
    /// Read a full snapshot for a given array
    fn read_array(&self, name: &str) -> Result<MdArraySnapshot, MdError>;
    
    /// Enumerate member device names for an array
    fn enumerate_members(&self, name: &str) -> Result<Vec<String>, MdError>;
    
    /// Read raw string value from sysfs attribute (for testing/debug)
    fn read_attr(&self, array: &str, attr: &str) -> Result<String, MdError>;
    
    /// Read raw string value for a member attribute
    fn read_member_attr(&self, array: &str, member: &str, attr: &str) 
        -> Result<String, MdError>;
}

#[derive(Debug)]
pub enum MdError {
    ArrayDisappeared,   // ENOENT during read — normal during remove
    PermissionDenied,   // EACCES/EPERM
    ParseError(String), // unexpected format
    Io(std::io::Error),
}
```

### G.3 Sysfs Native Backend

```rust
pub struct SysfsBackend {
    /// Root of sysfs — normally "/" but "/tmp/test-fixtures" in tests
    pub root: PathBuf,
}

impl SysfsBackend {
    pub fn new() -> Self {
        Self { root: PathBuf::from("/") }
    }
    
    fn md_path(&self, name: &str) -> PathBuf {
        self.root.join("sys/block").join(name).join("md")
    }
}

impl MdBackend for SysfsBackend {
    fn enumerate_arrays(&self) -> Result<Vec<String>, MdError> {
        let block_dir = self.root.join("sys/block");
        let mut arrays = Vec::new();
        for entry in std::fs::read_dir(&block_dir).map_err(MdError::Io)? {
            let entry = entry.map_err(MdError::Io)?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("md") { continue; }
            // Verify it is actually an md array by checking md/array_state exists
            let state_path = self.md_path(&name).join("array_state");
            if state_path.exists() {
                arrays.push(name);
            }
        }
        Ok(arrays)
    }

    fn read_attr(&self, array: &str, attr: &str) -> Result<String, MdError> {
        let path = self.md_path(array).join(attr);
        match std::fs::read_to_string(&path) {
            Ok(s) => Ok(s.trim_end_matches('\n').to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(MdError::ArrayDisappeared),
            Err(e) => Err(MdError::Io(e)),
        }
    }

    fn read_member_attr(&self, array: &str, member: &str, attr: &str) 
        -> Result<String, MdError> 
    {
        let path = self.md_path(array).join(member).join(attr);
        match std::fs::read_to_string(&path) {
            Ok(s) => Ok(s.trim_end_matches('\n').to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(MdError::ArrayDisappeared),
            Err(e) => Err(MdError::Io(e)),
        }
    }
    
    fn enumerate_members(&self, name: &str) -> Result<Vec<String>, MdError> {
        let md_dir = self.md_path(name);
        let mut members = Vec::new();
        for entry in std::fs::read_dir(&md_dir).map_err(MdError::Io)? {
            let entry = entry.map_err(MdError::Io)?;
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.starts_with("dev-") {
                members.push(fname);
            }
        }
        Ok(members)
    }

    fn read_array(&self, name: &str) -> Result<MdArraySnapshot, MdError> {
        // NOTE: read array_state FIRST — if it disappears, return ArrayDisappeared
        let array_state_str = self.read_attr(name, "array_state")?;
        let array_state = ArrayState::from_str(&array_state_str);

        // Early exit for non-active arrays
        if matches!(array_state, ArrayState::Clear | ArrayState::Inactive) {
            // Still return a partial snapshot — caller decides what to do
        }

        let level_str = self.read_attr(name, "level").unwrap_or_default();
        let raid_disks = self.read_attr(name, "raid_disks")
            .unwrap_or_default()
            .parse::<u32>()
            .unwrap_or(0);
        
        // ... (full implementation in code skeleton below)
        
        Ok(MdArraySnapshot {
            name: name.to_string(),
            sysfs_md_path: self.md_path(name),
            level: RaidLevel::from_str(&level_str),
            array_state,
            metadata_version: self.read_attr(name, "metadata_version").unwrap_or_default(),
            uuid: self.read_attr(name, "uuid").ok(),
            raid_disks,
            chunk_size_bytes: self.read_attr(name, "chunk_size").ok()
                .and_then(|s| s.parse::<u64>().ok()),
            component_size_sectors: self.read_attr(name, "component_size")
                .unwrap_or_default().parse().unwrap_or(0),
            degraded: self.read_attr(name, "degraded")
                .unwrap_or_default().parse().unwrap_or(0),
            sync_action: SyncAction::from_str(
                &self.read_attr(name, "sync_action").unwrap_or_default()
            ),
            sync_progress: self.read_sync_progress(name),
            mismatch_cnt: self.read_attr(name, "mismatch_cnt")
                .unwrap_or_default().parse().unwrap_or(0),
            consistency_policy: self.read_attr(name, "consistency_policy").ok()
                .map(|s| ConsistencyPolicy::from_str(&s)),
            reshape_position: self.read_attr(name, "reshape_position").ok()
                .and_then(|s| if s == "none" { None } else { s.parse::<u64>().ok() }),
            members: self.read_members(name).unwrap_or_default(),
            bitmap: self.read_bitmap_info(name).ok().flatten(),
            collected_at: Instant::now(),
        })
    }
}
```

***

## H. Rust Code Skeleton — sysfs Traversal

```rust
impl SysfsBackend {
    fn read_sync_progress(&self, name: &str) -> Option<SyncProgress> {
        let action_str = self.read_attr(name, "sync_action").ok()?;
        let action = SyncAction::from_str(&action_str);
        
        // ถ้า idle → ไม่มี progress data
        if matches!(action, SyncAction::Idle) { return None; }
        
        // sync_completed format: "N / M" หรือ "none"
        let sync_completed_str = self.read_attr(name, "sync_completed").ok()?;
        if sync_completed_str == "none" || sync_completed_str.is_empty() {
            return None;
        }
        
        // Parse "N / M"
        let parts: Vec<&str> = sync_completed_str.splitn(2, '/').collect();
        if parts.len() != 2 { return None; }
        
        let completed = parts.trim().parse::<u64>().ok()?;
        let total = parts[^1].trim().parse::<u64>().ok()?;
        
        // sync_speed: parse integer, strip optional " (local)" or " (system)" suffix
        let speed_str = self.read_attr(name, "sync_speed").ok()?;
        let speed = speed_str.split_whitespace().next()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        
        Some(SyncProgress {
            completed_sectors: completed,
            total_sectors: total,
            speed_kib_s: speed,
            sync_action: action,
        })
    }

    fn read_members(&self, name: &str) -> Result<Vec<MdMember>, MdError> {
        let member_names = self.enumerate_members(name)?;
        let mut members = Vec::new();
        
        for dev_entry in member_names {
            // "dev-sda1" → "sda1"
            let dev_name = dev_entry.trim_start_matches("dev-").to_string();
            
            let state_str = self.read_member_attr(name, &dev_entry, "state")
                .unwrap_or_default();
            
            let slot_str = self.read_member_attr(name, &dev_entry, "slot")
                .unwrap_or_default();
            let slot = if slot_str == "none" { None } 
                       else { slot_str.parse::<u32>().ok() };
            
            let errors = self.read_member_attr(name, &dev_entry, "errors")
                .unwrap_or_default().parse::<u64>().unwrap_or(0);
            
            let recovery_start = {
                let rs = self.read_member_attr(name, &dev_entry, "recovery_start")
                    .unwrap_or_default();
                if rs == "none" { None } else { rs.parse::<u64>().ok() }
            };
            
            members.push(MdMember {
                dev_name,
                sysfs_dev_path: self.md_path(name).join(&dev_entry),
                state: MemberState::from_str(&state_str),
                slot,
                errors,
                recovery_start,
            });
        }
        
        // Sort by slot for consistent ordering
        members.sort_by_key(|m| m.slot.unwrap_or(u32::MAX));
        Ok(members)
    }
    
    fn read_bitmap_info(&self, name: &str) -> Result<Option<BitmapInfo>, MdError> {
        // bitmap attributes only exist when bitmap is active
        let location = self.read_attr(name, "bitmap/location");
        let metadata = self.read_attr(name, "bitmap/metadata");
        
        match (location, metadata) {
            (Ok(loc), Ok(meta)) if loc != "none" => {
                let chunksize = self.read_attr(name, "bitmap/chunksize").ok()
                    .and_then(|s| s.parse::<u64>().ok());
                Ok(Some(BitmapInfo {
                    location: loc,
                    metadata: meta,
                    chunksize_bytes: chunksize,
                }))
            }
            _ => Ok(None),
        }
    }
}

// --- Async poll wrapper using tokio::task::spawn_blocking ---
pub async fn watch_array_state_change(
    backend: Arc<SysfsBackend>,
    array_name: String,
) -> Result<(), MdError> {
    use std::os::unix::io::AsRawFd;
    
    tokio::task::spawn_blocking(move || {
        // open array_state for polling
        let path = backend.md_path(&array_name).join("array_state");
        
        loop {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .open(&path)
                .map_err(MdError::Io)?;
            
            // Initial read to consume current value
            let _ = std::io::Read::read_to_string(&mut file.try_clone().unwrap(), &mut String::new());
            
            // poll() waiting for POLLERR|POLLPRI which sysfs uses for attribute changes
            let mut pollfd = libc::pollfd {
                fd: file.as_raw_fd(),
                events: libc::POLLERR | libc::POLLPRI,
                revents: 0,
            };
            
            // Block until change (timeout 60s max — guard against silent failures)
            unsafe { libc::poll(&mut pollfd, 1, 60_000); }
            
            // ⚠️ MUST close and reopen — cannot simply seek+read after poll on sysfs
            drop(file);
            
            // Re-open and read new value
            // ... notify via channel
        }
    }).await.map_err(|_| MdError::ArrayDisappeared)?
}
```

***

## I. State-Transition and Race-Condition Handling

### I.1 Known Race Conditions

| Scenario | อาการ | วิธีจัดการ |
|----------|-------|------------|
| Array removed ระหว่าง read | `ENOENT` หรือ `EIO` บน sysfs file | Return `MdError::ArrayDisappeared`; emit `ArrayDisappeared` event; mark all members as gone |
| `sync_action` เปลี่ยนเป็น `idle` ระหว่างที่กำลังอ่าน `sync_completed` | ได้ `sync_completed` ของ completed sync | ตรวจ sync_action **หลัง** อ่าน sync_completed ด้วย; ถ้า idle → ตีความว่า 100% |
| Member hot-remove ระหว่าง enumerate `dev-*` | `readdir()` returns entry แต่ file ไม่ exist แล้ว | TOCTOU ปกติ — ใช้ `ENOENT` เป็น "member vanished" |
| `array_state` เปลี่ยนจาก `active` → `inactive` ระหว่าง snapshot | ข้อมูลบางตัวอาจ stale | อ่าน `array_state` ก่อนและหลัง snapshot; ถ้า state เปลี่ยน → discard snapshot |
| External metadata array (IMSM/DDF) blocked menunggu mdmon | array_state = `write-pending` นานผิดปกติ | ตรวจ `metadata_version` ว่าขึ้นต้นด้วย `external:` → special handling |
| reshape กำลังเกิดขึ้น | `raid_disks` อ่านได้ค่าใหม่(old) format | parse `"N (M)"` format ของ reshape_position |

### I.2 External Metadata Arrays

**[mdmon documentation — documented]** เมื่อ `metadata_version` ขึ้นต้นด้วย `external:` (เช่น `external:imsm` หรือ `external:/dev/md/imsm0`):

1. `mdmon` process กำลัง monitor array และจะ suspend IO เพื่อรอ metadata update[^5]
2. monitoring software **ห้าม** พยายาม write ไปยัง sysfs attributes ใด ๆ ของ array เหล่านี้
3. `array_state` อาจค้างที่ `inactive` หรือ `write-pending` นานกว่าปกติ[^6]
4. Container device (เช่น `/dev/md/imsm0`) จะ listed ใน `/proc/mdstat` ด้วย `external:imsm` — ไม่ใช่ array ที่ใช้งานได้โดยตรง

**[ต้องยืนยัน]**: พฤติกรรมที่แน่นอนเมื่อ mdmon ไม่ได้ running กับ external metadata array — สาเหตุที่ IO blocked ต้องทดสอบกับ hardware IMSM

***

## J. Test Matrix

### J.1 Fixture-Based Unit Tests (ไม่ต้องมี Hardware)

**[ข้อเสนอแนะผู้วิจัย]** สร้าง fixture directory tree ที่ simulate `/sys/block/mdX/md/`:

```rust
// test/fixtures/layout:
// sysfs_root/sys/block/
//   md0/md/
//     level          → "raid5"
//     array_state    → "active"
//     raid_disks     → "3"
//     degraded       → "0"
//     sync_action    → "idle"
//     consistency_policy → "bitmap"
//     chunk_size     → "524288"
//     component_size → "1953523055"
//     mismatch_cnt   → "0"
//     metadata_version → "1.2"
//     dev-sda1/
//       state        → "in_sync"
//       slot         → "0"
//       errors       → "0"
//       recovery_start → "none"
//     dev-sdb1/
//       state        → "in_sync"
//       slot         → "1"
//       ...

fn make_test_backend(fixture_dir: &str) -> SysfsBackend {
    SysfsBackend { root: PathBuf::from(fixture_dir) }
}
```

### J.2 Test Cases Matrix

| Test Scenario | Fixture | Expected Result |
|--------------|---------|----------------|
| Healthy RAID5, 3 disks | `degraded=0`, `sync_action=idle`, all members `in_sync` | `degraded=0`, no sync progress |
| Degraded RAID1, 1 disk failed | `degraded=1`, one member `faulty`, no spare | alert: degraded, no rebuild |
| Rebuilding RAID5 (recovering spare) | `sync_action=recover`, `sync_completed="123456 / 9876543"`, `sync_speed=50000` | progress=1.25%, ETA calculated |
| Check running | `sync_action=check`, `sync_completed="5000 / 10000"` | progress=50%, no alert |
| Array undergoing reshape | `reshape_position="1024"`, `raid_disks="4 (3)"` | parse both old/new raid_disks |
| Multiple arrays (md0, md127, md_ssd) | 3 separate fixture subdirs | enumerate returns all 3 |
| Array disappears mid-read | `array_state` file deleted after first read | `MdError::ArrayDisappeared` propagated |
| Member with bad blocks | `bad_blocks="1024 16\n2048 8"` | bad block list parsed |
| External metadata array (IMSM) | `metadata_version=external:imsm` | flagged as external, special handling |
| PPL array | `consistency_policy=ppl`, member has `ppl_sector`, `ppl_size` | parsed correctly |
| sync_completed = "none" | idle array | `sync_progress = None` |
| Malformed sync_completed | `"abc / def"` | `ParseError`, log and continue |
| Empty raid_disks | `""` (assembling) | `raid_disks = 0`, partial state |

### J.3 Integration Tests ด้วย Loop Devices

```bash
# สร้าง test array ด้วย loop devices (ต้องการ root)
setup_test_raid5() {
    # Create 3 loop devices
    for i in 0 1 2; do
        dd if=/dev/zero of=/tmp/raid-disk$i.img bs=1M count=100
        losetup /dev/loop$i /tmp/raid-disk$i.img
    done
    # Create RAID5 (mdadm ใช้เฉพาะ test setup เท่านั้น)
    mdadm --create /dev/md/test-raid5 --level=5 --raid-devices=3 \
        /dev/loop0 /dev/loop1 /dev/loop2
}
# integration test: อ่าน sysfs ด้วย Rust code และ validate ด้วย mdadm --detail
```

### J.4 CI Requirements

| Requirement | ทางแก้ |
|-------------|--------|
| Root privileges สำหรับ loop devices + md | Docker container `--privileged` หรือ GitHub Actions self-hosted runner |
| `mdadm` ใน test container | ติดตั้งใน Dockerfile (test-only dep) |
| Kernel md module loaded | ต้องการ host kernel มี `md_mod` loaded |
| Alternative: ไม่ต้องการ hardware | Unit tests ด้วย fixture tree ทั้งหมด (ไม่ต้องการ root) |

***

## K. Migration Plan จาก `/proc/mdstat` Parser

### K.1 Phase 1 — Parallel Running (MVP)

- Implement `MdBackend` trait และ `SysfsBackend`
- Map ทุก field จาก `/proc/mdstat` parser ไปยัง sysfs equivalent
- Run ทั้งสองพร้อมกันและ compare values ใน debug mode
- `/proc/mdstat` parser ยังเป็น primary; sysfs เป็น validation

### K.2 Phase 2 — Sysfs Primary

- เปลี่ยน sysfs เป็น primary data source
- ใช้ `/proc/mdstat` เฉพาะ fields ที่ยังไม่มีใน sysfs:
  - `read_ahead` (ไม่สำคัญ — ไม่ต้องใช้)
  - Human-readable personality list (ใช้ `level` จาก sysfs แทน)
- ตัด `/proc/mdstat` dependency ออก

### K.3 `/proc/mdstat` Field Mapping

| /proc/mdstat field | sysfs equivalent | หมายเหตุ |
|-------------------|-----------------|---------|
| `md0 : active raid5` | `md/array_state` + `md/level` | ใช้ได้ |
| `sda1 sdb1[^1] sdc1[^2]` | `md/dev-*/slot` | ครบกว่า — รวม state flags |
| `[3/3] [UUU]` | `md/degraded` + member `state` | ต้อง reconstruct bitmap จาก members |
| `blocks` count | `md/component_size` × `md/raid_disks` | คำนวณได้ |
| `= 12345/567890` progress | `md/sync_completed` | ดีกว่า — separate N/M |
| `speed=12345K/sec` | `md/sync_speed` | เหมือนกัน |
| `finish=1.5min` | คำนวณจาก sync_speed | ต้องคำนวณเอง |
| resync=DELAYED | ไม่มีค่าเทียบตรงๆ | sysfs: `sync_action` ไม่แสดง DELAYED |

> **⚠️ ข้อสังเกต [ต้องยืนยัน]**: `resync=DELAYED` ใน `/proc/mdstat` หมายถึง resync ถูก queue ไว้รอ — sysfs `sync_action` อาจยังเป็น `idle` ในกรณีนี้ ต้องทดสอบว่า `array_state` หรือ attribute อื่นระบุ delayed state ได้หรือไม่

***

## L. Known Limitations โดยเฉพาะ External Metadata/Container RAID

### L.1 External Metadata (IMSM / DDF)

**[mdmon documentation]** Arrays ที่ใช้ `external:` metadata มีข้อจำกัดดังนี้:[^5]

1. **mdmon ต้องรันอยู่ตลอดเวลา** — monitoring software ต้องไม่ฆ่า หรือ block mdmon process ไม่เช่นนั้น IO ของ array จะถูก block ตลอด
2. **Container device ≠ member array**: Container (เช่น `/dev/md/imsm0`) คือ pseudo-array ที่ hold references ถึง disks ทั้งหมด — member arrays เป็น sub-arrays แยกกัน
3. **Disk removal ทำได้เฉพาะที่ container level** — monitoring ต้องรับรู้ hierarchy นี้
4. **sysfs attributes บาง attribute ของ container** อาจ behave ต่างจาก native arrays
5. **ห้าม write ไปยัง `array_state` หรือ `sync_action`** ของ external metadata arrays — mdmon เป็น sole authority สำหรับ state transitions

### L.2 Nested MD RAID (md-of-md)

- member ของ outer array อาจเป็น `/dev/md/inner` — อ่าน state จาก sysfs ได้ตามปกติ
- `dev-mdX/block` symlink จะชี้ไปยัง `/sys/block/mdX` ของ inner array
- Dependency graph ต้องสร้างจาก sysfs symlinks — ต้องระวัง circular dependency

### L.3 Partition-Based Members

- member อาจเป็น partition เช่น `sda1`, `nvme0n1p2` — ชื่อใน `dev-XXX` จะเป็น `dev-sda1`, `dev-nvme0n1p2`
- Block device symlink ใน `dev-XXX/block` ชี้ไปยัง parent block device ได้ถูกต้อง[^11]
- sysfs hierarchy สำหรับ partitions: `/sys/block/sda/sda1/` — traverse parent เพื่อหาข้อมูล disk

### L.4 Named Arrays (ไม่ใช่ mdN)

- Arrays สามารถมีชื่อ เช่น `/dev/md/data` หรือ `/dev/md_d0`
- `/sys/block/` จะมี entry ตามชื่อ device node จริง (ซึ่งอาจเป็น `md127` หรือ `md/data`)
- udev สร้าง symlink ใน `/dev/md/` และ `/dev/disk/by-id/md-name-*`[^6]
- **ห้ามสมมติว่าทุก array ชื่อขึ้นต้นด้วย `md0`, `md1`** — enumerate จาก `/sys/block/md*` เสมอ

### L.5 คุณสมบัติที่ขึ้นกับ Kernel Version

| Feature | Minimum Kernel | Status |
|---------|--------------|--------|
| sysfs MD interface พื้นฐาน | 2.6.12 | Stable |
| `resync_start = none` | 2.6.30-rc1 | Stable[^2] |
| External metadata support | 2.6.27 | Stable[^5] |
| Bad blocks (BBL) sysfs | ~3.5 | Should verify |
| `consistency_policy = journal` | ~4.10–4.15 | **ต้องยืนยัน** |
| `consistency_policy = ppl` | ~4.15–4.19 | Documented in 5.8 docs[^8] |
| `uuid` sysfs attribute | ~5.x | ปรากฏใน 6.x docs[^2] |
| `bitmap_type` / `llbitmap` | ~6.x (PATCH 2025)[^12] | **Very new — ไม่ stable** |
| `logical_block_size` sysfs | Recent | Marked experimental[^2] |

***

## M. Primary-Source Links

| ชื่อ | URL | เนื้อหา | ความน่าเชื่อถือ |
|------|-----|---------|---------------|
| Linux Kernel md Documentation (latest) | https://docs.kernel.org/admin-guide/md.html | **Primary** — sysfs ABI official doc[^2] | ✅ Authoritative |
| md(4) man page | https://linux.die.net/man/4/md | ioctl API, superblock formats[^13] | ✅ Authoritative |
| mdmon(8) man page | https://manpages.ubuntu.com/manpages/noble/man8/mdmon.8.html | External metadata monitoring[^5] | ✅ Authoritative |
| Partial Parity Log docs | https://www.kernel.org/doc/html/v5.8/driver-api/md/raid5-ppl.html | PPL feature, consistency_policy[^8] | ✅ Authoritative |
| inotify sysfs limitation | stackoverflow.com/questions/13697615 | inotify ไม่รองรับ pseudo-fs[^4] | ✅ Well-established |
| sysfs poll/select mechanism | mail-archive kernelnewbies | Poll semantics, close-and-reopen rule[^10] | ✅ From kernel developer list |
| Prometheus/procfs sysfs PR | github.com/prometheus/procfs/pull/509 | Real-world sysfs parsing experience[^1] | ✅ Production reference |
| mdadm udev rules | github kernel.googlesource.com udev-md-raid-arrays.rules | udev events for md arrays[^6] | ✅ From mdadm upstream |
| Bugzilla kernel.org 205929 | bugzilla.kernel.org/show_bug.cgi?id=205929 | Real sysfs output from RAID6, consistency_policy example[^9] | ✅ Real system data |
| mdadm/sysfs.c source | github.com/md-raid-utilities/mdadm/blob/main/sysfs.c | How mdadm reads/writes sysfs[^14] | ✅ Reference implementation |
| SG_IO sysfs poll detailed | kernel.org LDD excerpt | `sysfs_notify()` mechanism[^15] | ✅ Official |

### M.1 Rust Crates ที่แนะนำ

| Crate | Version | ใช้สำหรับ | หมายเหตุ |
|-------|---------|----------|---------|
| `std::fs` | — | sysfs reads (blocking) | Primary — ไม่ต้องการ crate พิเศษ |
| `libc` | 0.2+ | `poll()` syscall for sysfs watch | สำหรับ `POLLERR\|POLLPRI` |
| `udev` | 0.8+ | Hot-plug monitoring via netlink[^16] | Optional แต่แนะนำ |
| `tokio` | 1.x | `spawn_blocking` สำหรับ poll thread | **ใช้ `spawn_blocking`** ไม่ใช่ `tokio::fs` |
| `nix` | 0.29+ | `poll()` type-safe wrapper | Alternative to raw libc |

> **[ข้อเสนอแนะผู้วิจัย]** ใช้ `std::fs` (blocking) + `tokio::task::spawn_blocking` สำหรับ sysfs reads ทั้งหมด เช่นเดียวกับที่แนะนำสำหรับ SCSI monitoring — sysfs เป็น virtual filesystem ที่ไม่มี real I/O latency

---

## References

1. [Implement mdraid sysfs parsing by dswarbrick · Pull Request #509 · prometheus/procfs](https://github.com/prometheus/procfs/pull/509) - Modernised method of fetching mdraid statistics via machine-readable sysfs entries, instead of parsi...

2. [RAID arrays](https://docs.kernel.org/admin-guide/md.html)

3. [Proposal: collect mdraid metrics from sysfs instead of parsing /proc/mdstat · Issue #1085 · prometheus/node_exporter](https://github.com/prometheus/node_exporter/issues/1085) - Proposal I just discovered the that node_md_blocks_synced metric, which is currently parsed from /pr...

4. [inotify_add_watch to /proc folder](https://stackoverflow.com/questions/13697615/inotify-add-watch-to-proc-folder) - I am trying to put "inotify_add_watch" for process. My intent of doing this is to get notification w...

5. [mdmon - monitor MD external metadata arrays](https://manpages.ubuntu.com/manpages/noble/man8/mdmon.8.html)

6. [udev-md-raid-arrays.rules - pub/scm/utils/mdadm/mdadm](https://kernel.googlesource.com/pub/scm/utils/mdadm/mdadm/+/cluster/udev-md-raid-arrays.rules)

7. [Linux MD - TomsWeb](https://www.stewarts.org.uk/posts/linuxmd/) - /proc/mdstat information # Linux has a software RAID subsystem and it is called md. It is generally ...

8. [Partial Parity Log - The Linux Kernel Archives](https://www.kernel.org/doc/html/v5.8/driver-api/md/raid5-ppl.html)

9. [205929](https://bugzilla.kernel.org/show_bug.cgi?id=205929)

10. [Re: sysfs_notify & poll](https://www.mail-archive.com/kernelnewbies@nl.linux.org/msg06352.html)

11. [linux-kernel/Documentation/md.txt at master · tinganho/linux-kernel](https://github.com/tinganho/linux-kernel/blob/master/Documentation/md.txt) - imx6sl linux kernel source. Contribute to tinganho/linux-kernel development by creating an account o...

12. [[PATCH RFC v2 04/14] md: add a new sysfs api bitmap_version](https://patchew.org/linux/20250328060853.4124527-1-yukuai1@huaweicloud.com/20250328060853.4124527-5-yukuai1@huaweicloud.com/)

13. [md(4) - Linux man page](https://linux.die.net/man/4/md) - The md driver provides virtual devices that are created from one or more independent underlying devi...

14. [mdadm/sysfs.c at main · md-raid-utilities/mdadm](https://github.com/md-raid-utilities/mdadm/blob/main/sysfs.c) - Manager of Linux Software RAID implemented through Multiple Devices driver. - md-raid-utilities/mdad...

15. [Allowing sysfs attribute files to be pollable - Linux Device ...](https://www.oreilly.com/library/view/linux-device-drivers/9781785280009/8ba619c0-1bb5-4ea1-9926-7fa73b6c3591.xhtml) - Allowing sysfs attribute files to be pollable Here we will see how not to make CPU wasting polling t...

16. [udev](https://lib.rs/crates/udev) - libudev bindings for Rust

