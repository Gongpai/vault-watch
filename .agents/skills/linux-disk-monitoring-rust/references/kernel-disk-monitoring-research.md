# Linux Disk Monitoring ด้วย Rust: Kernel Interfaces, Device Discovery และ Throughput Measurement

***

## A. Executive Summary

รายงานฉบับนี้ครอบคลุมการสร้าง Linux disk-monitoring library ใน Rust โดยอ่าน kernel interfaces โดยตรงจาก sysfs (`/sys/block`, `/sys/class/block`) และ procfs (`/proc/diskstats`) โดยไม่เรียกใช้ `iostat`, `lsblk`, `sysstat` หรือ external CLI ใด ๆ

**ข้อสรุปหลักจากเอกสาร (kernel-documented):**
- `/proc/diskstats` มี 14 fields (kernel 2.5.69+), 18 fields (kernel 4.18+), 20 fields (kernel 5.5+)[^1][^2]
- `/sys/block/<dev>/stat` มี 17 fields เป็น atomic consistent snapshot สำหรับ device เดียว[^3]
- Sector ในบริบท kernel I/O statistics **เสมอ = 512 bytes** โดยไม่ขึ้นกับ physical sector size ของ hardware[^4][^3]
- `queue/rotational` (kernel 2.6.29+) บ่งบอก HDD (1) หรือ SSD/NVMe (0) แต่อาจไม่แม่นยำใน virtual environment[^5][^6]
- `/sys/block/<disk>/slaves/` และ `/sys/block/<disk>/holders/` ใช้แยก stacked devices (dm, md) ออกจาก physical disks[^7]

**ข้อเสนอแนะเชิงสถาปัตยกรรม (ผู้วิจัย):**
- ใช้ `std::fs` (blocking) แทน `tokio::fs` สำหรับการอ่าน sysfs/procfs ขนาดเล็ก เพราะ virtual filesystem เหล่านี้ไม่มี real I/O latency
- ออกแบบ `BlockDeviceSource` trait เพื่อ mock path prefix สำหรับ unit testing
- ใช้ `std::time::Instant` (CLOCK_MONOTONIC) เป็น timing reference ระหว่าง samples

***

## B. Recommended Architecture

```
DiskMonitor
├── DeviceDiscovery (sysfs enumerator)
│   ├── SysFsEnumerator       ← อ่าน /sys/block/* โดยตรง (ไม่ต้องการ libudev)
│   └── UdevEnumerator        ← optional: ใช้ udev crate สำหรับ hot-plug events
├── DeviceInfo (metadata)
│   ├── sysfs_path: /sys/block/<dev>
│   ├── model / vendor / serial / firmware
│   ├── rotational / logical_block_size / physical_block_size
│   └── device_type: Physical | Partition | Loop | Dm | Md | Ram | NvmeNs | Virtual
├── ThroughputSampler
│   ├── DataSource trait { read_stat(&self, dev: &str) -> RawStat }
│   ├── ProcDiskstatsSource    ← /proc/diskstats (batch: ดีเมื่อ track หลาย device)
│   ├── SysBlockStatSource     ← /sys/block/<dev>/stat (single device, consistent snapshot)
│   └── MockSource             ← สำหรับ testing
└── MetricsCalculator
    ├── DiskSample { raw: RawStat, timestamp: Instant }
    └── compute_delta(s1: &DiskSample, s2: &DiskSample) -> DiskMetrics
```

**[สถาปัตยกรรม — ข้อเสนอแนะผู้วิจัย]** การแยก `DataSource` trait ออกมาทำให้ทดสอบได้โดยไม่ต้องมี disk จริง และรองรับการเพิ่ม data source ใหม่ในอนาคตได้

***

## C. Device Discovery Algorithm (ทีละขั้น)

### ขั้นที่ 1: อ่าน /sys/class/block หรือ /sys/block

```
enumerate /sys/class/block/*   (preferred — symlinks ไปยัง /sys/devices/...)
หรือ      /sys/block/*         (legacy compat, available kernel 2.6+)
```

`/sys/class/block` เป็น canonical interface ที่แนะนำโดย kernel documentation ส่วน `/sys/block` ยังคงอยู่เพื่อ backward compatibility แต่ตั้งแต่ kernel 2.6.26 เป็นต้นมา entries ใน `/sys/block` กลายเป็น symlinks ไปยัง `/sys/devices/`[^8][^9]

### ขั้นที่ 2: กรอง virtual/stacked devices

อ่าน `uevent` file ใน sysfs ของแต่ละ device — field `DEVTYPE` บ่งบอก `disk` หรือ `partition`:[^10][^11]

```
/sys/block/<dev>/uevent  →  DEVTYPE=disk  (whole disk)
                             DEVTYPE=partition  (ข้ามไป)
```

จากนั้นใช้ **name-based filter** ตาม kernel naming convention (stable pattern):

| Pattern | Device Type | Action |
|---------|-------------|--------|
| `loop*` | Loop device | ข้าม |
| `ram*` | RAM disk | ข้าม |
| `dm-*` | Device mapper (LVM, dm-crypt) | ข้ามในขั้นต้น (ดูข้อ 4) |
| `md*` | MD RAID | ข้ามในขั้นต้น |
| `nbd*` | Network block device | ข้าม |
| `zram*` | Compressed RAM block device | ข้าม |
| `nvme[0-9]+c[0-9]+n[0-9]+` | NVMe multipath hidden path | ข้าม[^12] |
| `sd*`, `hd*`, `nvme[0-9]+n[0-9]+`, `vd*` | Physical (candidate) | ดำเนินการต่อ |

### ขั้นที่ 3: ตรวจสอบ slaves/holders เพื่อแยก stacked devices

```
/sys/block/<dev>/slaves/   → ถ้า non-empty = device นี้ถูกสร้างจาก device อื่น (dm, md)
/sys/block/<dev>/holders/  → ถ้า non-empty = มี device อื่นใช้ device นี้อยู่
```

ตัวอย่างจาก kernel patch documentation:[^7]
```
/sys/block/dm-0/slaves/sda → แสดงว่า dm-0 อยู่บน sda
/sys/block/sda/holders/dm-0 → แสดงว่า sda ถูกใช้โดย dm-0
```

**[ข้อเสนอแนะผู้วิจัย]** สำหรับ use case monitoring throughput: ควร **include** physical disks ที่มี holders (เพราะนั่นคือ physical disk จริงที่อยู่ข้างล่าง) และ **exclude** dm-/md- devices เว้นแต่ผู้ใช้ต้องการ monitor logical volume โดยเฉพาะ

### ขั้นที่ 4: ตรวจสอบ rotational flag

```
/sys/block/<dev>/queue/rotational
  1 = HDD (rotational)
  0 = SSD, NVMe, flash
```

