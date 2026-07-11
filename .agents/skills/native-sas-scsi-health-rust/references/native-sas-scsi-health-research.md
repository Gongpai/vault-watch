# Native SAS/SCSI Disk Health Monitoring ด้วย Rust: SG_IO, SCSI Commands และ Architecture

***

## A. Feasibility Verdict

**✅ Feasible — แต่มีข้อจำกัดที่ต้องเข้าใจก่อน**

การสร้าง native SAS/SCSI health backend ใน Rust ที่ส่ง SCSI commands โดยตรงผ่าน Linux `SG_IO` ioctl เป็นไปได้อย่างสมบูรณ์ เนื่องจาก:

- `SG_IO` ioctl เป็น **stable, well-documented Linux kernel ABI** ที่ใช้งานอยู่ใน production tooling (sg3_utils, smartmontools) มาหลาย decade[^1]
- Commands สำคัญสำหรับ health monitoring (`INQUIRY`, `LOG SENSE`, `TEST UNIT READY`, `MODE SENSE`) ทำได้จาก `O_RDONLY` โดยไม่ต้องการ `CAP_SYS_RAWIO`[^2]
- ข้อมูลใน `sg_io_hdr_t` structure เป็น public UAPI ใน `<scsi/sg.h>`[^3][^4]

**ข้อจำกัดสำคัญ (confidence: HIGH — kernel-documented):**
1. **Permission barrier**: `/dev/sg*` nodes ต้องการ `disk` group หรือ `root` สำหรับ open — ต้องวางแผน privilege model ก่อน[^1]
2. **SAS vs SATA**: หาก disk เป็น SATA ที่อยู่หลัง SAS HBA หรือ hardware RAID controller ที่ไม่ forward passthrough — SCSI health commands อาจล้มเหลวโดยสิ้นเชิง ต้องมี SAT/smartctl fallback[^1]
3. **Vendor-specific data**: บาง log page ที่สำคัญที่สุดสำหรับ predictive failure เป็น vendor-specific — standardized fields ให้ข้อมูลพื้นฐานแต่ไม่ครบ
4. **SG_IO v4 interface** (sg driver ≥ 4.0): มีใน kernel 5.0+ แต่ยัง experimental — ควรใช้ `sg_io_hdr_t` (v3) เป็น primary interface[^5]

***

## B. Minimum Viable Set of SCSI Commands

สำหรับ health monitoring ที่ใช้งานได้จริง ต้องการ commands เหล่านี้ขั้นต่ำ (เรียงตาม priority):

| Priority | Command | Opcode | วัตถุประสงค์ |
|----------|---------|--------|-------------|
| P1 | `TEST UNIT READY` | 0x00 | ตรวจว่า device พร้อมรับคำสั่ง |
| P1 | `INQUIRY` (standard) | 0x12 | Vendor, product, revision, device type |
| P1 | `INQUIRY` VPD 0x00 | 0x12 EVPD=1 | รายการ supported VPD pages |
| P1 | `INQUIRY` VPD 0x80 | 0x12 EVPD=1 | Unit serial number |
| P1 | `INQUIRY` VPD 0x83 | 0x12 EVPD=1 | Device identification (SAS address) |
| P1 | `LOG SENSE` page 0x0D | 0x4D | Temperature (current + reference max) |
| P1 | `LOG SENSE` page 0x02 | 0x4D | Write error counter |
| P1 | `LOG SENSE` page 0x03 | 0x4D | Read error counter |
| P1 | `LOG SENSE` page 0x2F | 0x4D | Informational exceptions (failure prediction) |
| P2 | `INQUIRY` VPD 0xB1 | 0x12 EVPD=1 | Block Device Characteristics (rotation rate) |
| P2 | `INQUIRY` VPD 0x89 | 0x12 EVPD=1 | ATA Information (ตรวจว่าเป็น SATA-behind-SAT) |
| P2 | `LOG SENSE` page 0x0E | 0x4D | Start-stop cycle counter |
| P2 | `LOG SENSE` page 0x15 | 0x4D | Background scan results |
| P2 | `LOG SENSE` page 0x05 | 0x4D | Verify error counter |
| P2 | `LOG SENSE` page 0x06 | 0x4D | Non-medium error counter |
| P2 | `LOG SENSE` page 0x18 | 0x4D | Protocol-specific port (SAS PHY info) |
| P3 | `READ DEFECT DATA (10)` | 0x37 | Grown defect list count |
| P3 | `LOG SENSE` page 0x00 | 0x4D | ตรวจว่า device support page ใดบ้าง |
| P3 | `REQUEST SENSE` | 0x03 | อ่าน deferred/pending sense data |

> **[kernel-documented]** Commands ทั้งหมดใน P1 และส่วนใหญ่ใน P2 ทำได้ผ่าน `SG_IO` ด้วย `O_RDONLY` โดยไม่ต้องการ `CAP_SYS_RAWIO`[^6][^2]

***

## C. Command/Log Page Reference Table

### C.1 SCSI Commands

| Command | Opcode | Permission (sg) | Permission (block) | Standard | Safe? | Polling |
|---------|--------|-----------------|-------------------|----------|-------|---------|
| TEST UNIT READY | 0x00 | O_RDONLY | O_RDONLY | SPC-4 | ✅ read-only, no state change | 30s |
| INQUIRY standard | 0x12 | O_RDONLY | O_RDONLY | SPC-4 | ✅ read-only | Static (cache result) |
| INQUIRY VPD | 0x12 EVPD=1 | O_RDONLY | O_RDONLY | SPC-4 | ✅ read-only | Static |
| LOG SENSE | 0x4D | O_RDONLY | O_RDONLY | SPC-4 | ✅ read-only | See C.2 |
| MODE SENSE (6/10) | 0x1A / 0x5A | O_RDONLY | O_RDONLY | SPC-4 | ✅ read-only | Static |
| REQUEST SENSE | 0x03 | O_RDONLY | O_RDONLY | SPC-4 | ✅ read-only | On CHECK CONDITION only |
| READ DEFECT DATA (10) | 0x37 | **O_RDWR** (sg) | **O_RDONLY** (block) | SBC-3 | ✅ read-only | Weekly |
| LOG SELECT | 0x4C | O_RDWR | O_RDWR | SPC-4 | ⚠️ clears counters | ❌ ห้ามใช้ใน monitoring |
| FORMAT UNIT | 0x04 | O_RDWR | O_RDWR | SBC-3 | ❌ destructive | ❌ ห้ามใช้ |
| SYNCHRONIZE CACHE | 0x35 | O_RDWR | O_RDWR | SBC-3 | ⚠️ state change | ❌ ไม่ต้องการ |

> **[sg.danny.cz — documented]** สำหรับ `/dev/sg*` device: `READ DEFECT DATA (10)` ต้องการ `O_RDWR` แต่เมื่อเปิดผ่าน block device (`/dev/sdX`) SG_IO ใช้ `O_RDONLY` ได้[^2][^1]

### C.2 LOG SENSE Pages สำหรับ SAS Disk

