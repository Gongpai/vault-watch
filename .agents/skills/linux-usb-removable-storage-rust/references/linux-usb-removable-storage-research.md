# Linux USB & Removable Storage Monitoring ด้วย Rust — Technical Research Report

> **วิธีอ่านรายงาน:**
> - **[LINUX]** = kernel documentation, UAPI, sysfs-block ABI
> - **[SPEC]** = USB-IF, T10, T13, JEDEC eMMC/SD spec
> - **[OBSERVED]** = จากหลักฐาน community/smartmontools/vendor แต่ยังไม่มี primary spec อ้างอิง
> - **[INFERENCE]** = อนุมานจากหลักฐาน ยังต้องยืนยัน
> - **[ARCH-REC]** = คำแนะนำสถาปัตยกรรม

***

## A. Feasibility Matrix

| Storage Type | Detection (sysfs) | Health Data | Protocol | Confidence |
|---|---|---|---|---|
| USB Flash Drive | ✅ สมบูรณ์ | ⚠️ น้อยมาก (vendor SCSI LOG SENSE บางรุ่น) | USB BOT / UAS → SCSI | High |
| USB-to-SATA HDD/SSD (SAT bridge) | ✅ สมบูรณ์ | ✅ SAT APT-16 ถ้า bridge รองรับ | SG_IO → SAT | Medium (bridge-dependent) |
| USB-to-SATA (legacy bridge) | ✅ สมบูรณ์ | ⚠️ vendor-specific passthrough | JMicron/Cypress/Sunplus tunnels | Low–Medium |
| USB-to-NVMe (JMS583/ASM2362/RTL9210) | ✅ `/dev/sd*` เท่านั้น | ⚠️ vendor SNT tunnel (SCSI NVMe Translation) | Vendor-specific | Medium |
| SD/microSD ผ่าน USB card reader | ✅ สมบูรณ์ | ❌ ไม่มีมาตรฐาน SMART สำหรับ SD | — | Low |
| eMMC (native/embedded) | ✅ `/dev/mmcblk*` | ✅ JEDEC EXT_CSD ผ่าน `MMC_IOC_CMD` | MMC ioctl | High |
| Thunderbolt/USB4 NVMe enclosure | ⚠️ ขึ้นกับ firmware | ⚠️ อาจได้ `/dev/nvme*` หรือ `/dev/sd*` | NVMe หรือ USB BOT | **NEEDS HARDWARE** |
| Native SD reader (mmc subsystem) | ✅ `/dev/mmcblk*` | ❌ SD ไม่มี health registers ที่ standard | — | High (ไม่มีข้อมูล health) |
| Removable SCSI disk | ✅ สมบูรณ์ | ⚠️ SCSI LOG SENSE ถ้า device รองรับ | SG_IO | Medium |

**[OBSERVED]** USB-to-NVMe enclosure **ไม่** expose `/dev/nvme*` device — kernel เห็นเป็น SCSI block device (`/dev/sd*`) ผ่าน USB BOT/UAS driver เสมอ ไม่ว่าจะใช้ bridge chip รุ่นใด นี่เป็นข้อจำกัดพื้นฐานของ USB protocol ไม่ใช่ขีดจำกัดของ software[^1]

**[ARCH-REC]** ออกแบบ backend ให้ "degrade gracefully" — ทุก device type ต้องมี fallback path ที่ report `HealthCapability::NotSupported` แทนที่จะ crash หรือ hang

***

## B. Linux Sysfs Topology Mapping

### B.1 Block Device → USB Device Path

**[LINUX]** จาก sysfs rules และ kernel documentation:[^2][^3]

```
/sys/block/sdb
    └── device → symlink → ../../devices/pci0000:00/0000:00:1d.7/usb8/8-2/8-2:1.0/host9/target9:0:0/9:0:0:0

Path segments:
  8-2           = USB physical port (bus-port format)
  8-2:1.0       = USB interface (config:interface)
  host9         = SCSI host adapter (created by usb-storage or uas driver)
  target9:0:0   = SCSI target (host:channel:target)
  9:0:0:0       = SCSI device (host:channel:target:lun)
```

**[LINUX]** Sysfs path traversal เพื่อหา USB VID:PID:[^2]

```rust
// จาก /sys/block/sdb/device → symlink ไปยัง SCSI device
// traverse ขึ้น 6 ระดับเพื่อหา USB device node
fn find_usb_device_from_block(block_name: &str) -> Option<PathBuf> {
    let device_path = format!("/sys/block/{}/device", block_name);
    let resolved = std::fs::read_link(&device_path).ok()?;
    // traverse จาก SCSI LUN (9:0:0:0) → target → host → interface → USB device
    // หาก path มี "usb" component อยู่ → เป็น USB device
    let full = std::fs::canonicalize(
        format!("/sys/block/{}", block_name)
    ).ok()?;
    // ค้นหา pattern "usbN/N-P" ใน full path string
    // ...
}
```

### B.2 sysfs Files ที่ใช้งาน

**[LINUX]** จาก `Documentation/ABI/stable/sysfs-block`:[^4][^5]