Flag นี้ set by kernel based on hardware response (SCSI VPD BDC inquiry) ตั้งแต่ kernel 2.6.29 แต่มีข้อจำกัด: บาง virtual driver (virtio-blk), USB storage ที่ไม่มี VPD, และ hardware RAID controller อาจรายงานค่าผิด ค่านี้อยู่ใน stable ABI[^13][^14][^6][^15][^5]

### ขั้นที่ 5: อ่าน device metadata

**SCSI/SATA/SAS devices (sd*):**
```
/sys/block/<dev>/device/model      ← device model string
/sys/block/<dev>/device/vendor     ← vendor string  
/sys/block/<dev>/device/rev        ← firmware revision (บางครั้งเรียก firmware_rev)
```
หมายเหตุ: บาง driver ไม่ populate ทุก field[^16][^17]

**NVMe devices:**
NVMe metadata อยู่ที่ controller level ใน `/sys/class/nvme/nvmeN/`:[^18][^19]
```
/sys/class/nvme/nvme<N>/model
/sys/class/nvme/nvme<N>/serial
/sys/class/nvme/nvme<N>/firmware_rev
/sys/class/nvme/nvme<N>/state        (ควรเป็น "live")
```

การ map จาก `nvme0n1` (namespace) ไปยัง controller `nvme0` ทำได้โดย resolve symlink:
```
/sys/block/nvme0n1 → ../devices/pci.../nvme/nvme0/nvme0n1
```
จึง traverse ขึ้นไป 2 ระดับ (`nvme0n1` → `nvme0`) เพื่อถึง controller directory[^19]

**Sector sizes:**
```
/sys/block/<dev>/queue/logical_block_size    ← smallest addressable unit (มักเป็น 512)
/sys/block/<dev>/queue/physical_block_size   ← smallest atomic write unit (512 หรือ 4096)
/sys/block/<dev>/queue/hw_sector_size        ← hardware sector size
```
ทั้งหมดนี้อยู่ใน stable ABI ตาม kernel documentation[^15]

**[หมายเหตุสำคัญ — kernel-documented]** ขนาด sector ใน diskstats counters **ไม่ใช่** `logical_block_size` แต่เป็น **hardcoded 512 bytes** เสมอ ตามที่ kernel กำหนดใน `linux/blkdev.h` (`SECTOR_SIZE = 1 << 9`) ดังนั้นการแปลง sectors → bytes ต้องคูณด้วย 512 เสมอ ไม่ว่า hardware sector size จะเป็นเท่าใด[^4]

### ขั้นที่ 6: Bus/type detection

ใช้ sysfs symlink path หรือ udev properties เพื่อระบุ bus type:
- NVMe: device name pattern `nvme*` หรือ sysfs path มี `/nvme/`
- SATA/ATA: sysfs path มี `/ata[0-9]+/`
- SAS/SCSI: sysfs path มี `/scsi/` หรือ udev property `ID_BUS=scsi`
- udev property `ID_BUS` (ถ้าใช้ udev crate): `ata`, `scsi`, `usb`, `nvme`[^20]

***

## D. Kernel Data Sources: Path/API, Fields, Units, Stability, Permissions

### D.1 Statistics Sources

| Path | Fields | Units | Kernel Version | Stability | Permission |
|------|--------|-------|----------------|-----------|------------|
| `/proc/diskstats` | 14 fields (F1-F14) | mixed | 2.5.69+ | Stable ABI[^2][^21] | r--r--r-- |
| `/proc/diskstats` | +4 fields (F15-F18, discard) | ms | 4.18+ | Stable[^1] | r--r--r-- |
| `/proc/diskstats` | +2 fields (F19-F20, flush) | ms | 5.5+ | Stable[^1] | r--r--r-- |
| `/sys/block/<dev>/stat` | 17 fields (consistent snapshot) | mixed | 2.6+ | Stable[^15][^22] | r--r--r-- |

### D.2 Fields ของ /proc/diskstats และ /sys/block/<dev>/stat

| Field# | ชื่อ | หน่วย | ประเภท | หมายเหตุ |
|--------|------|-------|--------|---------|
| 1 | reads_completed | requests | unsigned long | I/Os ที่ complete สำเร็จ[^23] |
| 2 | reads_merged | requests | unsigned long | Adjacent reads ที่ merge กัน[^23] |
| 3 | sectors_read | sectors | unsigned long | **sectors = 512 bytes เสมอ**[^3][^4] |
| 4 | time_reading | milliseconds | unsigned int | Wall time ที่ใช้อ่าน[^23] |
| 5 | writes_completed | requests | unsigned long | |
| 6 | writes_merged | requests | unsigned long | |
| 7 | sectors_written | sectors | unsigned long | **sectors = 512 bytes เสมอ** |
| 8 | time_writing | milliseconds | unsigned int | |
| 9 | ios_in_progress | requests | unsigned int | **ไม่ monotonic** รีเซ็ตเป็น 0 เมื่อ I/Os complete[^23] |
| 10 | io_ticks | milliseconds | unsigned int | เวลาที่ device active (ใช้คำนวณ utilization)[^3][^23] |
| 11 | time_in_queue | milliseconds | unsigned int | Weighted time = ∑(in_progress × Δt)[^3] |
| 12 | discards_completed | requests | unsigned long | kernel 4.18+[^1] |
| 13 | discards_merged | requests | unsigned long | kernel 4.18+ |
| 14 | sectors_discarded | sectors | unsigned long | kernel 4.18+ |
| 15 | time_discarding | milliseconds | unsigned int | kernel 4.18+ |
| 16 | flush_requests | requests | unsigned int | kernel 5.5+, ไม่นับ partition[^1] |
| 17 | time_flushing | milliseconds | unsigned int | kernel 5.5+ |

> ใน `/proc/diskstats` มี 3 fields นำหน้า (major, minor, name) ก่อน field 1 ทำให้ column จริงคือ column 4–20[^1]

### D.3 Device Metadata Sources (sysfs)