| Page Code | ชื่อ | Key Parameters | Standard | Vendor-specific? | Polling Interval |
|-----------|------|----------------|----------|-----------------|-----------------|
| 0x00 | Supported log pages | List of supported page codes | SPC-4 | No | Once at startup |
| 0x02 | Write error counter | Total errors, corrected, uncorrected | SPC-4 §7.3 | No | 5 min |
| 0x03 | Read error counter | Total errors, corrected, uncorrected | SPC-4 §7.3 | No | 5 min |
| 0x05 | Verify error counter | Same structure as read/write | SPC-4 | No | 30 min |
| 0x06 | Non-medium error | Non-media errors count | SPC-4 | No | 30 min |
| 0x0D | Temperature | Current temp (°C), reference max temp | SPC-4 §7.3.12 | No | 60s |
| 0x0E | Start-stop cycle | Accumulated cycles vs. lifetime limit | SPC-4 §7.3.13 | No | Daily |
| 0x10 | Self-test results | Last 20 self-test results | SPC-4 | No | On demand |
| 0x15 | Background scan results | Medium scan status, defects found | SBC-3 §6.3.3 | No | 30 min |
| 0x18 | Protocol-specific port | SAS PHY info, error counters | SAS-2/SAS-3 | SAS-specific | 5 min |
| 0x2F | Informational exceptions | AURA failure prediction, MRIE | SPC-4 §7.3.18 | Partially | 30s |
| 0x37 | Seagate cache | Cache hit ratio, etc. | Seagate | **Yes** | 5 min |

> **[SPC-4 / T10-documented, confidence: HIGH]** Log pages 0x02, 0x03, 0x05, 0x06, 0x0D, 0x0E, 0x2F เป็น standardized และ mandatory สำหรับ SAS HDD ตาม Microsoft HLK requirement[^7]

### C.3 VPD Pages สำคัญ

| Page Code | ชื่อ | ข้อมูล | ใช้ detect SAT/ATA ได้? |
|-----------|------|--------|------------------------|
| 0x00 | Supported VPD Pages | รายการ supported pages | ✅ ตรวจว่า 0x89 present |
| 0x80 | Unit Serial Number | Serial number string | No |
| 0x83 | Device Identification | NAA IEEE SAS address, LUN ID | No (SAS-specific: protocol=6h)[^8] |
| 0x89 | ATA Information | ATA IDENTIFY DEVICE data | ✅ **ถ้า present = SATA-via-SAT**[^9] |
| 0xB0 | Block Limits | Max unmap LBA count | No |
| 0xB1 | Block Device Characteristics | Medium rotation rate (0001h=SSD, RPM otherwise) | ✅ ใช้แยก SSD/HDD ได้[^10] |
| 0xB2 | Logical Block Provisioning | Thin provisioning support | No |

***

## D. Linux SG_IO Request/Response Flow

### D.1 Flow Diagram

```
User Process
    │
    ├─► open("/dev/sg0", O_RDONLY | O_NONBLOCK)
    │         │
    │         ▼
    │   fd = file descriptor (major number 21)
    │
    ├─► ตั้งค่า sg_io_hdr_t:
    │   ┌─────────────────────────────────────┐
    │   │ interface_id = 'S'                  │ [i] required
    │   │ dxfer_direction = SG_DXFER_FROM_DEV │ [i] read from device
    │   │ cmd_len = length of CDB             │ [i]
    │   │ mx_sb_len = sizeof(sense_buf)       │ [i] max sense data
    │   │ dxfer_len = sizeof(data_buf)        │ [i]
    │   │ dxferp = &data_buf                  │ [i*] output buffer
    │   │ cmdp = &cdb                         │ [i*] SCSI CDB
    │   │ sbp = &sense_buf                    │ [i*] sense data output
    │   │ timeout = 30000 (ms)                │ [i]
    │   └─────────────────────────────────────┘
    │
    ├─► ioctl(fd, SG_IO, &io_hdr)
    │         │
    │         ▼
    │   Phase 1: Kernel validates sg_io_hdr_t
    │         │  ─ interface_id != 'S' → ENOSYS (sg) / EINVAL (block)
    │         │  ─ Command permission check (opcode sniffing)
    │         │  ─ Copies CDB and setup metadata from userspace
    │         │
    │         ▼
    │   Phase 2: SCSI mid-level → LLD → HBA → Device
    │         │  ─ DMA transfer occurs here
    │         │  ─ Waits for response or timeout
    │         │
    │         ▼
    │   Phase 3: Write output fields back to sg_io_hdr_t
    │         │  ─ status (SCSI status byte)
    │         │  ─ host_status (DID_* codes)
    │         │  ─ driver_status (DRIVER_* codes)
    │         │  ─ sb_len_wr, resid, duration
    │         │  ─ สำหรับ data transfer: write to *dxferp
    │
    ▼
    io_hdr.status == 0x00 (GOOD)
    io_hdr.status == 0x02 (CHECK CONDITION) → parse sense data
```

### D.2 ตรวจสอบ Response

ลำดับการตรวจสอบ response ที่ถูกต้อง:

```rust
// Step 1: ioctl return value
if ioctl_ret < 0 { /* errno set: transport/syscall error */ }

// Step 2: Check SG_INFO_OK bit
if (io_hdr.info & SG_INFO_OK_MASK) != SG_INFO_OK {
    // SCSI command completed with error
}

// Step 3: Check SCSI status
match io_hdr.status {
    0x00 => { /* GOOD — data in dxferp is valid */ }
    0x02 => { /* CHECK CONDITION — parse sense data */ }
    0x08 => { /* BUSY — retry after delay */ }
    0x18 => { /* RESERVATION CONFLICT */ }
    _    => { /* other status */ }
}

// Step 4: Check host_status (transport error)
if io_hdr.host_status != 0 { /* DID_NO_CONNECT, DID_TIMEOUT, etc. */ }

// Step 5: Check driver_status
if io_hdr.driver_status != 0 { /* DRIVER_SENSE, DRIVER_TIMEOUT, etc. */ }
```

***

## E. Mapping Algorithm: Block Device → Generic SCSI Device

### E.1 Canonical Sysfs Method (No External Tools Required)

**[kernel ABI — ข้อเสนอแนะผู้วิจัย]** วิธีที่ดีที่สุดคือผ่าน sysfs โดยตรง:

```
/sys/block/sda/device/scsi_generic/sg0  ← directory ชื่อ sg0 = sg device name
```

ขั้นตอน:
1. Resolve `/sys/block/<disk_name>` (follow symlink ถ้าจำเป็น)
2. เข้าไปที่ `device/scsi_generic/` directory
3. `readdir()` เพื่อได้ชื่อ entry (e.g., "sg0") — นั่นคือ sg device name
4. sg device node อยู่ที่ `/dev/sg0`

```rust
fn find_sg_device(block_name: &str) -> Option<String> {
    let sg_generic_dir = format!("/sys/block/{}/device/scsi_generic", block_name);
    // readdir: entries in this directory are sg device names (e.g. "sg0")
    let entries = std::fs::read_dir(&sg_generic_dir).ok()?;
    for entry in entries.flatten() {
        let sg_name = entry.file_name().to_string_lossy().to_string();
        // sg_name = "sg0", "sg1", etc.
        return Some(format!("/dev/{}", sg_name));
    }
    None  // Not a SCSI device (e.g. ATA-only, NVMe)
}
```