| sysfs Path | Type | Description | Stable? |
|-----------|------|-------------|---------|
| `/sys/block/<dev>/removable` | `0`/`1` | Set from SCSI INQUIRY RMB bit[^6] | Stable |
| `/sys/block/<dev>/ro` | `0`/`1` | Read-only flag | Stable |
| `/sys/block/<dev>/size` | sectors | Capacity in 512-byte sectors | Stable |
| `/sys/block/<dev>/capability` | hex bitmask | `GENHD_FL_REMOVABLE=1`, `GENHD_FL_HIDDEN=4`[^4] | Stable |
| `/sys/block/<dev>/queue/rotational` | `0`/`1` | `0` = SSD/flash | Stable |
| `/sys/block/<dev>/queue/hw_sector_size` | bytes | Physical sector size | Stable |
| `/sys/block/<dev>/device/vendor` | string | SCSI vendor string (8 chars) | Stable |
| `/sys/block/<dev>/device/model` | string | SCSI product string (16 chars) | Stable |
| `/sys/block/<dev>/device/rev` | string | SCSI firmware rev | Stable |
| `/sys/block/<dev>/device/type` | int | SCSI peripheral type (0=disk) | Stable |
| `/sys/block/<dev>/device/power/runtime_status` | string | `active`/`suspended`/`idle` | Check before SG_IO |
| `/sys/block/<dev>/device/power/control` | string | `on`/`auto` | Read-only monitoring |
| `/sys/bus/usb/devices/<BUS-PORT>/idVendor` | hex | USB Vendor ID | Stable |
| `/sys/bus/usb/devices/<BUS-PORT>/idProduct` | hex | USB Product ID | Stable |
| `/sys/bus/usb/devices/<BUS-PORT>/serial` | string | USB device serial (optional) | May be empty |
| `/sys/bus/usb/devices/<BUS-PORT>/manufacturer` | string | USB manufacturer string | May be empty |
| `/sys/bus/usb/devices/<BUS-PORT>/product` | string | USB product string | May be empty |
| `/sys/bus/usb/devices/<BUS-PORT>/speed` | string | `480` / `5000` / `10000` Mbit/s | Stable |
| `/sys/bus/usb/devices/<BUS-PORT-IF>/driver` | symlink | `usb-storage` or `uas` | Stable |

**[LINUX]** ตรวจ UAS vs BOT: `readlink /sys/bus/usb/devices/<interface>/driver` — `uas` = UAS mode, `usb-storage` = BOT mode[^2]

**[LINUX]** USB port path (e.g., `8-2.3`) เป็น persistent physical location identifier ที่ไม่เปลี่ยนเมื่อ reconnect ต่าง platform อาจใช้รูปแบบ `busN-portN.portN`[^7]

### B.3 eMMC/MMC Topology

**[LINUX]** `/dev/mmcblkN` อยู่ภายใต้ `/sys/bus/mmc/devices/mmc0:XXXX/` ซึ่งมี:
- `type` = `MMC` (eMMC) หรือ `SD` (SD card)
- `name` = device model string
- `cid` = Card Identification Register (hex) — contains manufacturer ID, OEM, product name, revision, serial

***

## C. Backend-Selection Decision Tree

```
Block device: /dev/sdX, /dev/mmcblkX, /dev/sgX
         │
         ├─── /sys/block/<dev>/device path contains "mmc"?
         │         → eMMC/SD backend (MMC_IOC_CMD)
         │
         ├─── readlink /sys/block/<dev>/device path contains "nvme"?
         │         → Native NVMe backend (NVME_IOCTL_ADMIN_CMD)
         │         [ไม่ใช่ external NVMe — external จะมี "usb" ใน path]
         │
         ├─── path contains "usb"?
         │    │
         │    ├─── find USB VID:PID from sysfs
         │    │    │
         │    │    ├─── VID:PID match NVMe bridge quirk db?
         │    │    │    (JMS583=152d:0583, ASM2362=174c:2362, RTL9210=0bda:9210)
         │    │    │    → USB-NVMe SNT backend (vendor-specific)
         │    │    │
         │    │    ├─── VID:PID match SAT-capable SATA bridge?
         │    │    │    (probe: INQUIRY → VPD 0x89 → APT-16 capability probe)
         │    │    │    → SAT backend via SG_IO
         │    │    │
         │    │    ├─── VID:PID match legacy ATA bridge quirk db?
         │    │    │    (JM20329, CY7C68300B, SPIF215, PL2571...)
         │    │    │    → Legacy bridge backend (vendor tunnel)
         │    │    │
         │    │    └─── Unknown VID:PID
         │    │         → Generic SCSI backend (SCSI INQUIRY + LOG SENSE only)
         │    │
         │    └─── driver = "uas"?
         │         → Check UAS quirk first (some UAS bridges reject SAT APT-16)
         │         → Retry with APT-12 if APT-16 fails
         │
         └─── path contains "scsi" but NOT "usb"?
              │
              ├─── INQUIRY VPD 0x89 available? → SAT backend
              └─── No VPD 0x89 → Native SCSI backend
```

***

## D. Health-Passthrough Capability Matrix

**[OBSERVED]** ข้อมูลจาก smartmontools wiki (GPL reference, ห้าม copy code):[^8][^9]

| Bridge Chip | Type | ATA Pass-Through | NVMe Pass-Through | Protocol | SMART Confidence |
|---|---|---|---|---|---|
| ASMedia ASM1051/1053 | USB-SATA | SAT APT-16 | — | Standard SAT | High |
| ASMedia ASM2362 | USB-NVMe | — | SNT vendor-specific | `sntasmedia` | Medium |
| JMicron JM20329/20335 | USB-SATA | JMicron vendor ATACB | — | `usbjmicron` | Medium |
| JMicron JMS583 | USB-NVMe | — | SNT vendor-specific | `sntjmicron` | Medium |
| JMicron JMS561 | USB-SATA | SAT APT-16 (many models) | — | SAT or quirk | Medium[^10] |
| Realtek RTL9210/RTL9210B | USB-NVMe | — | SNT vendor-specific | `sntrealtek` | Medium[^8] |
| Cypress CY7C68300B/C | USB-SATA | ATACB vendor | — | `usbcypress` | Low (no 48-bit) |
| Cypress CY7C68300A | USB-SATA | ❌ ไม่รองรับ | — | — | None[^9] |
| Prolific PL2571/2771 | USB-SATA | Prolific vendor | — | `usbprolific` | Medium |
| Sunplus SPIF215/225 | USB-SATA | Sunplus vendor | — | `usbsunplus` | Medium |
| Initio | USB-SATA | SAT APT-16 | — | Standard SAT | High |
| Oxford Semiconductor | USB-SATA | SAT APT-16 | — | Standard SAT | High |
| USB Flash Drive | USB-UMS | ❌ | ❌ | None | None |
| Generic UMS HDD | USB-BOT | Depends on bridge | — | — | Unknown |