| Path | ข้อมูล | Stability | Permission | หมายเหตุ |
|------|--------|-----------|------------|---------|
| `/sys/block/<dev>/queue/rotational` | 0=SSD, 1=HDD | Stable[^15] | rw-r--r-- | kernel 2.6.29+ |
| `/sys/block/<dev>/queue/logical_block_size` | bytes | Stable[^15] | r--r--r-- | kernel 2.6.31+ |
| `/sys/block/<dev>/queue/physical_block_size` | bytes | Stable[^15] | r--r--r-- | kernel 2.6.31+ |
| `/sys/block/<dev>/queue/hw_sector_size` | bytes | Stable[^15] | r--r--r-- | |
| `/sys/block/<dev>/device/model` | string | Testing ABI[^24] | r--r--r-- | SCSI/SATA เท่านั้น |
| `/sys/block/<dev>/device/vendor` | string | Testing ABI[^24] | r--r--r-- | SCSI/SATA |
| `/sys/block/<dev>/device/rev` | string | Testing ABI | r--r--r-- | firmware revision |
| `/sys/class/nvme/nvmeN/model` | string | Testing ABI[^25] | r--r--r-- | NVMe controller |
| `/sys/class/nvme/nvmeN/serial` | string | Testing ABI | r--r--r-- | NVMe |
| `/sys/class/nvme/nvmeN/firmware_rev` | string | Testing ABI | r--r--r-- | NVMe |
| `/sys/block/<dev>/slaves/` | symlinks | Testing ABI[^7] | r-xr-xr-x | empty = physical |
| `/sys/block/<dev>/holders/` | symlinks | Testing ABI[^7] | r-xr-xr-x | ใครอยู่บน device นี้ |
| `/sys/block/<dev>/hidden` | 0/1 | Stable[^15] | r--r--r-- | kernel 6.2+, NVMe multipath path |
| `/sys/block/<dev>/uevent` | KEY=VALUE | N/A | r--r--r-- | DEVTYPE, DEVNAME |

> **[kernel-documented]** `device/model`, `device/vendor` อยู่ใน **Testing ABI** (อาจเปลี่ยนได้) ไม่ใช่ stable ABI ส่วน `/proc/diskstats` fields อยู่ใน stable ABI[^2][^1]

***

## E. สูตรคำนวณ Metrics พร้อมตัวอย่าง

### E.1 ตัวแปรที่ใช้

กำหนด sample ที่เวลา T₁ และ T₂:
- \(\Delta t\) = elapsed time in seconds = `(T2 - T1).as_secs_f64()`  
- \(\Delta \text{sectors\_read}\) = `s2.sectors_read - s1.sectors_read`
- \(\Delta \text{sectors\_written}\) = `s2.sectors_written - s1.sectors_written`
- \(\Delta \text{reads\_completed}\) = `s2.reads_completed - s1.reads_completed`
- \(\Delta \text{writes\_completed}\) = `s2.writes_completed - s1.writes_completed`
- \(\Delta \text{io\_ticks}\) = `s2.io_ticks - s1.io_ticks` (in ms)
- \(\Delta \text{time\_reading}\) = `s2.time_reading - s1.time_reading` (in ms)
- \(\Delta \text{time\_writing}\) = `s2.time_writing - s1.time_writing` (in ms)

### E.2 สูตร Read/Write Throughput

\[
\text{Read MB/s} = \frac{\Delta \text{sectors\_read} \times 512}{1{,}048{,}576 \times \Delta t}
\]

\[
\text{Write MB/s} = \frac{\Delta \text{sectors\_written} \times 512}{1{,}048{,}576 \times \Delta t}
\]

**ตัวอย่าง** (Δt = 1.0 s, sectors_read diff = 204,800):

\[
\text{Read MB/s} = \frac{204{,}800 \times 512}{1{,}048{,}576 \times 1.0} = \frac{104{,}857{,}600}{1{,}048{,}576} = 100.0 \text{ MB/s}
\]

### E.3 สูตร IOPS

\[
\text{Read IOPS} = \frac{\Delta \text{reads\_completed}}{\Delta t}
\]

\[
\text{Write IOPS} = \frac{\Delta \text{writes\_completed}}{\Delta t}
\]

**ตัวอย่าง** (Δt = 1.0 s, reads diff = 5,000): Read IOPS = 5,000

### E.4 สูตร Disk Utilization

Utilization คือสัดส่วนเวลาที่ disk "busy" (มี I/O อยู่):[^23][^3]

\[
\text{Utilization (\%)} = \frac{\Delta \text{io\_ticks}}{{\Delta t} \times 1000} \times 100
\]

**ตัวอย่าง** (Δt = 1.0 s, io_ticks diff = 750 ms):

\[
\text{Utilization} = \frac{750}{1000} \times 100 = 75.0\%
\]

หมายเหตุ: `io_ticks` นับ wall-clock milliseconds ที่ device มี request queued ตั้งแต่ kernel 5.0 เปลี่ยนวิธีนับ — ถ้ามี concurrent requests หลายตัว อาจน้อยกว่า wall time เล็กน้อย[^23]

### E.5 สูตร Average Request Latency

\[
\text{Avg Read Latency (ms)} = \frac{\Delta \text{time\_reading}}{\Delta \text{reads\_completed}}
\]

\[
\text{Avg Write Latency (ms)} = \frac{\Delta \text{time\_writing}}{\Delta \text{writes\_completed}}
\]

**ตัวอย่าง** (time_reading diff = 25,000 ms, reads diff = 5,000):

\[
\text{Avg Read Latency} = \frac{25{,}000}{5{,}000} = 5.0 \text{ ms/req}
\]

> **[kernel-documented]** `time_reading` วัดตั้งแต่ `blk_mq_alloc_request()` ถึง `__blk_mq_end_request()` — นับรวม queue wait time ไม่ใช่แค่ device service time[^23]

### E.6 สูตร Average Queue Depth

\[
\text{Avg Queue Depth} = \frac{\Delta \text{time\_in\_queue}}{\Delta \text{io\_ticks}}
\]

ค่านี้ใกล้เคียงกับ average number of I/Os ที่ pending พร้อมกัน

### E.7 การแปลง Sector Size

```
bytes_read    = sectors_read    × 512   (hardcoded kernel constant)
bytes_written = sectors_written × 512   (ไม่ใช่ logical_block_size)
```

ค่า `logical_block_size` ใช้สำหรับ filesystem/partition alignment ไม่ใช่ diskstats sector unit[^4]

***

## F. Rust Crates/API ที่แนะนำ

### F.1 Crate Comparison

| Crate | Version (2025) | ใช้สำหรับ | ข้อดี | ข้อเสีย |
|-------|---------------|----------|-------|---------|
| `procfs` | 0.16+ | `/proc/diskstats` parsing | DiskStat struct พร้อมใช้, Option fields สำหรับ kernel ต่าง version[^26][^27] | อ้าง `/proc/` path ตรง ไม่รองรับ custom path สำหรับ testing ง่าย ๆ |
| `udev` | 0.8+ | Block device enumeration, hot-plug | match_subsystem("block"), ดู udev DB[^28][^29] | ต้องมี libudev dynamic library, เพิ่ม dependency |
| `nix` | 0.29+ | ioctl (BLKGETSIZE64, BLKPBSZGET) | type-safe ioctl macros[^30][^31] | unsafe code, ต้องรู้ ioctl number |
| `rustix` | 0.38+ | Low-level syscalls, ioctl | Safe wrappers, ไม่ต้องมี libc wrapper[^32] | API ยังเปลี่ยนบ่อย |
| `std::fs` | stdlib | อ่าน sysfs/procfs | ไม่มี dependency เพิ่ม, เพียงพอสำหรับ virtual FS | ไม่มี hot-plug monitoring |
| `tokio::fs` | 1.x | async file ops | async API | **ไม่แนะนำ** สำหรับ sysfs/procfs ขนาดเล็ก เนื่องจาก spawn_blocking overhead มากกว่าประโยชน์[^33][^34] |