**ข้อสังเกต**: ถ้า `device/scsi_generic/` directory ไม่มีอยู่ = device ไม่ใช่ SCSI device หรือ `sg` module ยังไม่ได้ load[^11][^12]

### E.2 Alternative: Major:Minor Method

```rust
fn find_sg_via_major_minor(block_name: &str) -> Option<String> {
    // อ่าน major:minor ของ block device
    let dev_str = std::fs::read_to_string(
        format!("/sys/block/{}/dev", block_name)
    ).ok()?;  // e.g. "8:0"
    
    // Enumerate /sys/class/scsi_generic/sg*/dev และเปรียบเทียบ major:minor
    // ไม่แนะนำ: ซับซ้อนกว่าวิธีแรกและ major:minor ไม่ unique ใน multi-path
    None  // fallback
}
```

### E.3 udev Property Method (ต้องการ udev crate)

ผ่าน udev properties `SCSI_GENERIC` หรือ parent device sysname:
```rust
// ผ่าน udev: enumerate "scsi_generic" subsystem และ match parent sysname
// สำหรับ hot-plug monitoring ใช้ udev::MonitorBuilder
```

### E.4 ข้อควรระวัง

- `/dev/sgX` numbers ไม่ stable ระหว่าง reboot — ใช้ sysfs path หรือ `/dev/disk/by-id/` เป็น persistent identifier[^13]
- Device อาจมีหลาย sg devices (multipath) — sysfs directory อาจมีมากกว่าหนึ่ง entry
- ถ้า `sg` kernel module ไม่ได้ load — directory จะว่าง (`modprobe sg` แก้ได้)[^14]

***

## F. Rust Architecture และ Trait Definitions

### F.1 High-Level Architecture

```
ScsiHealthMonitor
├── DeviceMap (block → sg mapping via sysfs)
├── ScsiBackend trait
│   ├── NativeScsiBackend  ← SG_IO ioctl (primary)
│   ├── SatBackend         ← ATA PASS-THROUGH (SATA-via-SAT)
│   └── SmartctlBackend    ← smartctl JSON fallback (validation/unsupported)
├── CommandBuilder         ← CDB construction
├── ResponseParser         ← parse log pages, VPD pages
├── SenseDecoder           ← decode fixed/descriptor sense data
└── HealthAggregator       ← combine metrics into DiskHealth struct
```

### F.2 Core Traits

```rust
use std::time::Duration;

/// Abstract SCSI command execution — allows mock in tests
pub trait ScsiTransport: Send + Sync {
    /// Send a SCSI command and return response data
    fn execute(
        &self,
        cdb: &[u8],
        direction: DataDirection,
        data_buf: &mut Vec<u8>,
        timeout: Duration,
    ) -> Result<ScsiResponse, ScsiError>;
}

#[derive(Debug, Clone, Copy)]
pub enum DataDirection {
    None,        // SG_DXFER_NONE  (no data, e.g. TEST UNIT READY)
    FromDevice,  // SG_DXFER_FROM_DEV (read from device to host)
    ToDevice,    // SG_DXFER_TO_DEV  (write — avoid in monitoring)
}

#[derive(Debug)]
pub struct ScsiResponse {
    pub scsi_status: u8,      // 0x00=GOOD, 0x02=CHECK CONDITION
    pub host_status: u16,     // DID_* codes
    pub driver_status: u16,   // DRIVER_* codes
    pub sense_data: Vec<u8>,  // populated when status == CHECK CONDITION
    pub data: Vec<u8>,        // response payload
    pub duration_ms: u32,
    pub resid: i32,           // bytes not transferred
}

/// Parsed, decoded sense data
#[derive(Debug)]
pub struct SenseData {
    pub format: SenseFormat,   // Fixed or Descriptor
    pub sense_key: u8,
    pub asc: u8,
    pub ascq: u8,
    pub info: Option<u64>,
    pub raw: Vec<u8>,
}

#[derive(Debug)]
pub enum SenseFormat { Fixed, Descriptor }

/// Health data from a disk
#[derive(Debug)]
pub struct DiskHealth {
    pub temperature_c: Option<u8>,
    pub reference_temperature_c: Option<u8>,
    pub read_errors_corrected: Option<u64>,
    pub read_errors_uncorrected: Option<u64>,
    pub write_errors_corrected: Option<u64>,
    pub write_errors_uncorrected: Option<u64>,
    pub non_medium_errors: Option<u64>,
    pub start_stop_cycles: Option<u32>,
    pub max_start_stop_cycles: Option<u32>,
    pub background_scan_errors: Option<u32>,
    pub failure_prediction: Option<FailurePrediction>,
    pub sas_phy_errors: Vec<SasPhyError>,
    pub grown_defects: Option<u32>,
}

#[derive(Debug)]
pub struct FailurePrediction {
    pub failure_predicted: bool,
    pub sense_key: u8,
    pub asc: u8,
    pub ascq: u8,
}
```

### F.3 Native SG_IO Backend

```rust
use std::os::unix::io::AsRawFd;
use std::fs::OpenOptions;

pub struct NativeSgBackend {
    sg_path: std::path::PathBuf,  // e.g. /dev/sg0
}

// ⚠️ sg_io_hdr_t ต้องเป็น #[repr(C)] เสมอ — ABI compatibility กับ kernel
#[repr(C)]
struct SgIoHdr {
    interface_id: libc::c_int,         // [i] 'S'
    dxfer_direction: libc::c_int,      // [i]
    cmd_len: libc::c_uchar,            // [i]
    mx_sb_len: libc::c_uchar,          // [i]
    iovec_count: libc::c_ushort,       // [i] = 0 (no scatter-gather)
    dxfer_len: libc::c_uint,           // [i]
    dxferp: *mut libc::c_void,         // [i, *io]
    cmdp: *const libc::c_uchar,        // [i, *i]
    sbp: *mut libc::c_uchar,           // [i, *o]
    timeout: libc::c_uint,             // [i] milliseconds
    flags: libc::c_uint,               // [i]
    pack_id: libc::c_int,              // [i->o]
    usr_ptr: *mut libc::c_void,        // [i->o]
    status: libc::c_uchar,             // [o] SCSI status
    masked_status: libc::c_uchar,      // [o] deprecated
    msg_status: libc::c_uchar,         // [o] deprecated
    sb_len_wr: libc::c_uchar,          // [o] actual sense bytes written
    host_status: libc::c_ushort,       // [o] DID_* error
    driver_status: libc::c_ushort,     // [o] DRIVER_* error
    resid: libc::c_int,                // [o]
    duration: libc::c_uint,            // [o] ms elapsed
    info: libc::c_uint,                // [o] SG_INFO_*
}

const SG_IO: libc::c_ulong = 0x2285;  // from <scsi/sg.h>
const SG_DXFER_NONE: libc::c_int = -1;
const SG_DXFER_TO_DEV: libc::c_int = -2;
const SG_DXFER_FROM_DEV: libc::c_int = -3;
const SG_INFO_OK_MASK: libc::c_uint = 0x1;
const SG_INFO_OK: libc::c_uint = 0x0;
```