**[SPEC]** "SNT" = "SCSI NVMe Translation" — เป็น vendor-specific protocol ไม่มี official standard กำหนด (T10 ยังไม่มี standardized NVMe passthrough ผ่าน SCSI)[^9]

**[ARCH-REC]** ห้าม assume ว่า VID:PID เดียวกันทุกตัวรองรับ passthrough — bridge firmware version มีผล; ออกแบบ capability probe ที่ safe และ non-destructive

***

## E. USB Bridge Quirk Architecture

### E.1 Probe Order (Safe Capability Detection)

**[ARCH-REC]** ลำดับ probe ที่ปลอดภัย — แต่ละขั้นตอนต้องไม่ทำให้ device reset:

```
Step 1: VID:PID lookup ใน static quirk table
        → ถ้า match NVMe bridge → ข้ามไป SNT probe
        → ถ้า match legacy SATA bridge → ข้ามไป vendor ATACB probe

Step 2: SCSI STANDARD INQUIRY (opcode 0x12, 36 bytes)
        → ตรวจ VendorID, ProductID, version
        → ถ้า VendorID = "ATA     " (8 chars) → likely SAT via usb-storage

Step 3: INQUIRY VPD page 0x89 (ATA Information)
        → ถ้าสำเร็จ → confirmed SAT, IDENTIFY data อยู่ใน response
        → ถ้า ILLEGAL REQUEST → ไม่รองรับ VPD

Step 4 (SATA bridge ที่ไม่มี VPD 0x89):
        ลอง SAT ATA PASS-THROUGH (12) ด้วย IDENTIFY DEVICE
        → timeout = 10 seconds
        → ถ้าสำเร็จ → SAT mode
        → ถ้า CHECK CONDITION sk=ILLEGAL REQUEST → ลอง vendor quirk
        → ถ้า bridge reset หรือ timeout → log warning, mark NO_PASSTHROUGH

Step 5 (สำหรับ UAS device):
        ถ้า APT-16 ล้มเหลวด้วย "unsupported field in scsi command"
        → ลอง APT-12 แทน [cite: web:409]
```

**[OBSERVED]** Linux kernel มี quirk list สำหรับ USB storage devices ที่ reject SAT commands — kernel จะ reject APT-16 ก่อนถึง bridge โดยอัตโนมัติสำหรับ device ที่ blacklisted[^9]

### E.2 Quirk Database Architecture

**[ARCH-REC]** ออกแบบ runtime database ไม่ใช่ compile-time constant เพื่อให้ update ได้:

```rust
#[derive(Debug, Clone)]
pub struct BridgeQuirk {
    pub vid: u16,
    pub pid: u16,
    pub pid_mask: u16,         // 0xFFFF = exact match; 0xFF00 = family match
    pub protocol: BridgeProtocol,
    pub capabilities: BridgeCapabilities,
    pub notes: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BridgeProtocol {
    StandardSat,               // SAT APT-16 (modern SATA bridges)
    StandardSat12Only,         // SAT APT-12 only (older kernel/bridge)
    JmicronAtacb { ext_48bit: bool }, // JMicron SATA legacy
    CypressAtacb { cmd_byte: u8 },    // Cypress AT2LP
    ProlificAta,               // Prolific PL257x
    SunplusAta,                // Sunplus SPIF2xx
    SntJmicron,                // JMS583 NVMe tunnel
    SntAsmedia,                // ASM2362 NVMe tunnel
    SntRealtek,                // RTL9210/9211 NVMe tunnel
    NoPassthrough,             // Confirmed not supported
    Unknown,                   // Must probe
}

// ❗ GPL notice: ข้อมูล VID:PID ต้องรวบรวมจากแหล่ง public domain
// ห้าม copy จาก smartmontools drivedb.h (GPL-2.0)
// แหล่งที่ถูกต้อง: USB-IF database (public), vendor datasheets, hardware testing
static KNOWN_BRIDGES: &[BridgeQuirk] = &[
    BridgeQuirk {
        vid: 0x174c, pid: 0x2362, pid_mask: 0xFFFF,
        protocol: BridgeProtocol::SntAsmedia,
        capabilities: BridgeCapabilities { smart: false, nvme_snt: true, identify: true },
        notes: "ASMedia ASM2362 USB-NVMe bridge",
    },
    BridgeQuirk {
        vid: 0x152d, pid: 0x0583, pid_mask: 0xFFFF,
        protocol: BridgeProtocol::SntJmicron,
        capabilities: BridgeCapabilities { smart: false, nvme_snt: true, identify: true },
        notes: "JMicron JMS583 USB-NVMe bridge",
    },
    BridgeQuirk {
        vid: 0x0bda, pid: 0x9210, pid_mask: 0xFFF0, // RTL9210/9211
        protocol: BridgeProtocol::SntRealtek,
        capabilities: BridgeCapabilities { smart: false, nvme_snt: true, identify: true },
        notes: "Realtek RTL9210/9211 USB-NVMe bridge",
    },
    // ... 
];
```

**[OBSERVED]** smartmontools USB database อยู่ใน `drivedb.h` ซึ่งเป็น **GPL-2.0** — ห้าม copy entries เข้า non-GPL project สามารถ reference เป็น oracle สำหรับ testing ได้เท่านั้น VID:PID สามารถหาได้อิสระจาก USB-IF public database และ vendor datasheets

***

## F. SD/eMMC Health Interfaces

### F.1 eMMC EXT_CSD ผ่าน MMC_IOC_CMD

**[LINUX+SPEC]** จาก `include/uapi/linux/mmc/ioctl.h`:[^11][^12]