### F.2 คำแนะนำ

**[ข้อเสนอแนะผู้วิจัย]**

1. **สำหรับ MVP**: ใช้แค่ `std::fs` อ่าน `/proc/diskstats` และ `/sys/block/*/` โดยตรง — ไม่มี external dependency, ง่ายที่สุดในการ test

2. **สำหรับ production**: เพิ่ม `udev` crate เฉพาะส่วน hot-plug monitoring ไม่จำเป็นต้องใช้ udev สำหรับ initial enumeration (สามารถ readdir `/sys/class/block` ได้เอง)

3. **สำหรับ ioctl**: ใช้ `nix` crate `ioctl_read!` macro สำหรับ `BLKGETSIZE64` (0x12, 114) เมื่อต้องการ disk capacity ที่แม่นยำ[^30]

4. **ไม่ควรใช้ `tokio::fs`** สำหรับ sysfs/procfs: virtual filesystem เหล่านี้ไม่มี real blocking I/O, การใช้ tokio::fs จะสร้าง spawn_blocking tasks โดยไม่จำเป็น และมี memcpy overhead เพิ่ม[^33][^34][^35]

5. **อ่าน procfs/sysfs ด้วย single read syscall**: อ่าน buffer ขนาด 4096 bytes ในครั้งเดียว เพราะ kernel generate content ทั้งหมดใน single read — ไม่ควรใช้ `read_to_end()` ที่อาจเรียก read syscall หลายครั้ง[^35]

### F.3 Procfs Crate — DiskStat Fields

`procfs::DiskStat` struct มี fields ครบตาม kernel ABI:[^26]
- `reads`, `merged`, `sectors_read`, `time_reading` (F1-F4)
- `writes`, `writes_merged`, `sectors_written`, `time_writing` (F5-F8)
- `in_progress`, `time_in_progress`, `weighted_time_in_progress` (F9-F11)
- `discards: Option<usize>`, `discards_merged: Option<usize>` (F12-F13, kernel 4.18+)
- `sectors_discarded: Option<usize>`, `time_discarding: Option<usize>` (F14-F15)
- `flushes: Option<usize>`, `time_flushing: Option<usize>` (F16-F17, kernel 5.5+)

`Option` fields ทำให้รองรับ kernel ทุก version อัตโนมัติ

***

## G. Rust Code Skeleton

### G.1 Data Structures

```rust
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Identifies what kind of block device this is
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceKind {
    Physical,   // sda, nvme0n1, hda, sas0 — real hardware
    Partition,  // sda1, nvme0n1p1
    Loop,       // loop0..N
    DevMapper,  // dm-0..N (LVM, dm-crypt, multipath)
    MdRaid,     // md0..N
    Ram,        // ram0..N
    Zram,       // zram0..N
    NvmeHidden, // nvme0c0n1 (multipath hidden path)
    Unknown,
}

#[derive(Debug, Clone)]
pub struct BlockDevice {
    pub name: String,               // e.g. "sda", "nvme0n1"
    pub sysfs_path: PathBuf,        // /sys/block/<name>
    pub kind: DeviceKind,
    pub rotational: Option<bool>,   // None if file missing/unreadable
    pub model: Option<String>,
    pub vendor: Option<String>,
    pub serial: Option<String>,
    pub firmware_rev: Option<String>,
    pub logical_block_size: u64,    // from queue/logical_block_size
    pub physical_block_size: u64,   // from queue/physical_block_size
}

/// Raw counter snapshot from kernel
#[derive(Debug, Clone, Copy)]
pub struct RawStat {
    pub reads_completed: u64,
    pub reads_merged: u64,
    pub sectors_read: u64,
    pub time_reading_ms: u64,
    pub writes_completed: u64,
    pub writes_merged: u64,
    pub sectors_written: u64,
    pub time_writing_ms: u64,
    pub ios_in_progress: u64,    // NOT monotonic
    pub io_ticks_ms: u64,
    pub time_in_queue_ms: u64,
    // kernel 4.18+
    pub discards_completed: Option<u64>,
    pub sectors_discarded: Option<u64>,
    pub time_discarding_ms: Option<u64>,
    // kernel 5.5+
    pub flush_requests: Option<u64>,
    pub time_flushing_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct DiskSample {
    pub raw: RawStat,
    pub timestamp: Instant,  // std::time::Instant = CLOCK_MONOTONIC
}

#[derive(Debug, Clone)]
pub struct DiskMetrics {
    pub read_mb_per_s: f64,
    pub write_mb_per_s: f64,
    pub read_iops: f64,
    pub write_iops: f64,
    pub utilization_pct: f64,
    pub avg_read_latency_ms: f64,
    pub avg_write_latency_ms: f64,
    pub avg_queue_depth: f64,
    pub ios_in_progress: u64,    // point-in-time (not delta)
}
```

### G.2 Data Source Trait (testable abstraction)

```rust
/// Abstraction over /proc/diskstats or /sys/block/<dev>/stat
/// The `root` parameter allows injecting a test fixture directory
pub trait BlockDeviceSource {
    fn read_stat(&self, device_name: &str) -> Result<RawStat, MonitorError>;
    fn list_devices(&self) -> Result<Vec<String>, MonitorError>;
}

pub struct SysfsSource {
    pub root: PathBuf,  // normally "/" — set to fixture dir in tests
}

pub struct ProcDiskstatsSource {
    pub root: PathBuf,  // normally "/" — set to fixture dir in tests
}

impl BlockDeviceSource for SysfsSource {
    fn read_stat(&self, device_name: &str) -> Result<RawStat, MonitorError> {
        let path = self.root
            .join("sys/block")
            .join(device_name)
            .join("stat");
        let content = read_small_file(&path)?;  // single read, 4096-byte buffer
        parse_sysfs_stat(&content)
    }
    // ...
}
```

### G.3 Device Discovery