### F.4 Endianness และ Big-Endian SCSI

**[SCSI standard — documented]** SCSI protocol ใช้ **big-endian (network byte order)** สำหรับ multi-byte fields ทั้งหมดใน CDB และ response data Rust ต้องแปลงอย่างชัดเจน:[^15]

```rust
// อ่าน 2-byte big-endian field จาก response buffer
fn be16(buf: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([buf[offset], buf[offset + 1]])
}

fn be32(buf: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(buf[offset..offset + 4].try_into().unwrap())
}

fn be64(buf: &[u8], offset: usize) -> u64 {
    u64::from_be_bytes(buf[offset..offset + 8].try_into().unwrap())
}

// CDB construction: ตัวอย่าง LOG SENSE
fn build_log_sense_cdb(page_code: u8, subpage_code: u8, alloc_len: u16) -> [u8; 10] {
    [
        0x4D,                              // opcode: LOG SENSE
        0x00,                              // SP=0, PPC=0
        (0b01 << 6) | (page_code & 0x3F), // PC=01 (current), page code
        subpage_code,                      // subpage code
        0x00,                              // reserved
        0x00, 0x00,                        // parameter pointer = 0
        (alloc_len >> 8) as u8,            // allocation length MSB
        (alloc_len & 0xFF) as u8,          // allocation length LSB
        0x00,                              // control
    ]
}
```

***

## G. Safe Rust Code Skeleton สำหรับ Read-Only SG_IO

```rust
use std::time::Duration;
use std::os::unix::io::AsRawFd;

/// Execute a read-only SCSI command via SG_IO ioctl on /dev/sgX
/// Safety: Only called with commands in the O_RDONLY-permitted list
pub fn sg_execute_read(
    sg_path: &std::path::Path,
    cdb: &[u8],
    timeout: Duration,
    max_response_bytes: usize,
) -> Result<ScsiResponse, ScsiError> {
    // 1. Open device O_RDONLY | O_NONBLOCK (safest for read-only commands)
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(sg_path)
        .map_err(|e| ScsiError::OpenFailed(e))?;

    let fd = file.as_raw_fd();

    // 2. Allocate aligned buffers
    // ⚠️ sense_buf must be at least 32 bytes per SCSI spec minimum
    let mut sense_buf = vec![0u8; 64];
    let mut data_buf = vec![0u8; max_response_bytes];

    // 3. Build sg_io_hdr_t — MUST zero-initialize first
    // Safety: SgIoHdr is repr(C), all fields have valid zero values as defaults
    let mut io_hdr = unsafe { std::mem::zeroed::<SgIoHdr>() };

    io_hdr.interface_id = b'S' as libc::c_int;          // required sentinel
    io_hdr.dxfer_direction = SG_DXFER_FROM_DEV;
    io_hdr.cmd_len = cdb.len() as libc::c_uchar;
    io_hdr.mx_sb_len = sense_buf.len() as libc::c_uchar;
    io_hdr.dxfer_len = data_buf.len() as libc::c_uint;
    io_hdr.dxferp = data_buf.as_mut_ptr() as *mut libc::c_void;
    io_hdr.cmdp = cdb.as_ptr();
    io_hdr.sbp = sense_buf.as_mut_ptr();
    io_hdr.timeout = timeout.as_millis().min(u32::MAX as u128) as libc::c_uint;

    // 4. Issue ioctl
    // Safety: fd is valid, io_hdr and its pointed-to buffers are valid for the
    // duration of the ioctl call (buffers are on stack, io_hdr borrows them)
    let ret = unsafe { libc::ioctl(fd, SG_IO, &mut io_hdr as *mut SgIoHdr) };

    if ret < 0 {
        return Err(ScsiError::IoctlFailed(
            std::io::Error::last_os_error()
        ));
    }

    // 5. Check SG_INFO_OK — quick overall error check
    if (io_hdr.info & SG_INFO_OK_MASK) != SG_INFO_OK {
        // Some error occurred — inspect status fields
    }

    // 6. Collect response
    let sense_len = io_hdr.sb_len_wr as usize;
    let data_len = (max_response_bytes as i32 - io_hdr.resid).max(0) as usize;

    Ok(ScsiResponse {
        scsi_status: io_hdr.status,
        host_status: io_hdr.host_status,
        driver_status: io_hdr.driver_status,
        sense_data: sense_buf[..sense_len].to_vec(),
        data: data_buf[..data_len].to_vec(),
        duration_ms: io_hdr.duration,
        resid: io_hdr.resid,
    })
}

// --- LOG SENSE page 0x0D: Temperature parser ---
pub fn parse_temperature_page(data: &[u8]) -> Option<(u8, Option<u8>)> {
    // Page structure: byte 0 = page code, byte 1 = subpage/reserved,
    //                 byte 2-3 = page length, then parameter records
    if data.len() < 4 { return None; }

    let page_len = be16(data, 2) as usize;
    let mut offset = 4usize;
    let mut current_temp: Option<u8> = None;
    let mut ref_temp: Option<u8> = None;

    while offset + 4 <= (4 + page_len).min(data.len()) {
        let param_code = be16(data, offset);
        let param_len = data[offset + 3] as usize;
        if offset + 4 + param_len > data.len() { break; }

        match param_code {
            0x0000 => {
                // Current temperature: byte offset 4+1 = value (0xFF = unknown)
                if param_len >= 2 {
                    let t = data[offset + 5];
                    if t != 0xFF { current_temp = Some(t); }
                }
            }
            0x0001 => {
                // Reference temperature (max)
                if param_len >= 2 {
                    let t = data[offset + 5];
                    if t != 0xFF { ref_temp = Some(t); }
                }
            }
            _ => {}
        }
        offset += 4 + param_len;
    }

    current_temp.map(|t| (t, ref_temp))
}

// --- LOG SENSE page 0x02/0x03: Error counter parser ---
pub fn parse_error_counter_page(data: &[u8]) -> ErrorCounters {
    let mut counters = ErrorCounters::default();
    if data.len() < 4 { return counters; }

    let page_len = be16(data, 2) as usize;
    let mut offset = 4usize;

    while offset + 4 <= (4 + page_len).min(data.len()) {
        let param_code = be16(data, offset);
        let param_len = data[offset + 3] as usize;
        if offset + 4 + param_len > data.len() { break; }

        let value = match param_len {
            1 => data[offset + 4] as u64,
            2 => be16(data, offset + 4) as u64,
            4 => be32(data, offset + 4) as u64,
            8 => be64(data, offset + 4),
            _ => 0,
        };

        match param_code {
            0x0002 => counters.total_rewrites_rereads = Some(value),
            0x0003 => counters.total_errors_corrected = Some(value),
            0x0004 => counters.total_correction_algorithm_invocations = Some(value),
            0x0005 => counters.total_bytes_processed = Some(value),
            0x0006 => counters.total_uncorrected_errors = Some(value),
            _ => {}
        }
        offset += 4 + param_len;
    }
    counters
}

// --- LOG SENSE page 0x2F: Informational Exceptions parser ---
pub fn parse_informational_exceptions(data: &[u8]) -> Option<FailurePrediction> {
    if data.len() < 4 { return None; }

    let page_len = be16(data, 2) as usize;
    if 4 + page_len < 8 { return None; }

    // Parameter code 0x0000: informational exception general
    let param_code = be16(data, 4);
    if param_code != 0x0000 { return None; }
    let param_len = data[^7] as usize;
    if param_len < 3 || 8 + param_len > data.len() { return None; }

    let sense_key = data[^8];
    let asc = data[^9];
    let ascq = data[^10];

    // sense_key != 0 → failure predicted
    Some(FailurePrediction {
        failure_predicted: sense_key != 0,
        sense_key,
        asc,
        ascq,
    })
}
```