```c
struct mmc_ioc_cmd {
    int    write_flag;       // 0 = read, 1 = write
    int    is_acmd;          // 1 = precede with CMD55
    __u32  opcode;           // MMC command opcode
    __u32  arg;              // command argument
    __u32  response[^4];      // CMD response
    uint   flags;            // response type flags
    uint   blksz;            // block size (512 for EXT_CSD)
    uint   blocks;           // number of blocks
    uint   postsleep_min_us; // sleep after command (μs)
    uint   postsleep_max_us;
    uint   data_timeout_ns;
    uint   cmd_timeout_ms;
    __u64  data_ptr;         // ⚠️ pointer-width issue on 32-bit vs 64-bit
};
#define MMC_IOC_CMD _IOWR(MMC_BLOCK_MAJOR, 0, struct mmc_ioc_cmd)
#define MMC_IOC_MAX_BYTES (512L * 256)  // max per ioctl
#define MMC_IOC_MAX_CMDS 255
```

**[LINUX]** ต้องการ `CAP_SYS_RAWIO` และ user ต้องอยู่ใน `disk` group[^13]

**[SPEC]** `MMC_IOC_MULTI_CMD` (kernel ≥ 4.4) สำหรับส่ง multiple commands atomically[^14]

### F.2 Reading EXT_CSD (MMC CMD8 = SEND_EXT_CSD)

**[SPEC]** จาก JEDEC Standard JESD84-B51 (eMMC 5.1):[^15][^16][^17]

```rust
// opcode: MMC_SEND_EXT_CSD = 8
// arg: 0
// flags: MMC_RSP_SPI_R1 | MMC_RSP_R1 | MMC_CMD_ADTC = 0x0095
// blksz: 512, blocks: 1
const MMC_SEND_EXT_CSD: u32 = 8;
const EXT_CSD_SIZE: usize = 512;

// EXT_CSD field offsets (JEDEC JESD84-B51 — stable ABI):
const EXT_CSD_REV_OFFSET: usize = 192;             // eMMC revision
const EXT_CSD_PRE_EOL_INFO: usize = 267;            // Pre-EOL info
const EXT_CSD_DEVICE_LIFE_TIME_EST_TYP_A: usize = 268; // SLC life estimate
const EXT_CSD_DEVICE_LIFE_TIME_EST_TYP_B: usize = 269; // MLC life estimate
```

**[SPEC]** การ interpret EXT_CSD health fields:[^18][^16][^15]

| Field | Offset | Value | Meaning |
|-------|--------|-------|---------|
| `DEVICE_LIFE_TIME_EST_TYP_A` | 268 | 0x00 | Not defined |
| (SLC eraseblocks) | | 0x01 | 0–10% life used |
| | | 0x02 | 10–20% life used |
| | | … | 10% increments |
| | | 0x0A | 90–100% life used |
| | | 0x0B | ≥100% (exceeded estimate) |
| `DEVICE_LIFE_TIME_EST_TYP_B` | 269 | same encoding | MLC eraseblocks |
| `PRE_EOL_INFO` | 267 | 0x00 | Not defined |
| | | 0x01 | Normal (< 80% reserved blocks used) |
| | | 0x02 | Warning (≥ 80% reserved blocks used) |
| | | 0x03 | Urgent (≥ 90% reserved blocks used) |

**[LINUX]** อ่าน EXT_CSD ได้อีกทางผ่าน debugfs (ไม่ต้อง ioctl): `/sys/kernel/debug/mmc0/mmc0:XXXX/ext_csd` เป็น hex string — byte 268 = EXT_CSD_DEVICE_LIFE_TIME_EST_TYP_A (ตำแหน่งใน hex string = offset × 2) แต่ debugfs path **ไม่ใช่ stable ABI** — ใช้เฉพาะ development oracle[^17]

**[SPEC]** eMMC 5.0+ เพิ่ม field และ revision 8 (`EXT_CSD_REV = 0x08`) รองรับ Life Time Estimation — อ่าน `EXT_CSD_REV` ก่อน parse health fields[^19][^17]

### F.3 SD Card Health

**[SPEC/INFERENCE]** SD Association specification ไม่กำหนด standardized health/endurance registers ที่เทียบเท่า eMMC EXT_CSD — SD Card มี SD Status register (ACMD13) แต่ไม่มี life estimate fields ที่ standardized SD 3.0+ อาจมี `FULE` (Flash Used Life Estimation) ใน SD Security specification แต่ไม่ใช่ public spec

**[INFERENCE]** SD card health ผ่าน standard Linux interfaces ไม่มีข้อมูลที่น่าเชื่อถือ — ควรรายงานเป็น `HealthCapability::NotSupported` สำหรับ SD

***

## G. Safe Command Allowlist

### G.1 SG_IO Commands (USB SATA Bridges)

| Command | Opcode | Direction | Timeout | Safety | Notes |
|---------|--------|-----------|---------|--------|-------|
| STANDARD INQUIRY | 0x12 | Data-In | 5s | ✅ Safe | Standard probe |
| INQUIRY VPD 0x89 | 0x12 (EVPD=1) | Data-In | 5s | ✅ Safe | ATA Info page |
| INQUIRY VPD 0x80 | 0x12 (EVPD=1) | Data-In | 5s | ✅ Safe | Unit Serial Number |
| SAT APT-16 IDENTIFY | 0x85 + 0xEC | Data-In | 10s | ✅ Safe | ATA IDENTIFY DEVICE |
| SAT APT-16 SMART READ DATA | 0x85 + 0xB0/0xD0 | Data-In | 20s | ✅ Safe | |
| SAT APT-16 SMART RETURN STATUS | 0x85 + 0xB0/0xDA | None | 10s | ✅ Safe | |
| SCSI LOG SENSE | 0x4D | Data-In | 10s | ✅ Safe | page code varies |
| REQUEST SENSE | 0x03 | Data-In | 5s | ✅ Safe | Error recovery |