```rust
use std::fs;

pub fn discover_physical_disks(root: &Path) -> Vec<BlockDevice> {
    let block_dir = root.join("sys/class/block");
    let mut devices = Vec::new();

    let entries = match fs::read_dir(&block_dir) {
        Ok(e) => e,
        Err(_) => return devices,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let sysfs_path = entry.path();  // follows symlink via read_dir

        // Step 1: Classify device kind
        let kind = classify_device(&name, &sysfs_path);
        if !matches!(kind, DeviceKind::Physical) {
            continue;
        }

        // Step 2: Check slaves/ — if non-empty, this is a virtual device
        let slaves_dir = sysfs_path.join("slaves");
        if slaves_has_entries(&slaves_dir) {
            continue;  // dm, md constructed device
        }

        // Step 3: Check hidden flag (NVMe multipath hidden paths, kernel 6.2+)
        if read_sysfs_u64(&sysfs_path.join("hidden")).unwrap_or(0) == 1 {
            continue;
        }

        // Step 4: Read metadata
        let device = read_device_metadata(&name, &sysfs_path);
        devices.push(device);
    }
    devices
}

fn classify_device(name: &str, sysfs_path: &Path) -> DeviceKind {
    // Name-based patterns (stable kernel naming convention)
    if name.starts_with("loop") { return DeviceKind::Loop; }
    if name.starts_with("ram") { return DeviceKind::Ram; }
    if name.starts_with("dm-") { return DeviceKind::DevMapper; }
    if name.starts_with("md") && name.chars().nth(2).map_or(false, |c| c.is_ascii_digit()) {
        return DeviceKind::MdRaid;
    }
    if name.starts_with("zram") { return DeviceKind::Zram; }

    // NVMe multipath hidden path: nvme0c0n1 pattern
    if name.starts_with("nvme") && name.contains('c') {
        return DeviceKind::NvmeHidden;
    }

    // Check DEVTYPE from uevent (more reliable than name alone)
    let uevent = sysfs_path.join("uevent");
    if let Ok(content) = fs::read_to_string(&uevent) {
        for line in content.lines() {
            if line == "DEVTYPE=partition" { return DeviceKind::Partition; }
            if line == "DEVTYPE=disk" { return DeviceKind::Physical; }
        }
    }

    // Partitions: name ends with digit(s) after a letter
    // e.g. sda1, nvme0n1p1 — but NOT dm-0, md0, nvme0n1 (whole namespace)
    if name.contains(|c: char| c.is_ascii_alphabetic())
        && name.ends_with(|c: char| c.is_ascii_digit())
        && !name.starts_with("nvme")  // nvme0n1 is whole disk
    {
        return DeviceKind::Partition;
    }

    DeviceKind::Physical
}
```

### G.4 Throughput Calculation

```rust
pub fn compute_delta(s1: &DiskSample, s2: &DiskSample) -> DiskMetrics {
    let dt = s2.timestamp.duration_since(s1.timestamp).as_secs_f64();
    if dt <= 0.0 { /* handle zero/negative interval */ }

    // Counter wraparound: if new < old, a reset occurred — skip this interval
    let safe_delta = |new: u64, old: u64| -> u64 {
        new.wrapping_sub(old)  // u64 wrapping handles overflow correctly
    };

    let d_sectors_read = safe_delta(s2.raw.sectors_read, s1.raw.sectors_read);
    let d_sectors_written = safe_delta(s2.raw.sectors_written, s1.raw.sectors_written);
    let d_reads = safe_delta(s2.raw.reads_completed, s1.raw.reads_completed);
    let d_writes = safe_delta(s2.raw.writes_completed, s1.raw.writes_completed);
    let d_io_ticks = safe_delta(s2.raw.io_ticks_ms, s1.raw.io_ticks_ms);
    let d_time_reading = safe_delta(s2.raw.time_reading_ms, s1.raw.time_reading_ms);
    let d_time_writing = safe_delta(s2.raw.time_writing_ms, s1.raw.time_writing_ms);
    let d_time_in_queue = safe_delta(s2.raw.time_in_queue_ms, s1.raw.time_in_queue_ms);

    const SECTOR_SIZE: f64 = 512.0;  // kernel hardcoded constant
    const MB: f64 = 1_048_576.0;

    DiskMetrics {
        read_mb_per_s: (d_sectors_read as f64 * SECTOR_SIZE) / (MB * dt),
        write_mb_per_s: (d_sectors_written as f64 * SECTOR_SIZE) / (MB * dt),
        read_iops: d_reads as f64 / dt,
        write_iops: d_writes as f64 / dt,
        utilization_pct: (d_io_ticks as f64 / (dt * 1000.0)) * 100.0,
        avg_read_latency_ms: if d_reads > 0 {
            d_time_reading as f64 / d_reads as f64
        } else { 0.0 },
        avg_write_latency_ms: if d_writes > 0 {
            d_time_writing as f64 / d_writes as f64
        } else { 0.0 },
        avg_queue_depth: if d_io_ticks > 0 {
            d_time_in_queue as f64 / d_io_ticks as f64
        } else { 0.0 },
        ios_in_progress: s2.raw.ios_in_progress,  // point-in-time, no delta
    }
}
```

### G.5 Efficient Single-Read Helper

```rust
/// Read a small virtual FS file with a single syscall.
/// Virtual FS files should be read in one pass — not with read_to_end() which loops.
fn read_small_file(path: &Path) -> Result<String, MonitorError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut buf = [0u8; 4096];
    let n = file.read(&mut buf)?;
    Ok(std::str::from_utf8(&buf[..n])?.trim().to_owned())
}

fn read_sysfs_u64(path: &Path) -> Option<u64> {
    read_small_file(path).ok()?.parse().ok()
}

fn read_sysfs_string(path: &Path) -> Option<String> {
    read_small_file(path).ok().map(|s| s.trim().to_owned())
}
```

### G.6 NVMe Metadata Resolution

```rust
/// For nvme0n1, resolve controller path to get model/serial/firmware
fn read_nvme_metadata(name: &str, sysfs_path: &Path) -> (Option<String>, Option<String>, Option<String>) {
    // nvme0n1 → resolve symlink → .../nvme/nvme0/nvme0n1
    // Go up 1 level from nvme0n1 to nvme0 (controller)
    let controller_path = sysfs_path.parent();
    if let Some(ctrl) = controller_path {
        let model = read_sysfs_string(&ctrl.join("model"));
        let serial = read_sysfs_string(&ctrl.join("serial"));
        let firmware = read_sysfs_string(&ctrl.join("firmware_rev"));
        return (model, serial, firmware);
    }
    (None, None, None)
}
```

> **[ข้อเสนอแนะผู้วิจัย]** path traversal ข้างต้นต้องการ canonical path จาก symlink resolution — ควรใช้ `std::fs::canonicalize(sysfs_path)` ก่อนแล้วจึง traverse parent directories

***

## H. Edge Cases และ Failure Modes

### H.1 Counter Wraparound และ Device Reset

**[kernel-documented]** Counter รีเซ็ตที่:[^23]
- Boot time
- Device reattachment/reinitialization (hot-unplug/replug)
- Underlying counter overflow