> **[ข้อเสนอแนะผู้วิจัย]** ห้ามเก็บ `*mut c_void` pointers ไว้ใน struct ข้ามขอบเขต call เพราะ `sg_io_hdr` pointers ชี้ไปยัง local buffers — ต้องให้ buffer มีชีวิตอยู่ตลอดขณะที่ ioctl กำลังรัน (single synchronous call ด้านบนปลอดภัยเพราะ ioctl blocks จนเสร็จ)

***

## H. Sense/Error Decoding Strategy

### H.1 Sense Data Format Detection

**[SPC-4 standard — documented]** sense data error code byte ระบุ format:[^16]

```rust
fn detect_sense_format(sense: &[u8]) -> SenseFormat {
    match sense.get(0).copied().map(|b| b & 0x7F) {
        Some(0x70) | Some(0x71) => SenseFormat::Fixed,       // error code 70h/71h
        Some(0x72) | Some(0x73) => SenseFormat::Descriptor,  // error code 72h/73h
        _ => SenseFormat::Unknown,
    }
}
```

### H.2 Fixed-Format Sense Data Layout

```
Byte 0:  [Valid|Error code 70h/71h]
Byte 1:  Segment number (obsolete)
Byte 2:  [FileMark|EOM|ILI|Reserved|Sense Key (4 bits)]
Byte 3-6: Information (valid only if Valid bit = 1)
Byte 7:  Additional sense length
Byte 8-11: Command-specific information
Byte 12: Additional Sense Code (ASC)
Byte 13: Additional Sense Code Qualifier (ASCQ)
Byte 14: Field Replaceable Unit code
Byte 15-17: Sense Key Specific
```

### H.3 Key Sense Keys สำหรับ Health Monitoring

| Sense Key | Hex | ความหมาย | Action |
|-----------|-----|---------|--------|
| NO SENSE | 0x0 | No error | ปกติ |
| RECOVERED ERROR | 0x1 | Soft error, corrected | Log เท่านั้น |
| NOT READY | 0x2 | Device not ready | Retry หลัง delay |
| MEDIUM ERROR | 0x3 | Hard media error | Log + alert |
| HARDWARE ERROR | 0x4 | Hardware failure | Log + alert |
| ILLEGAL REQUEST | 0x5 | Unsupported opcode/page | Log + skip this command |
| UNIT ATTENTION | 0x6 | State change | Re-issue command once |
| ABORTED COMMAND | 0xB | Command aborted | Retry once |

### H.4 สำคัญมาก: ASC/ASCQ สำหรับ Unsupported Page

```
ASC=0x24, ASCQ=0x00: "INVALID FIELD IN CDB" — log page อาจไม่ supported
ASC=0x20, ASCQ=0x00: "INVALID COMMAND OPERATION CODE"
ASC=0x26, ASCQ=0x00: "INVALID FIELD IN PARAMETER LIST"
ASC=0x3A, ASCQ=0x00: "MEDIUM NOT PRESENT"
ASC=0x29, ASCQ=0x00: "POWER ON, RESET, OR BUS DEVICE RESET OCCURRED" (Unit Attention)
```

```rust
fn handle_check_condition(sense: &SenseData) -> SenseAction {
    match (sense.sense_key, sense.asc, sense.ascq) {
        (0x5, 0x24, 0x00) => SenseAction::SkipUnsupported,  // illegal field in CDB
        (0x5, 0x20, 0x00) => SenseAction::SkipUnsupported,  // invalid opcode
        (0x6, _, _) => SenseAction::RetryOnce,               // Unit Attention
        (0x2, _, _) => SenseAction::RetryAfterDelay(std::time::Duration::from_secs(5)),
        (0x1, _, _) => SenseAction::LogAndContinue,          // Recovered error
        (0x3, _, _) | (0x4, _, _) => SenseAction::Alert,    // Media/Hardware error
        (0xB, _, _) => SenseAction::RetryOnce,              // Aborted command
        _ => SenseAction::LogAndContinue,
    }
}
```

***

## I. SAS vs SAT vs Controller-Hidden Decision Tree

```
START: Found block device /dev/sdX
         │
         ▼
1. Check sysfs: /sys/block/sdX/device/scsi_generic/
         │
         ├── Not present (NVMe, virtio, etc.)
         │         └── Use NVMe admin commands or sysfs health ← NOT SAS path
         │
         └── Present: sg device exists → open /dev/sgN
                   │
                   ▼
2. Send INQUIRY (standard, page 0x00)
         │
         ├── EPERM or EACCES → Permission barrier
         │         └── Need privilege escalation (see Section J)
         │
         ├── ENOENT or host_status != 0
         │         └── Transport error — controller blocking passthrough
         │                   └── Fallback: smartctl or sysfs metadata only
         │
         └── Success: parse Peripheral Device Type (byte 0 bits 4-0)
                   │
                   ├── 0x00 (Direct-access, disk)
                   │         │
                   │         ▼
                   │  3. Send INQUIRY VPD page 0x89 (ATA Information)
                   │         │
                   │         ├── CHECK CONDITION (ASC=0x24: Illegal field)
                   │         │         → Native SAS/SCSI disk ✅
                   │         │           Proceed with LOG SENSE health commands
                   │         │
                   │         └── Success: ATA IDENTIFY data present
                   │                   → SATA disk behind SAT layer ⚠️
                   │                     SCSI health data may be partial
                   │                     ATA SMART available via ATA PASS-THROUGH(16)
                   │
                   └── Other device types (tape, scanner, etc.)
                             └── Different command set — out of scope
```

### I.1 ตรวจสอบเพิ่มเติม: VPD 0xB1 Rotation Rate

```rust
// Block Device Characteristics VPD page (0xB1)
// Bytes 4-5: MEDIUM ROTATION RATE
// 0x0000 = not reported
// 0x0001 = non-rotating medium (SSD)
// 0x0401-0xFFFE = nominal RPM (e.g. 0x1C20 = 7200 RPM)
fn detect_device_type_from_vpd_b1(data: &[u8]) -> DeviceRotationType {
    if data.len() < 6 { return DeviceRotationType::Unknown; }
    match be16(data, 4) {
        0x0001 => DeviceRotationType::SolidState,
        0x0401..=0xFFFE => DeviceRotationType::Rotational(be16(data, 4)),
        _ => DeviceRotationType::Unknown,
    }
}
```