**⛔ ห้ามส่งใน production:**

| Command | เหตุผล |
|---------|-------|
| ATA SMART EXECUTE OFF-LINE | อาจทำให้ device pause / reset |
| FORMAT UNIT (0x04) | Destructive |
| WRITE BUFFER | Firmware update risk |
| START STOP UNIT ยกเว้น inquiry mode | อาจ spin up sleeping disk |

### G.2 MMC Commands (eMMC)

| Command | Opcode | write_flag | Safety | Notes |
|---------|--------|-----------|--------|-------|
| `SEND_EXT_CSD` | 8 | 0 (read) | ✅ Safe | Read 512-byte EXT_CSD |
| `SEND_STATUS` | 13 | 0 | ✅ Safe | Card status |
| `SEND_CSD` | 9 | 0 | ✅ Safe | Card Specific Data |
| `SEND_CID` | 10 | 0 | ✅ Safe | Card Identification |
| `MMC_SWITCH` / `SWITCH` | 6 | **1 (write)** | ⛔ Forbidden | Changes EXT_CSD — state-changing |
| `SANITIZE_START` | — | 1 | ⛔ Forbidden | Erases data |

***

## H. Power Management and Polling Policy

**[LINUX]** ตรวจสถานะ power ก่อน SG_IO:[^20]

```rust
pub fn check_device_runtime_status(block_dev: &str) -> PowerStatus {
    let path = format!("/sys/block/{}/device/power/runtime_status", block_dev);
    match std::fs::read_to_string(&path).as_deref() {
        Ok("suspended\n") | Ok("suspended") => PowerStatus::Suspended,
        Ok("active\n")    | Ok("active")    => PowerStatus::Active,
        Ok("idle\n")      | Ok("idle")      => PowerStatus::Idle,
        _ => PowerStatus::Unknown,  // ไม่มีไฟล์ = ไม่ใช้ runtime PM
    }
}
```

**[LINUX]** `power/control = "auto"` หมายความว่า kernel อาจ suspend device ได้ — ถ้า `runtime_status = "suspended"` และ `control = "auto"` **อย่าส่ง SG_IO** เพราะจะ wake disk และเพิ่ม power cycles โดยไม่จำเป็น[^21][^20]

**[LINUX]** `/sys/bus/usb/devices/<BUS-PORT>/power/autosuspend_delay_ms` = delay ก่อน suspend (milliseconds)[^21]

**[ARCH-REC]** Polling cadence แนะนำ:

| Device Type | Base Cadence | Trigger | Wake Disk? |
|---|---|---|---|
| USB HDD (spinning) | 30 min | udev event | ❌ ไม่ wake ถ้า suspended |
| USB SSD | 10 min | udev event | ✅ ไม่มีผลเรื่อง spin |
| eMMC | 60 min | startup only | N/A (always active) |
| USB Flash Drive | Startup only | connect/disconnect | ✅ |
| USB-NVMe enclosure | 10 min | udev event | ✅ |

**[ARCH-REC]** ก่อนส่ง SMART query ให้ตรวจ `runtime_status` — ถ้า suspended ให้ log และ skip (ไม่ใช่ error) — บันทึก timestamp ของ last successful reading แทน

***

## I. Rust Architecture and Traits

### I.1 Full Backend Trait Hierarchy

```rust
/// Hot-plug event
#[derive(Debug, Clone)]
pub enum StorageEvent {
    Added { devnode: PathBuf, syspath: PathBuf },
    Removed { syspath: PathBuf },
    Changed { syspath: PathBuf },
}

/// Capability probe result — ไม่ใช่ error ถ้า passthrough ไม่รองรับ
#[derive(Debug, Clone)]
pub struct StorageCapabilities {
    pub health_readable: bool,
    pub identity_readable: bool,
    pub protocol: StorageProtocol,
    pub passthrough_type: PassthroughType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StorageProtocol {
    NativeNvme,          // /dev/nvme*
    SataViaSat,          // /dev/sd* via SG_IO + SAT
    SataViaLegacyBridge, // /dev/sd* via vendor ATACB
    UsbNvmeSnt,          // /dev/sd* via SNT tunnel (JMS583/ASM2362/RTL9210)
    Emmc,                // /dev/mmcblk* via MMC_IOC_CMD
    ScsiGeneric,         // /dev/sd* SCSI only (no ATA passthrough)
    Unknown,
}

/// Topology information (sysfs-derived)
#[derive(Debug, Clone)]
pub struct StorageTopology {
    pub block_device: PathBuf,          // /dev/sdb
    pub syspath: PathBuf,               // /sys/block/sdb
    pub usb_vid_pid: Option<(u16, u16)>,
    pub usb_serial: Option<String>,
    pub usb_port_path: Option<String>,  // persistent: "8-2.3"
    pub removable: bool,
    pub runtime_status: PowerStatus,
    pub driver: String,                  // "usb-storage" or "uas"
}

/// Backend trait
pub trait RemovableStorageBackend: Send + Sync {
    fn probe_capabilities(&self) -> Result<StorageCapabilities, BackendError>;
    fn read_topology(&self) -> Result<StorageTopology, BackendError>;
    fn read_identity(&self) -> Result<StorageIdentity, BackendError>;
    fn read_health(&self) -> Result<HealthSnapshot, BackendError>;
}

/// udev monitor — เลือก tokio-udev crate (MIT)[cite: web:602]
pub struct UdevMonitor {
    // tokio-udev 0.9+ ใช้ Mio + Tokio integration
}

impl UdevMonitor {
    pub fn subscribe_block_events(&self) -> impl Stream<Item = StorageEvent>;
}
```

### I.2 SNT NVMe Tunnel (USB-NVMe bridges)