**Detection strategy**: ถ้า new_value < old_value (หลังจาก accounting สำหรับ u64 wrap), ให้ถือว่า counter reset — skip delta interval นั้น หรือ report gap:

```rust
fn detect_reset(new: u64, old: u64) -> bool {
    // u64 overflow: จะ wrap ที่ ~18.4 exabytes ของ sector reads — ไม่เกิดในชีวิตจริง
    // Real reset: new value ต่ำกว่า old อย่างมีนัยสำคัญ
    new < old && (old - new) > u64::MAX / 2  // likely wrap, not reset
    // ถ้า old - new <= u64::MAX/2 = true reset
}
```

### H.2 Hot-plug: Device Removal ระหว่าง Sampling

- อ่าน `/sys/block/<dev>/stat` แล้วได้ `ENOENT` = device ถูก remove ออก
- Handle ด้วย `Result<>` และ remove device จาก tracking map
- Spurious removal/reattachment (USB) จะรีเซ็ต counters — ตรวจจับด้วย `diskseq` file (kernel 5.11+): monotonically increasing sequence number ต่อ drive — ถ้า diskseq เปลี่ยน = device ใหม่[^15]

### H.3 Elapsed Time ไม่คงที่

- `std::time::Instant` บน Linux ใช้ `CLOCK_MONOTONIC` (ไม่กระโดดถอยหลัง)[^36][^37]
- NTP adjustments อาจทำให้ CLOCK_MONOTONIC ถูก slew (ไม่ใช่ jump) — ยอมรับได้สำหรับ ~1 second intervals[^38]
- ถ้าต้องการ NTP-immune timing ใช้ `CLOCK_MONOTONIC_RAW` ผ่าน `nix::time::clock_gettime`

### H.4 Concurrent Reads

`io_ticks` ตั้งแต่ kernel 5.0 นับ jiffies ที่มี request อย่างน้อยหนึ่งตัว — ถ้ามี concurrent requests มาก utilization อาจ under-count เล็กน้อย[^23]

### H.5 Virtual Machines

ใน KVM/QEMU: `virtio-blk` driver อาจไม่ set `rotational` flag ถูกต้อง — ไม่สามารถแยก SSD จาก HDD ได้จาก `rotational` เพียงอย่างเดียว[^5]

### H.6 /sys/block vs /sys/class/block

ใน kernel 2.6.22–2.6.25 (เก่ามาก): `/sys/block` อาจไม่มีถ้า `CONFIG_SYSFS_DEPRECATED` ไม่ได้ enable — ใน kernel ปัจจุบัน `/sys/class/block` เป็น canonical path[^9]

### H.7 Kernel Version ที่ไม่ support flush/discard fields

Fields เหล่านี้จะไม่อยู่ใน `/proc/diskstats` บน kernel เก่า — `procfs` crate จัดการด้วย `Option<>` fields อัตโนมัติ สำหรับ manual parsing ให้นับจำนวน columns และ skip fields ที่เกิน[^26]

***

## I. Testing Strategy

### I.1 Fixture-based Unit Tests

**[ข้อเสนอแนะผู้วิจัย]** สร้าง directory structure จำลอง sysfs/procfs:

```
tests/fixtures/
├── sys/
│   └── class/
│       └── block/
│           ├── sda -> ../../block/sda   (symlink, optional)
│           ├── nvme0n1/
│           │   ├── stat        ← คัดลอกจาก real system หรือสร้างเอง
│           │   ├── uevent      ← "DEVTYPE=disk\n"
│           │   ├── hidden      ← "0\n"
│           │   ├── slaves/     ← empty directory
│           │   └── queue/
│           │       ├── rotational        ← "0\n"
│           │       ├── logical_block_size ← "512\n"
│           │       └── physical_block_size ← "4096\n"
│           ├── sda/
│           │   ├── stat
│           │   ├── uevent
│           │   └── device/
│           │       ├── model   ← "Samsung SSD 860\n"
│           │       └── vendor  ← "ATA\n"
│           └── dm-0/           ← should be filtered out
│               ├── stat
│               └── uevent      ← "DEVTYPE=disk\n"
└── proc/
    └── diskstats               ← realistic multi-device content
```

จากนั้น inject root prefix ผ่าน `BlockDeviceSource::new("/path/to/fixture")` แทนที่จะเป็น `"/"`

### I.2 Counter Wraparound Tests

```rust
#[test]
fn test_counter_wraparound() {
    let s1 = DiskSample { raw: RawStat { sectors_read: u64::MAX - 100, .. }, .. };
    let s2 = DiskSample { raw: RawStat { sectors_read: 100, .. }, .. };
    // wrapping_sub: (100u64).wrapping_sub(u64::MAX - 100) = 201
    let metrics = compute_delta(&s1, &s2);
    assert_eq!(metrics sector delta, 201);
}
```

### I.3 Device Removal Simulation

จำลองโดย create fixture path แล้วลบไฟล์ระหว่าง test:

```rust
#[test]
fn test_device_removal_returns_error() {
    let dir = tempdir().unwrap();
    // สร้าง fixture stat file
    let stat_path = dir.path().join("sys/block/sda/stat");
    fs::write(&stat_path, b"1 2 3 ...").unwrap();
    let source = SysfsSource::new(dir.path());
    // ลบไฟล์
    fs::remove_file(&stat_path).unwrap();
    assert!(source.read_stat("sda").is_err());
}
```

### I.4 Integration Tests (สำหรับ development)

```bash
# ตรวจสอบผล library เทียบกับ iostat ระหว่าง development
# (ไม่ใช่ implementation หลัก)
iostat -x 1 2 | grep sda
cargo run --example monitor -- --device sda --interval 1
```

### I.5 Parse Tests สำหรับ Different Kernel Versions

สร้าง fixture `proc/diskstats` แยกสำหรับ:
- 14-field format (kernel < 4.18)
- 18-field format (kernel 4.18–5.4)
- 20-field format (kernel 5.5+)

***

## J. Implementation Roadmap

### Phase 1: MVP

**เป้าหมาย**: Basic disk monitoring ที่ใช้งานได้บน production Linux

| Task | Details | Kernel ABI Used |
|------|---------|----------------|
| `/proc/diskstats` parser | Parse 14, 18, 20 field variants | Stable ABI |
| Device discovery | readdir `/sys/class/block`, name filter | Stable |
| Physical disk filter | uevent DEVTYPE, slaves/ check | Testing ABI |
| Throughput calculator | delta metrics ทุก 5 สูตร | N/A (คำนวณ) |
| Rotational detection | `queue/rotational` | Stable |
| Sector size read | `queue/logical_block_size`, `physical_block_size` | Stable |
| Basic error handling | ENOENT, parse error, zero interval | N/A |
| Unit tests | Fixture-based, no real hardware needed | N/A |