### I.2 Hardware RAID / Controller-Hidden Device

บาง hardware RAID controller (HP SmartArray, Dell PERC, LSI MegaRAID) **ไม่ forward SCSI passthrough** ไปยัง physical disk:
- `ioctl(SG_IO)` อาจคืน `EINVAL` หรือ `host_status = DID_NO_CONNECT`
- smartmontools ใช้ controller-specific ioctl (CCISS, megaraid) ที่ต้องการ proprietary interface
- **[ข้อเสนอแนะผู้วิจัย]** สำหรับ hardware RAID: ตรวจ `host_status` แล้ว fallback ไปยัง smartctl พร้อม device type flag (`-d megaraid,N`)

***

## J. Security and Privilege Model

### J.1 Minimum Permission Requirements

| Command Set | Device | Minimum Permission | Note |
|-------------|--------|-------------------|------|
| INQUIRY, LOG SENSE, MODE SENSE | `/dev/sg*` | File read permission (`disk` group) | No CAP needed[^2] |
| INQUIRY, LOG SENSE | `/dev/sdX` | File read permission | SG_IO on block device[^1] |
| READ DEFECT DATA | `/dev/sgX` | `O_RDWR` (sg driver) | สามารถใช้ `/dev/sdX` O_RDONLY แทน |
| ATA PASS-THROUGH | `/dev/sgX` | `CAP_SYS_RAWIO` | ATA commands ไม่อยู่ใน safe list |
| Vendor-specific commands | `/dev/sg*` | `CAP_SYS_RAWIO` | อันตราย — ไม่ใช้ใน monitoring |

### J.2 Privilege Separation Architecture

**[ข้อเสนอแนะผู้วิจัย]** สำหรับ production: ใช้ privilege separation daemon pattern:

```
┌─────────────────────────────────────────────────────────┐
│                  TUI / Frontend Process                  │
│                  (non-root, no CAP)                      │
│                                                          │
│   Requests: { device: "sda", command: "temperature" }   │
└────────────────────┬────────────────────────────────────┘
                     │ Unix domain socket (JSON/msgpack)
                     │
┌────────────────────▼────────────────────────────────────┐
│              disk-health-daemon                          │
│  (root at start, drops to disk group after opening fds) │
│  Or: setuid binary with cap_sys_rawio+p only            │
│                                                          │
│   - Opens /dev/sgX file descriptors at startup          │
│   - Validates only read-only commands                   │
│   - Sends SG_IO ioctl                                   │
│   - Returns parsed health data (no raw SCSI data)       │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼ SG_IO ioctl → /dev/sg0, /dev/sg1, ...
```

### J.3 setcap / Capability Approach

```bash
# Grant only CAP_SYS_RAWIO to monitoring binary (ไม่ต้องเป็น root)
setcap cap_sys_rawio+ep /usr/local/bin/disk-monitor

# หรือ: ใช้ udev rule ให้ disk group สามารถ read /dev/sg* ได้
# /etc/udev/rules.d/99-scsi-disk.rules
KERNEL=="sg*", GROUP="disk", MODE="0640"
```

### J.4 ⚠️ ข้อควรระวัง (documented risks)

- **CAP_SYS_RAWIO** bypasses opcode sniffing ทั้งหมด — process สามารถส่ง command ใด ๆ รวมถึง FORMAT, WRITE ได้[^1]
- **O_EXCL** บน sg device จะ block application อื่นที่ต้องการใช้ device เดียวกัน — ไม่ควรใช้ใน monitoring
- ห้ามเปิด block device (`/dev/sdX`) ด้วย `O_RDWR` เพื่อใช้ SG_IO ถ้าไม่จำเป็น — อาจ interfere กับ filesystem driver[^1]

***

## K. Test and Validation Plan

### K.1 Unit Tests (ไม่ต้องมี Hardware)

**[ข้อเสนอแนะผู้วิจัย]** สร้าง mock `ScsiTransport` trait เพื่อ inject binary SCSI responses:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockTransport {
        responses: std::collections::HashMap<u8, Vec<u8>>,  // opcode → response bytes
    }

    impl ScsiTransport for MockTransport {
        fn execute(&self, cdb: &[u8], ...) -> Result<ScsiResponse, ScsiError> {
            let opcode = cdb;
            let page_code = if opcode == 0x4D { cdb[^2] & 0x3F } else { 0 };
            
            // look up fixture response by opcode + page code
            let data = self.responses.get(&page_code)
                .cloned()
                .unwrap_or_default();
            
            Ok(ScsiResponse {
                scsi_status: 0x00,  // GOOD
                data,
                ..Default::default()
            })
        }
    }

    #[test]
    fn test_temperature_parse() {
        // Real binary response from sg_logs -p 0x0d /dev/sg0 --raw
        // Captured from hardware and stored as fixture
        let raw = include_bytes!("../../fixtures/log_page_0d_seagate_exos.bin");
        let (temp, ref_temp) = parse_temperature_page(raw).unwrap();
        assert!(temp > 10 && temp < 70);
        assert!(ref_temp.map_or(true, |r| r > temp));
    }

    #[test]
    fn test_sense_illegal_request_handled() {
        let sense = vec![0x70, 0x00, 0x05, 0,0,0,0, 0x0a, 0,0,0,0, 0x24, 0x00, 0,0,0,0];
        let decoded = decode_sense(&sense);
        assert_eq!(decoded.sense_key, 0x5);
        assert_eq!(decoded.asc, 0x24);
        assert!(matches!(handle_check_condition(&decoded), SenseAction::SkipUnsupported));
    }
}
```

### K.2 Fixture Capture จาก Hardware จริง

ใช้ `sg_logs` เพื่อ capture binary responses สำหรับ test fixtures:

```bash
# Capture raw binary response สำหรับ fixture
sg_logs --page=0x0d --raw /dev/sg0 > fixtures/log_page_0d_seagate.bin
sg_logs --page=0x2f --raw /dev/sg0 > fixtures/log_page_2f_seagate.bin
sg_vpd --page=0x83 --raw /dev/sg0 > fixtures/vpd_83_sas_id.bin

# ตรวจสอบ human-readable output สำหรับ expected values
sg_logs --page=0x0d /dev/sg0  # temperature
```

### K.3 Validation Oracle

เปรียบเทียบค่าจาก native implementation กับ smartctl และ sg3_utils:

```bash
# smartctl JSON output สำหรับ comparison
smartctl -A -j /dev/sda > smartctl_baseline.json
smartctl -l scsierrorlog /dev/sda -j >> smartctl_baseline.json