**[OBSERVED]** SNT (SCSI NVMe Translation) commands ใช้ vendor-specific CDB — โครงสร้างแตกต่างกันตาม bridge chip ตัวอย่าง JMicron JMS583:[^9]

```rust
// JMicron JMS583 NVMe passthrough — vendor-specific
// CDB = 0xC0 (vendor), CDB[^10] = length, NVMe command inline in CDB
// [INFERENCE] — structure ไม่มีใน public spec, มาจาก reverse engineering
// ต้องทดสอบกับ hardware จริงเท่านั้น
pub struct SntJmicronTransport { fd: File }

// ASMedia ASM2362 และ Realtek RTL9210 มี CDB format ต่างกัน
// [INFERENCE] ต้องอ้างอิง reverse-engineered protocol documentation
// หรือ สร้างโดย testing กับ vendor oracle
```

**[ARCH-REC]** ไม่แนะนำให้ implement SNT backends ก่อนมี hardware test fixtures — mark เป็น `unimplemented!()` ใน MVP

***

## J. Error Model

```rust
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Permission denied — requires CAP_SYS_RAWIO or disk group membership")]
    Permission(#[source] std::io::Error),

    #[error("No media present")]
    NoMedia,

    #[error("Device is suspended — skipped to avoid wake")]
    DeviceSuspended,

    #[error("Health passthrough not supported by this bridge (VID:{vid:04X} PID:{pid:04X})")]
    PassthroughUnsupported { vid: u16, pid: u16 },

    #[error("Bridge reset during command — possible unstable bridge")]
    BridgeReset,

    #[error("SCSI CHECK CONDITION: sense_key={sense_key:#x} asc={asc:#x} ascq={ascq:#x}")]
    CheckCondition { sense_key: u8, asc: u8, ascq: u8 },

    #[error("Command timeout after {ms}ms")]
    Timeout { ms: u32 },

    #[error("Device removed during operation")]
    DeviceGone,

    #[error("MMC ioctl failed")]
    MmcIoctl(#[source] std::io::Error),

    #[error("Parser error: {0}")]
    Parse(String),

    #[error("Bridge returned malformed/truncated response ({actual} of {expected} bytes)")]
    TruncatedResponse { expected: usize, actual: usize },
}
```

**[ARCH-REC]** `PassthroughUnsupported` ไม่ใช่ error ที่ต้อง log เป็น warning — เป็น normal path สำหรับ flash drive ทั่วไป

***

## K. Security Model

**[LINUX]** SG_IO บน `/dev/sdX` ต้องการ `CAP_SYS_RAWIO` `MMC_IOC_CMD` ต้องการ `CAP_SYS_RAWIO` และ user ใน `disk` group[^22][^13]

**[ARCH-REC]** สำหรับ monitoring daemon:
- Process สามารถเปิด device file ด้วย `O_RDONLY` แต่ ioctl ยังต้องการ capability
- แนะนำ: udev rule ให้ `/dev/nvme*` และ `/dev/mmcblk*` readable โดย `disk` group + `CAP_SYS_RAWIO` ใน systemd unit
- **ห้าม** expose API ที่รับ arbitrary CDB หรือ MMC opcode จาก caller

**[ARCH-REC]** USB bridge probe ต้องมี "blast protection":
- จำกัด probe attempts ต่อ device ไม่เกิน 3 ครั้ง ในช่วง 60 วินาที
- ถ้า bridge reset > 2 ครั้ง → mark `BRIDGE_UNSTABLE` และ skip passthrough ทั้งหมด
- เก็บ probe result ใน memory cache — ไม่ probe ซ้ำทุก poll cycle

***

## L. Fixture and Hardware Test Matrix

### L.1 Fixture Structure

```
fixtures/
├── usb_topology/
│   ├── sda_usb_sat_asmedia/
│   │   ├── sysfs_tree.json      # simulated sysfs paths+values
│   │   ├── identify.bin         # 512-byte IDENTIFY DEVICE response
│   │   └── smart_data.bin
│   ├── sda_jmicron_legacy/
│   ├── sdb_nvme_jms583/
│   │   └── snt_identify.bin     # vendor SNT IDENTIFY response
│   └── sdc_flash_drive/
│       └── inquiry.bin          # STANDARD INQUIRY only
├── emmc/
│   ├── ext_csd_healthy.bin      # 512-byte EXT_CSD, PRE_EOL=0x01
│   ├── ext_csd_warning.bin      # PRE_EOL=0x02
│   └── ext_csd_v57.bin          # eMMC 5.1 EXT_CSD_REV=0x08
└── error_cases/
    ├── bridge_reset.bin          # simulated premature EOF / ENODEV
    ├── illegal_request_sense.bin # CHECK CONDITION, ILLEGAL REQUEST
    └── truncated_smart.bin       # 256 bytes instead of 512
```

### L.2 Hardware Test Matrix

| Test Scenario | Device | Expected | Confidence |
|---|---|---|---|
| USB-SATA SAT (ASMedia) | ASMedia ASM105x enclosure | Full SMART via APT-16 | **NEEDS HARDWARE** |
| USB-SATA SAT (JMS561) | JMicron JMS561 enclosure | SAT or fallback[^10] | **NEEDS HARDWARE** |
| USB-NVMe (JMS583) | JMicron JMS583 enclosure | Identity via SNT | **NEEDS HARDWARE** |
| USB-NVMe (RTL9210B) | Realtek RTL9210B enclosure | Identity via SNT[^8] | **NEEDS HARDWARE** |
| USB Flash Drive | Generic USB 3.0 flash | No SMART, INQUIRY only | Easy to test |
| eMMC (healthy) | ARM SBC/embedded board | PRE_EOL=0x01 | Medium (need eMMC device) |
| eMMC (degraded) | Old embedded device | PRE_EOL ≥ 0x02 | Hard to reproduce |
| USB HDD (sleeping) | External HDD with autosuspend | Detect suspended, skip | Easy to test |
| Bridge reset | Cheap USB bridge | `BridgeReset` error, no retry storm | **NEEDS HARDWARE** |
| Hot-unplug during read | Any USB storage | `DeviceGone` error | Medium |
| No permission | Run without CAP_SYS_RAWIO | `Permission` error | Easy to test |
| UAS device | UAS-capable enclosure | APT-16 or fallback to APT-12 | **NEEDS HARDWARE** |
| Thunderbolt NVMe | TB3/4 enclosure | `/dev/nvme*` (if PCIe tunnel) or `/dev/sd*` | **NEEDS HARDWARE** |