### Phase 2: Production Hardening

| Task | Details | Kernel Version |
|------|---------|---------------|
| NVMe metadata | `/sys/class/nvme/nvmeN/` model, serial, firmware | 3.3+ (NVMe) |
| Hot-plug monitoring | `udev` crate, netlink socket monitor | N/A |
| `diskseq` tracking | detect genuine remove/reinsert | 5.11+ |
| Counter reset detection | skip intervals when reset detected | N/A |
| `hidden` flag | NVMe multipath hidden path filter | 6.2+ |
| CLOCK_MONOTONIC_RAW | NTP-immune timing via nix | 2.6.28+ |
| `dm-` / `md` device mapping | resolve slaves/ to physical disks | N/A |
| flush/discard metrics | Optional fields kernel 4.18+/5.5+ | 4.18+, 5.5+ |
| Per-partition stats | `/sys/block/<disk>/<part>/stat` | 2.6.25+ |
| Integration test harness | compare vs iostat in CI environment | N/A |

### Phase 3: Extended Features

- ioctl `BLKGETSIZE64` สำหรับ capacity ที่แม่นยำ
- ATA/SCSI identity ผ่าน `HDIO_GET_IDENTITY` ioctl (ต้องการ root)
- Bus type detection ผ่าน sysfs path parsing
- SAS/SCSI vendor-specific VPD pages

***

## K. Primary Sources

| ชื่อเอกสาร | URL | หมายเหตุ |
|-----------|-----|---------|
| Linux Kernel: I/O statistics fields | https://docs.kernel.org/admin-guide/iostats.html | **Primary** — /proc/diskstats field definitions[^23] |
| Linux Kernel: /sys/block/stat | https://docs.kernel.org/block/stat.html | **Primary** — stat file field definitions[^3] |
| Linux Kernel ABI: /proc/diskstats | https://www.kernel.org/doc/Documentation/ABI/testing/procfs-diskstats | Stable ABI reference[^1] |
| Linux Kernel ABI: sysfs-block (stable) | https://www.kernel.org/doc/Documentation/ABI/stable/sysfs-block | Stable queue attributes[^15] |
| man7.org: proc_diskstats(5) | https://man7.org/linux/man-pages/man5/proc_diskstats.5.html | Man page reference[^2] |
| sysfs rules | https://www.kernel.org/doc/html/v4.19/admin-guide/sysfs-rules.html | How to use sysfs correctly[^39] |
| procfs crate docs | https://docs.rs/procfs/latest/procfs/ | DiskStats, DiskStat struct[^27][^40] |
| udev crate (Rust) | https://docs.rs/udev/latest/udev/ | Enumerator, Monitor API[^29][^41] |
| nix crate — ioctl example | https://github.com/nix-rust/nix/issues/573 | BLKGETSIZE64 usage[^31] |
| procfs-rust-benchmarks | https://github.com/joshuarli/procfs-rust-benchmarks | Single-read vs multi-read perf[^35] |
| Linux block/stat.h SECTOR_SIZE | https://elixir.bootlin.com/linux/latest/ident/SECTOR_SIZE | 512-byte sector constant[^4] |
| sysfs(5) man page | https://manpages.debian.org/bookworm/manpages/sysfs.5.en.html | /sys hierarchy overview[^8] |
| dm/md slaves/holders patches | https://lwn.net/Articles/172689/ | sysfs stacked device representation[^7] |
| NVMe sysfs data | https://utcc.utoronto.ca/~cks/space/blog/linux/NVMeSysfsData | model, serial, firmware_rev paths[^18] |
| tokio::fs documentation | https://docs.rs/tokio/latest/tokio/fs/ | spawn_blocking behavior[^42] |
| std::time::Instant | https://doc.rust-lang.org/std/time/struct.Instant.html | CLOCK_MONOTONIC wrapper[^36] |

***

## สรุปข้อแตกต่าง: Stable ABI vs Implementation Detail

| ข้อมูล | สถานะ | หมายเหตุ |
|--------|--------|---------|
| `/proc/diskstats` fields 1-11 | **Stable ABI** | ตั้งแต่ kernel 2.5.69 |
| Fields 12-15 (discard) | **Stable ABI** | kernel 4.18+ |
| Fields 16-17 (flush) | **Stable ABI** | kernel 5.5+ |
| `/sys/block/<dev>/stat` format | **Stable ABI** | documented ใน sysfs-block stable |
| `/sys/block/<dev>/queue/rotational` | **Stable ABI** | อาจรายงานผิดใน VM |
| `/sys/block/<dev>/queue/logical_block_size` | **Stable ABI** | |
| `/sys/block/<dev>/device/model` | **Testing ABI** | อาจเปลี่ยนได้, driver-dependent |
| `/sys/class/nvme/nvmeN/model` | **Testing ABI** | kernel ≥ 3.3, driver-dependent |
| สูตรคำนวณ Read MB/s, utilization | **ข้อเสนอแนะผู้วิจัย** | ตาม kernel field definitions |
| การใช้ `std::fs` แทน `tokio::fs` | **ข้อเสนอแนะผู้วิจัย** | อิงจาก benchmark analysis |
| Fixture-based testing pattern | **ข้อเสนอแนะผู้วิจัย** | best practice |
| การ filter ด้วย `slaves/` directory | **ข้อเสนอแนะผู้วิจัย** | อิงจาก kernel patch documentation[^7] |

---

## References