# sg3_utils baseline
sg_logs -a /dev/sg0 2>&1 > sg3_baseline.txt
sg_inq -v /dev/sg0 > sg3_inq.txt
```

### K.4 CI Without Hardware

- ทดสอบ parser ด้วย binary fixtures (captured จาก real hardware, stored in repo)
- ทดสอบ CDB builder ด้วย unit tests ว่า bytes ถูกต้อง
- ทดสอบ sense decoder ด้วย known sense data patterns
- ใช้ `cargo test` บน fixtures ทั้งหมด — ไม่ต้องมี disk จริง

### K.5 Hardware Integration Test Matrix

**[ต้องทดสอบกับ hardware จริง]** ต้องทดสอบบน:

| Device Type | Examples | ความเสี่ยงสูง |
|-------------|---------|-------------|
| Native SAS HDD | Seagate Exos, WD Gold | Log page 0x18 (SAS PHY) |
| SATA HDD via SAS HBA | SATA + SAS expander | VPD 0x89 detection |
| SATA via USB bridge | External drive | SG_IO อาจไม่ forward ทุก command |
| Behind hardware RAID | HP SmartArray, LSI | ioctl อาจ block |
| SAS SSD | Seagate Nytro | Log page format differences |

***

## L. Development Roadmap

### Phase 1: MVP (Complexity: Medium, Risk: Low)

| Task | Details | Risk |
|------|---------|------|
| SG_IO wrapper | `sg_execute_read()`, `#[repr(C)] SgIoHdr`, basic error handling | Low — well-documented ABI |
| Device discovery | `/sys/class/block` → `/sys/block/X/device/scsi_generic/` | Low |
| INQUIRY parser | Standard + VPD 0x00, 0x80, 0x83 | Low |
| Temperature LOG SENSE | Page 0x0D parser | Low |
| Error counter LOG SENSE | Pages 0x02, 0x03 | Low |
| Informational exceptions | Page 0x2F parser | Medium — vendor behavior varies |
| Sense data decoder | Fixed + descriptor format | Medium |
| Unit tests | Fixtures, mock transport | Low |

### Phase 2: Full SAS Health (Complexity: High, Risk: Medium)

| Task | Details | Risk |
|------|---------|------|
| SAT/ATA detection | VPD 0x89 check, fallback path | Medium |
| SAS PHY log (0x18) | Protocol-specific port page | High — SAS-version dependent |
| Background scan (0x15) | SBC-3 background scan results | Medium |
| Start-stop cycle (0x0E) | SPC-4 standard, straightforward | Low |
| VPD 0xB1 rotation rate | Block Device Characteristics | Low |
| READ DEFECT DATA | Grown defect list | Medium — requires O_RDWR on sg |
| Privilege separation daemon | Unix socket + CAP handling | High |
| Hardware integration tests | Multiple device types | High — hardware access needed |

### Phase 3: Production Hardening (Complexity: High, Risk: High)

| Task | Details | Risk |
|------|---------|------|
| smartctl fallback backend | Parse JSON output, auto-select | Medium |
| Hardware RAID detection | Detect controller type, graceful degradation | High |
| Tokio async wrapping | `tokio::task::spawn_blocking` สำหรับ SG_IO calls | Medium |
| Vendor-specific extensions | Seagate, WD specific log pages (optional) | High — no standard |
| udev hot-plug | Monitor device add/remove | Medium |

***

## M. Primary Source Links และ Source Code References

### M.1 Linux Kernel / Official Documentation

| ชื่อ | URL | หมายเหตุ |
|------|-----|---------|
| SG_IO ioctl differences (sg.danny.cz) | https://sg.danny.cz/sg/sg_io.html | **Primary** — สมบูรณ์ที่สุด[^1] |
| sg_io_hdr_t structure detail | https://tldp.org/HOWTO/SCSI-Generic-HOWTO/sg_io_hdr_t.html | Field-level doc[^3] |
| Linux SCSI Generic driver doc | https://docs.kernel.org/scsi/scsi-generic.html | Kernel docs[^14] |
| Linux UAPI sg.h (torvalds) | https://github.com/torvalds/linux/blob/master/include/scsi/sg.h | UAPI header[^4] |
| SG_IO ioctl open() flags table | https://sg.danny.cz/sg/p/sg_v3_ho/ch08.html | Permission table[^2] |
| SG_IO command permissions table | https://sg.danny.cz/sg/sg_io.html (Table 3) | O_RDONLY allowed list[^1] |
| bsg (block-layer sg) interface | https://lwn.net/Articles/174469/ | Alternative to /dev/sg*[^17] |

### M.2 sg3_utils Source Code (BSD-3-Clause library, GPL-2.0+ utilities)

| Source File | Path | ใช้เป็น reference สำหรับ |
|-------------|------|------------------------|
| sg_logs.c | `src/sg_logs.c` (doug-gilbert/sg3_utils) | LOG SENSE parsing[^18] |
| sg_pt_linux.c | `lib/sg_pt_linux.c` (hreinecke/sg3_utils) | SG_IO ioctl wrapper pattern[^19] |
| sg_lib.h | `include/sg_lib.h` | Sense data tables[^20] |
| sg_io_linux.h | `include/sg_io_linux.h` | DID_*, DRIVER_* constants[^21] |
| Main repository | https://github.com/doug-gilbert/sg3_utils | Official[^22] |

> **⚠️ Licensing Note**: sg3_utils **utilities** (`src/*.c`) ใช้ **GPL-2.0-or-later**; แต่ **library** (`lib/*.c`, `libsgutils2`) ใช้ **BSD-3-Clause** ดังนั้น:[^23][^24]
> - **ห้าม** copy GPL utility code เข้าไปใน MIT-licensed Rust project
> - **อนุญาต** ใช้ library concepts และ data format definitions จาก BSD-3-Clause libsgutils2 เป็น reference
> - **อนุญาต** ใช้ Linux UAPI headers (`<scsi/sg.h>`) ซึ่ง ไม่มี license restriction สำหรับ interface definitions (Linux syscall ABI)
> - **แนะนำ**: เขียน parser ใหม่ทั้งหมดโดยอ้างอิง T10 standard specifications โดยตรง ไม่ใช่จาก sg3_utils code

### M.3 T10 Standards (Public Drafts)

| ชื่อ | URL | เนื้อหา |
|------|-----|---------|
| SPC-5 (SCSI Primary Commands) | https://www.t10.org/members/w_spc5.htm | LOG SENSE, INQUIRY, sense data |
| SBC-3 (SCSI Block Commands) | https://www.t10.org/members/w_sbc3.htm | READ DEFECT, block limits |
| SAT (SCSI/ATA Translation) | https://www.t10.org/ftp/t10/document.04/04-196r0.pdf | ATA Information VPD, passthrough[^25] |
| SAS Device ID VPD | https://www.t10.org/ftp/t10/document.02/02-396r2.pdf | VPD 0x83 for SAS[^8] |
| Block Device Characteristics | https://www.t10.org/ftp/t10/document.07/07-203r0.pdf | VPD 0xB1, rotation rate[^10] |
| ATA Information VPD (SAT) | https://www.t10.org/ftp/t10/document.04/04-218r4.pdf | VPD 0x89 for SAT detection[^26] |

### M.4 Rust Crates