***

## M. Implementation Roadmap

### MVP (3–4 Weeks)

**Week 1:** sysfs topology layer
- Block device → USB path traversal (read_link chain)
- USB VID:PID, speed, driver (usb-storage/uas) extraction
- removable, runtime_status, rotation flags
- udev integration ผ่าน `tokio-udev` crate (MIT license)[^23]

**Week 2:** eMMC health backend
- `MMC_IOC_CMD` ioctl wrapper (`repr(C)` + size assertion)
- `SEND_EXT_CSD` parser: EXT_CSD_REV + PRE_EOL_INFO + LIFE_TIME_EST_A/B
- Fixtures: 5 EXT_CSD binaries จาก hardware oracle

**Week 3:** USB-SATA SAT backend
- Capability probe sequence (INQUIRY → VPD 0x89 → APT-16 test)
- Static bridge quirk table (VID:PID lookup, แหล่งข้อมูล: USB-IF + hardware testing)
- UAS vs BOT detection + APT-16 vs APT-12 fallback

**Week 4:** Power management + error model
- `runtime_status` check ก่อนทุก SG_IO
- Bridge reset detection + blast protection (max 3 retries / 60s)
- Full error enum + fixture-based unit tests

### Production Hardening (3–4 Weeks)

**Week 5:** USB-NVMe SNT backends (JMS583 / ASM2362 / RTL9210)
- ต้องการ hardware fixtures ก่อน implement
- feature-gated, disabled by default ถ้าไม่มี hardware test

**Week 6:** Legacy bridge backends (JMicron ATACB, Cypress, Prolific, Sunplus)
- ใช้ smartmontools เป็น oracle (ห้าม copy GPL code)

**Week 7:** Hot-plug reconciliation
- udev event stream + periodic state reconciliation
- Device identity persistence (port path + VID:PID:serial)

**Week 8:** Fuzzing + security hardening
- Fuzz: `parse_ext_csd`, `parse_inquiry_response`, `parse_smart_data`
- MMC multi-command sequence testing
- Privilege-separation helper deployment

***

## N. Primary Source Links

| Source | Reference | License |
|--------|-----------|---------|
| Linux sysfs-block ABI | https://www.kernel.org/doc/Documentation/ABI/stable/sysfs-block [^5] | GPL (UAPI-compatible) |
| Linux block capability docs | https://docs.kernel.org/5.19/block/capability.html [^4] | GPL |
| Linux MMC UAPI ioctl.h | `include/uapi/linux/mmc/ioctl.h` (in kernel source) [^11][^12] | GPL with syscall exception |
| Linux mass-storage gadget docs | https://docs.kernel.org/usb/mass-storage.html [^24] | GPL |
| Linux sysfs rules | https://www.kernel.org/doc/html/v6.1/admin-guide/sysfs-rules.html [^3] | GPL |
| USB-IF public device database | https://www.usb.org/developers/docs/ | Public |
| smartmontools USB wiki (reference only) | https://www.smartmontools.org/wiki/USB [^9] | GPL-2.0 (reference only) |
| JMicron JMS561 quirk issue | https://github.com/smartmontools/smartmontools/issues/289 [^10] | GPL (reference only) |
| tokio-udev Rust crate | https://github.com/jeandudey/tokio-udev [^23] | MIT |
| eMMC EXT_CSD JEDEC reference | JESD84-B51 (JEDEC — free registration) [^15][^18][^16][^17] | Proprietary spec (free download) |
| Chromium mmc-utils (EXT_CSD read reference) | https://chromium.googlesource.com/chromiumos/third_party/mmc-utils/ [^14] | GPL-2.0 (reference only) |
| MMC_IOC_CMD permission requirements | https://stackoverflow.com/questions/58675866 [^13] | — |
| USB autosuspend sysfs | https://wiki.gentoo.org/wiki/USB_Power_Saving [^25] | — |
| UAS vs BOT technical article | https://www.devever.net/~hl/usbuas [^26] | — |
| RTL9210B enclosure review | https://dev.to/wixom/... [^8] | — |
| Linux block→USB path traversal | https://stackoverflow.com/questions/3493858 [^2] | — |

***

## สิ่งที่ยังต้องยืนยันด้วย Hardware

1. **[NEEDS HARDWARE]** Thunderbolt 3/4 NVMe enclosure — บาง firmware expose `/dev/nvme*` ผ่าน PCIe tunneling; บางตัว expose เพียง `/dev/sd*` ผ่าน USB BOT
2. **[NEEDS HARDWARE]** UAS SAT APT-16 failure pattern — kernel version ที่ block APT-16 สำหรับ specific VID:PID
3. **[NEEDS HARDWARE]** USB bridge reset behavior — timeout ที่เหมาะสมเพื่อไม่ trigger bus reset
4. **[NEEDS HARDWARE]** SNT NVMe tunnel commands — ทั้ง JMS583, ASM2362, RTL9210 CDB format ต้องยืนยัน
5. **[NEEDS HARDWARE]** eMMC EXT_CSD degraded value — ต้องใช้ device ที่มี PRE_EOL ≥ 0x02 จริง
6. **[INFERENCE NEEDS VALIDATION]** `GENHD_FL_REMOVABLE` behavior กับ USB HDD — บาง enclosure set removable=0 แม้จะเป็น external