1. [proc/diskstats](https://www.kernel.org/doc/Documentation/ABI/testing/procfs-diskstats)

2. [proc_diskstats(5) - Linux manual page - man7.org](https://man7.org/linux/man-pages/man5/proc_diskstats.5.html)

3. [Block layer statistics in /sys/block/<dev>/stat](https://docs.kernel.org/block/stat.html)

4. [[FR] New module for `sysfs-block` · Issue #1601 · prometheus/node_exporter](https://github.com/prometheus/node_exporter/issues/1601) - As per our chat, sysfs-block would be useful as an optional module to learn more about the underlyin...

5. [disks are SSD, but /sys/block/vdb/queue/rotational says it's rotational - should i worry?](https://unix.stackexchange.com/questions/508497/disks-are-ssd-but-sys-block-vdb-queue-rotational-says-its-rotational-should) - Is it a problem when the system believes a device to be rotational, but it is SSD really? The system...

6. [How To Check if a Disk is an SSD or an HDD on Linux](https://zaiste.net/os/unix/howtos/howto-check-ssd-hdd-linux/) - Zaiste Programming is a personal website by Jakub Neander about programming. It covers Java, Ruby, P...

7. [[PATCH 0/3] sysfs representation of stacked devices (dm/md)](https://lwn.net/Articles/172689/)

8. [sysfs(5) - bookworm](https://manpages.debian.org/bookworm/manpages/sysfs.5.en.html)

9. [linux /sys目录下的各个子目录说明](https://blog.csdn.net/luckywang1103/article/details/25715101) - 文章浏览阅读1.4w次，点赞2次，收藏17次。# ls /sys/block class firmware kernel powerbus devices fs module-------------...

10. [60-persistent-storage.rules - Git repositories on kernel](https://kernel.googlesource.com/pub/scm/linux/hotplug/udev/+/128/rules/rules.d/60-persistent-storage.rules)

11. [14 Discovery of block devices](https://www.zabbix.com/documentation/current/en/manual/discovery/low_level_discovery/examples/devices)

12. [Sysfs paths to NVME devices - Mailing Lists](https://lists.infradead.org/pipermail/linux-nvme/2021-November/028753.html)

13. [Should Hard drives always set the rotatinonal flag in /sys/block/<device>/queue/rotational?](https://serverfault.com/questions/1092590/should-hard-drives-always-set-the-rotatinonal-flag-in-sys-block-device-queue) - Folks, AWS exposes HDDs in their D3 instances (https://aws.amazon.com/ec2/instance-types/d3/) as nvm...

14. [90761](https://bugzilla.kernel.org/show_bug.cgi?id=90761)

15. [sysfs-block - The Linux Kernel Archives](https://www.kernel.org/doc/Documentation/ABI/stable/sysfs-block)

16. [Bad drivers dont populate /sys file system with correct vendor and model information · Issue #783 · openmediavault/openmediavault](https://github.com/openmediavault/openmediavault/issues/783) - Description of issue/question Bad drivers don't populate /sys file system with correct vendor and mo...

17. [`lsblk` shows empty VENDOR name for my devices. · Issue #823 · util-linux/util-linux](https://github.com/util-linux/util-linux/issues/823) - Hi, I am using lsblk to get information about my block device vendor name, and model But lsblk shows...

18. [What data about your NVMe drives Linux puts in sysfs](https://utcc.utoronto.ca/~cks/space/blog/linux/NVMeSysfsData)

19. [FAQ Entry | Online Support | Support](https://www.supermicro.com/en/support/faqs/faq.php?faq=44947) - Frequently Asked Questions

20. [Managing System Devices With udev](https://docs.oracle.com/en/operating-systems/oracle-linux/9/udev/udev-QueryingUdevandSysfs_limiting_device_information_by_query_type.html) - The following examples show how to limit device information by query type.

21. [proc/diskstats - disk I/O statistics](https://manpages.ubuntu.com/manpages/noble/man5/proc_diskstats.5.html)

22. [zte-kernel-msm7x27/Documentation/ABI/testing/sysfs-block at cm-11.0 · zeelog/zte-kernel-msm7x27](https://github.com/zeelog/zte-kernel-msm7x27/blob/cm-11.0/Documentation/ABI/testing/sysfs-block) - Linux kernel source for ZTE Blade. Contribute to zeelog/zte-kernel-msm7x27 development by creating a...

23. [I/O statistics fields - The Linux Kernel documentation](https://docs.kernel.org/admin-guide/iostats.html)

24. [sysfs-block](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-block)

25. [[PATCH] NVMe:Expose model attribute in sysfs - Mailing Lists](http://lists.infradead.org/pipermail/linux-nvme/2015-September/002315.html)

26. [procfs::DiskStat - Rust](https://tikv.github.io/doc/procfs/struct.DiskStat.html) - Disk IO stat information

27. [procfs - Rust - Docs.rs](https://docs.rs/procfs/latest/procfs/) - This crate provides to an interface into the linux `procfs` filesystem, usually mounted at `/proc`.

28. [GitHub - Stebalien/udev-rs: Udev bindings for rust](https://github.com/Stebalien/udev-rs) - Udev bindings for rust. Contribute to Stebalien/udev-rs development by creating an account on GitHub...

29. [udev](https://lib.rs/crates/udev) - libudev bindings for Rust

30. [How to get block device size on Linux with Rust - Distributed rumblings](http://syhpoon.ca/posts/how-to-get-block-device-size-on-linux-with-rust/) - While working on LearnFS, one of the task I had was to determine the block device size in bytes . Th...

31. [Additional examples for ioctl · Issue #573 · nix-rust/nix](https://github.com/nix-rust/nix/issues/573) - Some additional examples for ioctl would be very helpful. It's taken me a bit to figure out how I'm ...

32. [rustix/fs/ ioctl.rs](https://docs.rs/rustix/latest/src/rustix/fs/ioctl.rs.html) - Source of the Rust file `src/fs/ioctl.rs`.

33. [`tokio::fs` + async is 1-2 orders of magnitude slower than a blocking ...](https://github.com/tokio-rs/tokio/issues/3664) - Version 1.4.0 Platform 64-bit WSL2 Linux: Linux 4.19.104-microsoft-standard #1 SMP x86_64 x86_64 x86...

34. [File reading: async / sync performance differences (hyper, tokio)](https://users.rust-lang.org/t/file-reading-async-sync-performance-differences-hyper-tokio/34696/15) - A quick look at the async version (of the code you pasted inline) in perf reveals that tokio::fs::Fi...

35. [procfs-rust-benchmarks/README.md at master · joshuarli/procfs-rust-benchmarks](https://github.com/joshuarli/procfs-rust-benchmarks/blob/master/README.md) - procfs from rust, benchmarks. Contribute to joshuarli/procfs-rust-benchmarks development by creating...

36. [Instant in std::time - Rust](https://doc.rust-lang.org/std/time/struct.Instant.html) - A measurement of a monotonically nondecreasing clock. Opaque and useful only with `Duration`.

37. [High accuracy timer - help](https://users.rust-lang.org/t/high-accuracy-timer/29019) - How in Rust to measure time with high accuracy above million ticks/second like std::chrono in C++? R...

38. [time.rs should consider using CLOCK_MONOTONIC_RAW instead of CLOCK_MONOTONIC on Linux · Issue #37902 · rust-lang/rust](https://github.com/rust-lang/rust/issues/37902) - I have reproducible compiler crashes on arm64: error: internal compiler error: unexpected panic note...

39. [Rules on how to access information in sysfs](https://www.kernel.org/doc/html/v4.19/admin-guide/sysfs-rules.html)

40. [DiskStats in procfs - Rust - Docs.rs](https://docs.rs/procfs/latest/procfs/struct.DiskStats.html) - A list of disk stats.

41. [Struct EnumeratorCopy item path](https://docs.rs/udev/latest/udev/struct.Enumerator.html) - An enumeration context.

42. [tokio::fs - Rust](https://strawlab.org/strand-braid-api-docs/latest/tokio/fs/index.html) - Asynchronous file and standard stream adaptation.