| Crate | Version | ใช้สำหรับ | หมายเหตุ |
|-------|---------|----------|---------|
| `libc` | 0.2+ | ioctl syscall, c_int types | แนะนำสำหรับ SG_IO (low-level) |
| `nix` | 0.29+ | `ioctl_readwrite!` macro | Type-safe wrapper[^27] |
| `rustix` | 0.38+ | Safe POSIX syscalls | Alternative, API เปลี่ยนบ่อยกว่า |
| `udev` | 0.8+ | Hot-plug monitoring | Optional[^28] |

***

## Appendix: Confidence Levels

| ข้อมูล | Confidence | แหล่งที่มา |
|--------|-----------|----------|
| `sg_io_hdr_t` field definitions | **HIGH** | Linux UAPI, sg.danny.cz docs |
| O_RDONLY permitted commands | **HIGH** | sg.danny.cz Table 3, kernel source |
| LOG SENSE page codes 0x0D, 0x02, 0x03, 0x2F | **HIGH** | SPC-4 standard, sg_logs man page |
| sysfs `/sys/block/sda/device/scsi_generic/sg0` path | **HIGH** | Verified via lsscsi source, IBM docs |
| VPD 0x89 → SAT/SATA detection | **HIGH** | T10 SAT document[^25] |
| SAS PHY log page 0x18 structure | **MEDIUM** | SAS-3 standard, limited public documentation |
| Vendor-specific log pages (Seagate 0x37, 0x3E) | **LOW** | Empirical, no public standard |
| Hardware RAID controller passthrough behavior | **LOW** | Controller-specific, must test on hardware |
| Polling intervals ที่แนะนำ | **MEDIUM** | ข้อเสนอแนะผู้วิจัย, อ้างอิง smartd defaults |
| SG_IO v4 interface stability | **LOW** | Experimental as of kernel 6.x[^5][^29] |

---

## References

1. [Linux SG_IO ioctl in the 2.6 series](https://sg.danny.cz/sg/sg_io.html)

2. [Chapter 8. Ioctl()s](http://sg.danny.cz/sg/p/sg_v3_ho/ch08.html)

3. [Chapter 6. The sg_io_hdr_t structure in detail](https://tldp.org/HOWTO/SCSI-Generic-HOWTO/sg_io_hdr_t.html)

4. [linux/include/scsi/sg.h at master · torvalds/linux - GitHub](https://github.com/torvalds/linux/blob/master/include/scsi/sg.h) - SG_DXFER_FROM_DEV with the additional property than during indirect IO the user buffer is copied int...

5. [Linux SG driver version 4.0](https://sg.danny.cz/sg/sg_v40.html)

6. [Chapter 6. The sg_io_hdr_t structure in detail](https://sg.danny.cz/sg/p/sg_v3_ho/ch06.html)

7. [SCSI Reliability Counters Test (LOGO)](https://learn.microsoft.com/hi-in/windows-hardware/test/hlk/testref/f4fe41e4-62b2-4c58-b131-8f2e51b0bbe3) - SCSI Reliability Counters Test (LOGO)

8. [02-396r2 SAS Device Identification VPD page requirements.fm](https://www.t10.org/ftp/t10/document.02/02-396r2.pdf)

9. [[PDF] 04-219r1 SAT SPC-3 ATA Information VPD page - T10.org](https://www.t10.org/ftp/t10/document.04/04-219r1.pdf)

10. [07-203r0 SBC-3 SPC-4 Block Device Characteristics VPD page and medium rotation rate field.fm](https://www.t10.org/ftp/t10/document.07/07-203r0.pdf)

11. [Correspondence between SCSI device entries in /sys and the disks in /dev](https://unix.stackexchange.com/questions/268429/correspondence-between-scsi-device-entries-in-sys-and-the-disks-in-dev) - Under the /sys/class/scsi_device folder I have the following: root@linux01:/sys/class/scsi_device # ...

12. [Finding SCSI generic device names - Laurence's Blog](https://blog.entek.org.uk/notes/2021/04/08/finding-scsi-generic-device-names.html) - Linux has a number of SCSI drivers, many devices are managed by their own driver as well as the sg g...

13. [Persistent SCSI device naming](https://www.ibm.com/docs/en/linux-on-systems?topic=naming-scsi-device) - With udev, you can define naming schemes that provide persistent SCSI device naming.

14. [SCSI Generic (sg) driver - The Linux Kernel documentation](https://docs.kernel.org/scsi/scsi-generic.html)

15. [SCSI Commands Reference Manual](https://www.seagate.com/staticfiles/support/disc/manuals/scsi/100293068a.pdf)

16. [SCSI Reference](https://docs.oracle.com/en/storage/storage-software/acsls/8.5/acsir/request-sense-03h.html)

17. [bsg, block layer sg](https://lwn.net/Articles/174469/)

18. [sg3_utils/src/sg_logs.c at master · hreinecke/sg3_utils](https://github.com/hreinecke/sg3_utils/blob/master/src/sg_logs.c) - Deprecated git-svn mirror for sg3_utils. Contribute to hreinecke/sg3_utils development by creating a...

19. [sg3_utils/lib/sg_pt_linux.c at master · hreinecke/sg3_utils](https://github.com/hreinecke/sg3_utils/blob/master/lib/sg_pt_linux.c) - Deprecated git-svn mirror for sg3_utils. Contribute to hreinecke/sg3_utils development by creating a...

20. [sg3_utils/include/sg_lib.h at master · hreinecke/sg3_utils](https://github.com/hreinecke/sg3_utils/blob/master/include/sg_lib.h) - Deprecated git-svn mirror for sg3_utils. Contribute to hreinecke/sg3_utils development by creating a...

21. [sg3_utils/include/sg_io_linux.h at master · hreinecke/sg3_utils](https://github.com/hreinecke/sg3_utils/blob/master/include/sg_io_linux.h) - Deprecated git-svn mirror for sg3_utils. Contribute to hreinecke/sg3_utils development by creating a...

22. [doug-gilbert/sg3_utils: Author's own git mirror of his ...](https://github.com/doug-gilbert/sg3_utils) - sg3_utils is a package of utilities originally written to send individual SCSI commands to storage d...

23. [sg3_utils/suse/sg3_utils.spec at master · hreinecke/sg3_utils](https://github.com/hreinecke/sg3_utils/blob/master/suse/sg3_utils.spec) - Deprecated git-svn mirror for sg3_utils. Contribute to hreinecke/sg3_utils development by creating a...

24. [sg3_utils-1.48-9.el10.s390x RPM](https://rpmfind.net/linux/RPM/centos-stream/10/baseos/s390x/sg3_utils-1.48-9.el10.s390x.html)

25. [[PDF] SCSI / ATA Translation Standard - t10.org](https://www.t10.org/ftp/t10/document.04/04-196r0.pdf)

26. [04-218r4 SAT SPC-3 INQUIRY contents.fm](https://www.t10.org/ftp/t10/document.04/04-218r4.pdf)

27. [Rust and ioctl // Goran Mekić](https://meka.rs/blog/2025/03/18/rust-and-ioctl/) - When I explore new programming language, I like to poke audio. Rust is a new language for me and I n...

28. [udev](https://lib.rs/crates/udev) - libudev bindings for Rust

29. [sg: add v4 interface](https://lwn.net/Articles/912125/)