---

## References

1. [Can an NVM device attached via USB adapter be forced to be /dev/nvm* instead of /dev/sd*?](https://askubuntu.com/questions/1321106/can-an-nvm-device-attached-via-usb-adapter-be-forced-to-be-dev-nvm-instead-of) - I have an M2 NVM storage device that I need to access using nvme-cli, but because the device is conn...

2. [Linux: How to map a blockdevice to a USB-device?](https://stackoverflow.com/questions/3493858/linux-how-to-map-a-blockdevice-to-a-usb-device) - if I plugin a USB memory stick, I see a new folder in /sys/bus/usb/devices ... thus a new USB-device...

3. [Rules on how to access information in sysfs¶](https://www.kernel.org/doc/html/v6.1/admin-guide/sysfs-rules.html)

4. [Generic Block Device Capability - The Linux Kernel documentation](https://docs.kernel.org/5.19/block/capability.html)

5. [sysfs-block - The Linux Kernel Archives](https://www.kernel.org/doc/Documentation/ABI/stable/sysfs-block)

6. [How to tell if a SCSI device is removable?](https://unix.stackexchange.com/questions/125961/how-to-tell-if-a-scsi-device-is-removable) - In DMESG I see: [sdb] Attached SCSI removable disk How does Linux decide what is removable and not r...

7. [How to associate logical to physical USB topology on Linux](https://gist.github.com/pbelskiy/341d35e407cb74e51c4da0a4318547a5) - How to associate logical to physical USB topology on Linux - usb-linux.md

8. [The Engineering Behind Choosing a Hard Drive Enclosure for SSD](https://dev.to/wixom/stop-bottlenecking-your-code-the-engineering-behind-choosing-a-hard-drive-enclosure-for-ssd-41dl) - If you've ever tried migrating a project with half a million tiny files in node_modules across...

9. [USB – smartmontools](https://www.smartmontools.org/wiki/USB) - The USB bridge provides an ATA or NVMe pass-through command. · This command is supported by smartmon...

10. [Add yet another JMS561 controller to the database (0x152d:0xa580)](https://github.com/smartmontools/smartmontools/issues/289) - Device ID 152d:a580 JMicron Technology Corp. / JMicron USA Technology Corp. JMS56x Series used by th...

11. [Linux Kernel: include/uapi/linux/mmc/ioctl.h Source File](https://docs.huihoo.com/doxygen/linux/kernel/3.7/include_2uapi_2linux_2mmc_2ioctl_8h_source.html)

12. [libc/kernel/uapi/linux/mmc/ioctl.h - platform/bionic - Git at Google](https://android.googlesource.com/platform/bionic/+/05d08e9/libc/kernel/uapi/linux/mmc/ioctl.h)

13. [What capabilities required for ioctl() on emmc on systemd?](https://stackoverflow.com/questions/58675866/what-capabilities-required-for-ioctl-on-emmc-on-systemd) - I want to run my program with systemd with a regular user ( non-root). This program uses ioctl() sys...

14. [mmc_cmds.c - chromiumos/third_party/mmc-utils](https://chromium.googlesource.com/chromiumos/third_party/mmc-utils/+/83106780683e0a6e5741e1511636aa00d37d96b0/mmc_cmds.c)

15. [EMMC](https://adoyle.me/Today-I-Learned/hardware/emmc.html) - 博观而约取，厚积而薄发。ADoyle 的碎片化知识笔记。

16. [Interpreting Mmc Health Data](https://docs.netgate.com/pfsense/en/latest/troubleshooting/disk-lifetime.html)

17. [如何获取emmc 的健康状态？ 原创 - CSDN博客](https://blog.csdn.net/AAlvin/article/details/152929602) - 文章浏览阅读252次。3、如果是需要在程序内读取则通过下面的代码进行查询。嵌入式设备如何获取emmc 的健康状态？1、首先挂载debug 文件系统。查询到的值对应下方的寿命状态。_linux查看emm...

18. [(e)MMC - NetModule OEM Linux Distribution's documentation!](https://netmodule-linux.readthedocs.io/en/latest/howto/mmc.html) - You can read your eMMC health status by using the mmc command and reading the EXT_CSD (extended card...

19. [eMMC (Linux) - Toradex Developer Center](https://developer.toradex.com/software/linux-resources/linux-features/emmc-linux/) - This script is used on device monitoring, so that eMMC health status is sent to the Torizon Cloud. ....

20. [smartd/smartclt option '-n standby' without effect for disks with enabled Linux kernel runtime power management · Issue #229 · smartmontools/smartmontools](https://github.com/smartmontools/smartmontools/issues/229) - The Linux kernel runtime power management of disk devices is primarily controlled by the parameters:...

21. [How to disable USB autosuspend on kernel 3.7.10 or above?](https://unix.stackexchange.com/questions/91027/how-to-disable-usb-autosuspend-on-kernel-3-7-10-or-above) - I've updated my HTPC from kernel 3.7.10 to 3.10.7 and it seems CONFIG_USB_SUSPEND is now gone from t...

22. [block: fail SCSI passthrough ioctls on partition devices - linux-rng](https://git.zx2c4.com/linux-rng/commit/block/scsi_ioctl.c?id=0bfc96cb77224736dfa35c3c555d37b3646ef35e)

23. [GitHub - jeandudey/tokio-udev: Asynchronous udev hotplug monitor using Tokio and Mio.](https://github.com/jeandudey/tokio-udev) - Asynchronous udev hotplug monitor using Tokio and Mio. - jeandudey/tokio-udev

24. [Mass Storage Gadget (MSG)](https://docs.kernel.org/usb/mass-storage.html)

25. [USB Power Saving](https://wiki.gentoo.org/wiki/USB_Power_Saving)

26. [USB Mass Storage and USB-Attached SCSI... are both SCSI - devever](https://www.devever.net/~hl/usbuas)

