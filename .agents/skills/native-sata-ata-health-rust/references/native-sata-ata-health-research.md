# Native Linux SATA/ATA Health Monitoring ด้วย Rust — Technical Research Report

> **วิธีอ่านรายงาน:** ข้อความที่ระบุว่า **[DOCUMENTED FACT]** มาจากมาตรฐาน T13/T10, Linux kernel UAPI หรือ official vendor documentation โดยตรง  
> ข้อความที่ระบุว่า **[INFERENCE]** คืองานวิเคราะห์ที่อนุมานจากหลักฐานเหล่านั้น  
> ข้อความที่ระบุว่า **[ARCHITECTURAL RECOMMENDATION]** เป็นข้อเสนอสถาปัตยกรรมจากผู้วิจัย ยังต้องทดสอบ hardware จริง

***

## A. Feasibility Verdict

**[DOCUMENTED FACT]** การสร้าง native Linux SATA/ATA health-monitoring backend โดยไม่เรียก `smartctl` หรือ `hdparm` เป็น runtime dependency เป็นไปได้อย่างสมบูรณ์ Linux kernel expose ATA passthrough ผ่าน `SG_IO` ioctl บน `/dev/sdX` และ `/dev/sgX` ซึ่งเป็น documented stable kernel UAPI SATA disk ที่ต่อผ่าน native AHCI/libata จะถูก Linux translate เป็น SCSI device และรับ SAT ATA PASS-THROUGH command ผ่าน `SG_IO` ได้โดยตรง[^1][^2]

**[INFERENCE]** ความซับซ้อนที่แท้จริงไม่ได้อยู่ที่ protocol layer แต่อยู่ที่ความหลากหลายของ vendor-specific SMART attributes และ USB bridge quirks ซึ่งไม่มี single ABI รองรับทั้งหมด — ต้องออกแบบ pluggable parser แทนการ hardcode ความหมายของ raw bytes ต่อ vendor

**ขอบเขตที่ทำได้แน่นอน:**
- Native SATA ผ่าน AHCI/libata: ✅ สมบูรณ์
- SAT ผ่าน SAS HBA: ✅ สมบูรณ์ (ขึ้นกับ HBA firmware)
- USB-to-SATA ผ่าน SAT: ✅ ส่วนใหญ่ (ขึ้นกับ bridge chip)
- Hardware RAID logical volume: ⚠️ ต้องใช้ controller-specific ioctl หรือ fallback เท่านั้น

***

## B. Recommended Backend Architecture

**[ARCHITECTURAL RECOMMENDATION]** แนะนำสถาปัตยกรรมแบบ layered ดังนี้:

```
┌─────────────────────────────────────────────────────┐
│              Public API / HealthSnapshot            │
├─────────────────────────────────────────────────────┤
│         DiskHealthBackend trait (dyn dispatch)      │
├──────────┬──────────┬──────────┬────────────────────┤
│  Native  │  SAT/SG  │USB Bridge│  RAID Passthrough  │
│  ATA     │  (0x85)  │ Quirks   │  (megaraid/hpsa)   │
├──────────┴──────────┴──────────┴────────────────────┤
│         Transport Layer (SgIoTransport trait)       │
├─────────────────────────────────────────────────────┤
│   Pure Parsers: IDENTIFY / SMART READ / LOG pages   │
└─────────────────────────────────────────────────────┘
```

**[ARCHITECTURAL RECOMMENDATION]** การแยก Parser ออกจาก Transport เป็นข้อบังคับ ไม่ใช่แค่ best practice — เหตุผลหลักคือ unit test parser โดยไม่ต้องมี disk จริง และ fuzz test binary responses ที่ malformed

***

## C. ATA/SAT Command Allowlist

ตารางนี้แสดงเฉพาะ **read-only commands** ที่ปลอดภัยสำหรับ health monitoring

| Command | Opcode (CMD) | Feature (0xB0) | Direction | Protocol | Permissions | Cadence | Notes |
|---------|-------------|----------------|-----------|----------|-------------|---------|-------|
| IDENTIFY DEVICE | 0xEC | — | Data-In | PIO | CAP_SYS_RAWIO | startup + cache | **[DOCUMENTED FACT]** ACS-5 §7.12 |
| SMART READ DATA | 0xB0 | 0xD0 | Data-In | PIO | CAP_SYS_RAWIO | ≥5 min | **[DOCUMENTED FACT]** ACS-5 §7.15.9 |
| SMART READ THRESHOLDS | 0xB0 | 0xD1 | Data-In | PIO | CAP_SYS_RAWIO | startup | **[DOCUMENTED FACT]** Obsolete since ATA-4[^3] แต่ยังรองรับบน drives ส่วนใหญ่ |
| SMART RETURN STATUS | 0xB0 | 0xDA | Non-Data | Non-data | CAP_SYS_RAWIO | ≥1 min | Status บน LBA_MID/HIGH registers |
| SMART READ LOG | 0xB0 | 0xD5 | Data-In | PIO | CAP_SYS_RAWIO | on demand | 28-bit, log address ใน LBA_LOW |
| READ LOG EXT (GPL) | 0x2F | — | Data-In | PIO/DMA | CAP_SYS_RAWIO | on demand | **[DOCUMENTED FACT]** 48-bit, ACS-3 §7.21 — ต้องใช้ SAT APT(16) |
| INQUIRY VPD 0x89 | SCSI 0x12 | — | Data-In | SCSI | CAP_SYS_RAWIO | startup | ATA Information VPD page[^4] |

**⛔ Commands ที่ห้าม automate ใน production:**

| Command | เหตุผล |
|---------|-------|
| SMART EXECUTE OFF-LINE IMMEDIATE (0xD4) | อาจทำให้ drive pause I/O ชั่วคราว หรือ reset หากมีข้อผิดพลาด |
| SECURITY ERASE UNIT | Destructive — ลบข้อมูลทั้งหมด |
| ANY WRITE* command | Read-only health monitoring ไม่ควร write state |

**[DOCUMENTED FACT]** `HDIO_DRIVE_CMD` และ `HDIO_DRIVE_TASKFILE` เป็น legacy ioctls ที่ `libata` driver ใน Linux ไม่รองรับ — `SG_IO` กับ SAT ATA PASS-THROUGH เป็น primary interface ที่ถูกต้อง[^5]

***

## D. IDENTIFY DEVICE Field Table

**[DOCUMENTED FACT]** IDENTIFY DEVICE response คือ 512 bytes = 256 words (little-endian, word-swapped strings)[^6]

| Word | Field | Bits | Notes | Stability |
|------|-------|------|-------|-----------|
| 0 | General Config | 15:0 | bit 15=0 → ATA; bit 6=fixed-disk | Stable |
| 10–19 | Serial Number | — | 20 bytes, ATA byte-swapped string | Stable |
| 23–26 | Firmware Rev | — | 8 bytes, ATA byte-swapped string | Stable |
| 27–46 | Model Number | — | 40 bytes, ATA byte-swapped string | Stable |
| 60–61 | Total User LBAs (28-bit) | — | little-endian 32-bit | Stable |
| 76 | SATA Capabilities | 15:0 | bit 1=SATA Gen1, bit 2=Gen2, bit 3=Gen3 | Stable |
| 80 | ATA Major Version | 15:0 | bit N → supports ATA-N (bit 7=ATA-7, etc.) | Stable |
| 83 | Command Sets Supported | 15:0 | bit 10=HPA, bit 3=APM, bit 14=must=1 | Stable |
| 87 | Command Sets Enabled | — | mirror of 83 (enabled subset) | Stable |
| 100–103 | Total User LBAs (48-bit) | — | 64-bit LBA[^6] | Stable |
| 106 | Physical/Logical Sector Size | 15:0 | bit 12=logical > 512B; bits 3:0 = ratio power | Stable |
| 108–111 | WWN | — | 64-bit World Wide Name | ACS-2+ |
| 217 | Nominal Rotation Rate | 15:0 | **0x0001 = SSD** (non-rotating); 7200 = 7200 RPM[^6] | ACS-2+ |
| 222 | Transport Major Version | — | bit 12=SATA, bits 3:0=rev | Stable |

**[DOCUMENTED FACT]** ATA string fields (words 10-19, 23-26, 27-46) เป็น byte-swapped: ทุก 2 bytes ต้อง swap กัน — bytes `[A][B]` ใน disk response = character `B` then `A` ในความหมายที่แท้จริง[^7]

**[DOCUMENTED FACT]** word 217 = 0x0001 บ่งบอก SSD (non-rotating media) ตาม ACS-2 specification[^8]

***

## E. SMART Data / Log Structures

### E.1 SMART READ DATA Response (512 bytes)

**[DOCUMENTED FACT]** จาก Micron TN-FD-22 ซึ่งสอดคล้องกับ ATA8-ACS2/ACS3 spec:[^9]

```
Offset  Length  Description
0       2       SMART structure version (vendor-specific)
2       12      Attribute entry #1   ─┐
2+12    12      Attribute entry #2    │ up to 30 entries
...                                   │
2+(29*12) 12   Attribute entry #30  ─┘
362     ...     Reserved / vendor data
```

**SMART Attribute Entry (12 bytes) — [DOCUMENTED FACT]:**[^9]

```
Offset  Len  Field
0       1    ID (0x01–0xFF; 0x00 = invalid/empty slot)
1       2    Flags (see below)
3       1    Current (normalized) value  [1–253; 0/0xFE/0xFF invalid]
4       1    Worst value                 [lowest current ever recorded]
5       6    Raw data                    [vendor-specific 48-bit]
11      1    Reserved (0x00)
```

**Flag bits:**[^9]
- bit 0: Pre-fail (1) vs Old-age (0)
- bit 1: Online collection (1 = updated during normal operation)
- bit 2: Performance attribute
- bit 3: Error rate attribute
- bit 5: Self-preserving

**[INFERENCE]** Attribute entries ไม่มีการรับประกันลำดับ — ต้องค้นหาด้วย ID ไม่ใช่ index

### E.2 SMART Threshold Response (512 bytes)

**[DOCUMENTED FACT]** Command Feature = 0xD1 (obsolete since ATA-4 แต่ยังรองรับ)[^10][^3]

Structure เหมือน SMART READ DATA แต่แต่ละ entry มี:
- byte 0: ID
- byte 1: Threshold value (0x00 = always passing; 0xFF = always failing)
- bytes 2–11: Reserved

**[INFERENCE]** เมื่อ `current_value ≤ threshold` และ `threshold > 0` → attribute เกิน threshold (warning)

### E.3 GPL Log Pages (READ LOG EXT, 48-bit)

**[DOCUMENTED FACT]** ต้องใช้ ATA PASS-THROUGH (16) เท่านั้น (ไม่ใช่ APT-12) เพราะเป็น 48-bit command[^11]

| Log Address | Type | Description |
|-------------|------|-------------|
| 0x00 | GPL | Log Directory (list of available logs) |
| 0x01 | SL | Summary SMART error log (up to 5 most recent errors) |
| 0x02 | SL | Comprehensive SMART error log |
| 0x03 | GPL | Ext. Comprehensive SMART error log[^12] |
| 0x04 | GPL | Device Statistics log[^13] |
| 0x06 | SL | SMART self-test log |
| 0x07 | GPL | Extended self-test log[^12] |
| 0x11 | GPL | SATA PHY Event Counters[^12] |
| 0x80–0x9F | GPL/SL | Host vendor-specific |

***

## F. Standardized vs. Vendor-Specific Attribute Matrix

**⚠️ [DOCUMENTED FACT]** Samsung Samsung ประกาศว่า "SMART attributes vary in meaning and interpretation by manufacturer" และ "some attributes are trade secrets" — ข้อนี้สำคัญมาก: **ห้าม assume ว่า attribute ID เดียวกัน = raw format เดียวกันต่างผู้ผลิต**[^14]

### Truly Standardized Attributes (cross-vendor consistent)

| ID | Name | Raw Interpretation | Source |
|----|------|--------------------|--------|
| 5 | Reallocated Sector Count | raw = จำนวน reallocated sectors | ATA/industry convention |
| 9 | Power-On Hours | raw = hours powered on | ATA/industry convention |
| 12 | Power Cycle Count | raw = จำนวน power cycles | ATA/industry convention |
| 187 | Reported Uncorrectable Errors | raw = count | ATA/industry convention |
| 194 | Drive Temperature | raw = current °C (**แต่** HDD บางรุ่น encode ต่างกัน) | Quasi-standard |
| 197 | Current Pending Sector Count | raw = unstable sector count | ATA/industry convention |
| 198 | Offline Uncorrectable Errors | raw = count | ATA/industry convention |
| 199 | CRC Error Count (UDMA) | raw = interface CRC errors | ATA/industry convention |

### SSD Endurance Attributes — Vendor-Specific Matrix

**[DOCUMENTED FACT]** ข้อมูลจาก Samsung official app note, Micron TN-FD-22, Backblaze comparison:[^15][^16][^14][^9]

| ID | Samsung | Intel/Solidigm | Micron/Crucial | WD/SanDisk | Seagate | Kingston |
|----|---------|---------------|----------------|------------|---------|----------|
| 160 | — | — | — | — | — | — |
| 169 | — | — | — | Remaining Lifetime % | — | — |
| 173 | — | — | Avg Block Erase Count; raw=avg erase[^9] | SSD Wear Leveling Count | — | Erase Count Avg/Max |
| 177 | **Wear Leveling Count** raw=avg erase cycles[^14][^17] | — | — | Wear Range Delta | — | — |
| 202 | SSD Mode Status | — | **Percent Lifetime Remaining**; raw = % used[^9] | Percentage Lifetime Used | — | — |
| 231 | — | — | — | Life Left (normalized 100→0) | Life Left | — |
| 232 | — | Endurance Remaining | Endurance Remaining | Endurance Remaining[^15] | — | — |
| 233 | — | **Media Wearout Indicator** (100→1)[^18][^8] | — | — | — | Lifetime Writes to Flash |
| 235 | **POR Recovery Count**[^17] | — | Good Block Count + System Block Count (encoded 3+2 bytes) | — | — | — |
| 241 | Total LBAs Written | Total LBAs Written | Cumulative Host Sectors Written[^9] | Total LBAs Written | Total LBAs Written[^15] | Total LBAs Written |
| 242 | Total LBAs Read | Total LBAs Read | — | Total LBAs Read[^15] | — | — |
| 246 | — | — | **Cumulative Host Sectors Written**[^9] | — | — | Total Erase Count |
| 247 | — | — | Host Program NAND Pages Count; WAF = (247+248)/247[^9] | — | — | — |
| 248 | — | — | FTL Program NAND Pages Count[^9] | Background program page count[^15] | — | — |

**[INFERENCE]** Safe approach สำหรับ "% life remaining" คือ: ตรวจสอบ attribute IDs 169, 202, 231, 232, 233 ตามลำดับ และ normalize ตาม model/vendor ใน runtime database แทนการ hardcode

***

## G. SATA SSD Endurance Metric Matrix (per vendor)

| Vendor | Primary Life % Attr | Raw Interpretation | WAF Available? | Total Written Attr |
|--------|--------------------|--------------------|----------------|--------------------|
| Samsung | 177 (Wear Leveling Count) | normalized 100→0; raw = avg erase cycles[^17] | ไม่ตรง (ต้องคำนวณจาก 241) | 241 (raw × 512 bytes = total) |
| Intel/Solidigm | 233 (Media Wearout Indicator) | normalized 100→1[^18][^8] | ไม่ explicit | 241 |
| Micron/Crucial | 202 (Percent Lifetime Remaining) | normalized; raw = % used[^9] | **ใช่**: (attr247 + attr248) / attr247[^9] | 246 (raw = LBA count) |
| WD/SanDisk | 169 (Remaining Lifetime %) | normalized remaining | ไม่ | 241 |
| Seagate SSD | 231 (Life Left) | normalized 100→0[^15] | ไม่ | 241 |
| Kingston | 231 (Life Left) หรือ 233 | ขึ้นกับ model | ไม่ | 246 (Cumulative Sectors) |

**[INFERENCE]** Write Amplification Factor (WAF) ที่ Micron expose ผ่าน attr 247 + 248 เป็น unique feature ไม่มีใน vendor อื่น — ต้องตรวจสอบ vendor ก่อน parse

***

## H. Decision Tree: Native SATA vs SAT vs USB Bridge vs Hardware RAID

```
Device at /dev/sdX or /dev/sgX
         │
         ▼
SCSI INQUIRY standard (0x12, 36 bytes)
         │
         ├─[VendorID="ATA     " (8 chars)]──────────────► Native SATA via libata
         │                                                  Use SAT APT-16 directly
         │
         ├─[VPD 0x89 available?]─ Yes ──────────────────► SAT layer (HBA firmware)
         │                                                  Parse IDENTIFY from VPD
         │
         ├─[USB device (check /sys/bus/usb)?]
         │    │
         │    ├─[SAT APT-16 probe succeeds?]──────────── ► SAT over USB
         │    │
         │    ├─[JMicron 0xDF probe?]─────────────────── ► JMicron quirk path
         │    ├─[Cypress signature 0xC0?]──────────────── ► Cypress ATACB
         │    └─[Unknown] → Log warning, skip SMART
         │
         └─[RAID controller (MegaRAID/hpsa/aacraid)?]
              │
              └─ SMART unavailable from logical volume
                 → Use controller-specific ioctl (optional)
                 → Or report "RAID logical — SMART N/A"
```

**[DOCUMENTED FACT]** HPE Smart Array ใช้ `CCISS_PASSTHRU` / `CCISS_BIG_PASSTHRU` ioctl ผ่าน `/dev/sgX` หรือ `cciss_ioctl.h` LSI MegaRAID ใช้ vendor-specific MR ioctl ผ่าน `/dev/sdX`[^19][^20]

**[DOCUMENTED FACT]** USB bridges ที่รองรับ SAT ได้แก่ ASMedia, Initio, Oxford, newer JMicron; Cypress ใช้ ATACB; JMicron รุ่นเก่า (JM20329, JM20335) ใช้ 0xDF command; Sunplus ใช้ vendor passthrough ของตัวเอง[^21]

***

## I. Linux SG_IO Execution Flow

### I.1 sg_io_hdr_t Structure

**[DOCUMENTED FACT]** จาก `include/scsi/sg.h` Linux UAPI:[^22][^23]

```c
typedef struct sg_io_hdr {
    int       interface_id;    // [i] must = 'S' (0x53)
    int       dxfer_direction; // [i] SG_DXFER_FROM_DEV=-3, SG_DXFER_NONE=-5
    uint8_t   cmd_len;         // [i] CDB length (12 or 16 for ATA PT)
    uint8_t   mx_sb_len;       // [i] max sense buffer length (recommend 32+)
    uint16_t  iovec_count;     // [i] 0 for direct buffer
    uint32_t  dxfer_len;       // [i] data transfer length (512 for IDENTIFY)
    void     *dxferp;          // [i] data buffer pointer
    uint8_t  *cmdp;            // [i] CDB pointer
    uint8_t  *sbp;             // [i] sense buffer pointer
    uint32_t  timeout;         // [i] milliseconds (recommend 20000 for ATA)
    uint32_t  flags;           // [i] 0 for default
    /* ... output fields ... */
    uint8_t   status;          // [o] SCSI status
    uint8_t   sb_len_wr;       // [o] actual sense bytes written
    int       resid;           // [o] residual byte count
} sg_io_hdr_t;  // 64 bytes on i386
```

**[DOCUMENTED FACT]** `dxfer_direction` values: `SG_DXFER_FROM_DEV = -3`, `SG_DXFER_NONE = -5`, `SG_DXFER_TO_DEV = -2`[^1]

**[DOCUMENTED FACT]** `SG_IO` ioctl ต้องการ `CAP_SYS_RAWIO` บน partition device แต่ process ที่มี `CAP_SYS_RAWIO` จะผ่านได้ บน whole disk device (`/dev/sdX`) ต้องการ `CAP_SYS_RAWIO` เสมอสำหรับ ATA passthrough[^24][^25]

### I.2 ATA PASS-THROUGH (16) CDB Layout

**[DOCUMENTED FACT]** จาก SAT standard และ smartmontools `scsiata.cpp`:[^26]

```
cdb  = 0x85                          // APT-16 opcode
cdb[^1]  = (protocol << 1) | extend     // protocol=4 (PIO-in), extend=1 for 48-bit
cdb[^2]  = (ck_cond<<5) | (t_dir<<3) | (byte_block<<2) | t_length
          // ck_cond=1 to read back registers
          // t_dir=1 (from device), byte_block=1 (512-byte blocks), t_length=2
cdb[^3]  = features[15:8]  (hi, 0 for 28-bit)
cdb[^4]  = features[7:0]   (lo) = SMART subcommand (e.g., 0xD0)
cdb[^5]  = sector_count[15:8] (hi)
cdb[^6]  = sector_count[7:0]  (lo) = 1 (for 1 sector = 512 bytes)
cdb[^7]  = lba_low[15:8]
cdb[^8]  = lba_low[7:0]      = log address for SMART READ LOG
cdb[^9]  = lba_mid[15:8]
cdb[^10] = lba_mid[7:0]      = 0x4F for SMART commands (magic)
cdb[^11] = lba_high[15:8]
cdb[^12] = lba_high[7:0]     = 0xC2 for SMART commands (magic)
cdb[^13] = device            = 0xA0 or 0xE0
cdb[^14] = command           = 0xB0 (SMART), 0xEC (IDENTIFY), 0x2F (READ LOG EXT)
cdb[^15] = 0 (SCSI control)
```

**[DOCUMENTED FACT]** Protocol values (cdb bits 4:1):[^27][^26]
- 3 = Non-data
- 4 = PIO data-in
- 5 = PIO data-out

### I.3 ATA Return Descriptor (Sense Descriptor Code 0x09)

**[DOCUMENTED FACT]** จาก SAT-2 specification และ smartmontools source:[^26]

```
des = 0x09          // descriptor code
des[^1] = 0x0C          // additional length
des[^2] = extend bit
des[^3] = error register
des[^4] = sector_count[15:8]
des[^5] = sector_count[7:0]
des[^6] = lba_low[15:8]
des[^7] = lba_low[7:0]
des[^8] = lba_mid[15:8]
des[^9] = lba_mid[7:0]
des[^10] = lba_high[15:8]
des[^11] = lba_high[7:0]
des[^12] = device
des[^13] = status register
```

**SMART RETURN STATUS interpretation — [DOCUMENTED FACT]:**
- LBA_MID=0x4F, LBA_HIGH=0xC2 → Drive OK
- LBA_MID=0xF4, LBA_HIGH=0x2C → Pre-failure predicted[^26]

***

## J. Rust Transport Traits และ Data Model

### J.1 Trait Definitions

**[ARCHITECTURAL RECOMMENDATION]** ออกแบบ trait hierarchy ดังนี้:

```rust
/// Stable kernel UAPI — ioctl number จาก include/scsi/sg.h
const SG_IO: u64 = 0x2285;

/// Transport abstraction — enables mock injection for testing
pub trait AtaTransport: Send + Sync {
    fn execute(
        &self,
        cdb: &[u8],
        direction: Direction,
        data: &mut [u8],
        sense: &mut [u8; 64],
        timeout_ms: u32,
    ) -> Result<AtaStatus, TransportError>;
}

/// High-level backend trait
pub trait DiskHealthBackend: Send + Sync {
    fn identify(&self) -> Result<IdentifyDevice, BackendError>;
    fn smart_read_data(&self) -> Result<SmartData, BackendError>;
    fn smart_read_thresholds(&self) -> Result<SmartThresholds, BackendError>;
    fn smart_return_status(&self) -> Result<SmartStatus, BackendError>;
    fn read_log_page(&self, addr: u8, page: u16, count: u16) 
        -> Result<Vec<u8>, BackendError>;
    fn capabilities(&self) -> BackendCapabilities;
}
```

### J.2 Core Data Structures

```rust
#[derive(Debug, Clone)]
pub struct IdentifyDevice {
    pub model: String,           // words 27-46, de-swapped + trimmed
    pub serial: String,          // words 10-19
    pub firmware: String,        // words 23-26
    pub is_ssd: bool,            // word 217 == 0x0001
    pub rotation_rpm: Option<u16>, // word 217 (None if unknown)
    pub wwn: Option<u64>,        // words 108-111
    pub lba48_sectors: Option<u64>, // words 100-103
    pub logical_sector_bytes: u32,  // from word 106 or default 512
    pub physical_sector_bytes: u32, // from word 106 ratio
    pub sata_gen: Option<SataGeneration>, // from word 76
    pub smart_supported: bool,   // word 82 bit 0
    pub gpl_supported: bool,     // word 84 bit 5
    pub raw: Box<[u16; 256]>,    // preserve for parser extension
}

#[derive(Debug, Clone)]
pub struct SmartAttribute {
    pub id: u8,
    pub flags: u16,
    pub current: u8,
    pub worst: u8,
    pub raw: [u8; 6],    // raw 48-bit, vendor-specific interpretation
    pub threshold: Option<u8>,  // from separate SMART READ THRESHOLDS
}

impl SmartAttribute {
    pub fn is_pre_fail(&self) -> bool { self.flags & 0x01 != 0 }
    pub fn threshold_exceeded(&self) -> bool {
        self.threshold.map(|t| t > 0 && self.current <= t).unwrap_or(false)
    }
    pub fn raw_u48(&self) -> u64 {
        // little-endian 6 bytes → u64
        u64::from_le_bytes([
            self.raw, self.raw[^1], self.raw[^2],
            self.raw[^3], self.raw[^4], self.raw[^5], 0, 0
        ])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SmartStatus { Passed, PredictingFailure, Unknown }

#[derive(Debug, Clone)]
pub enum TransportError {
    Permission(io::Error),         // EPERM, EACCES
    NotSupported(String),          // bridge rejected, no SAT
    CheckCondition { sense_key: u8, asc: u8, ascq: u8 },
    Transport(io::Error),          // ioctl failed
    MalformedResponse(String),     // parse error
    DeviceGone,                    // ENODEV during operation
    Timeout,
}
```

### J.3 Vendor Database Architecture

**[ARCHITECTURAL RECOMMENDATION]** ออกแบบ vendor database เป็น runtime-loadable ไม่ใช่ compile-time constant:

```rust
pub struct VendorAttributeSpec {
    pub id: u8,
    pub name: &'static str,
    /// Closure หรือ enum ที่ interpret raw [u8; 6] → human value
    pub raw_parser: RawParser,
}

pub enum RawParser {
    Raw48AsU64,                   // common case
    Raw16 { byte_range: [u8; 2] }, // Seagate attr 188
    TempCurrentMinMax,             // Micron/WD temp encoding
    Custom(fn([u8; 6]) -> u64),
}
```

**[DOCUMENTED FACT]** smartmontools `drivedb.h` ใช้ GPL-2.0-or-later — **ห้าม copy code หรือ database entries เข้าใน MIT/Apache project** ต้องเขียน parser ใหม่อิสระจาก drivedb logic แม้จะ reference เป็น "oracle" สำหรับ development ได้[^26]

***

## K. Safe Rust Pseudocode / Code Skeleton

```rust
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;

/// SG_IO ioctl wrapper — ใช้ libc::ioctl โดยตรง (nix ก็ได้)
pub struct SgIoDevice {
    fd: File,
    path: PathBuf,
}

impl SgIoDevice {
    pub fn open(path: &Path) -> Result<Self, TransportError> {
        let fd = OpenOptions::new()
            .read(true)
            .write(false)          // read-only open เพียงพอสำหรับ SG_IO read commands
            .open(path)
            .map_err(TransportError::Transport)?;
        Ok(Self { fd, path: path.to_owned() })
    }
}

impl AtaTransport for SgIoDevice {
    fn execute(
        &self,
        cdb: &[u8],
        direction: Direction,
        data: &mut [u8],
        sense: &mut [u8; 64],
        timeout_ms: u32,
    ) -> Result<AtaStatus, TransportError> {
        // Build sg_io_hdr_t — repr(C) required
        #[repr(C)]
        struct SgIoHdr {
            interface_id: i32,    // 'S' = 0x53
            dxfer_direction: i32, // SG_DXFER_FROM_DEV = -3, SG_DXFER_NONE = -5
            cmd_len: u8,
            mx_sb_len: u8,
            iovec_count: u16,
            dxfer_len: u32,
            dxferp: *mut u8,
            cmdp: *const u8,
            sbp: *mut u8,
            timeout: u32,         // milliseconds
            flags: u32,
            pack_id: i32,
            usr_ptr: *mut (),
            status: u8,           // [out] SCSI status
            masked_status: u8,
            msg_status: u8,
            sb_len_wr: u8,        // [out] sense bytes written
            host_status: u16,
            driver_status: u16,
            resid: i32,
            duration: u32,
            info: u32,
        }

        let dxfer_dir = match direction {
            Direction::FromDevice => -3i32,  // SG_DXFER_FROM_DEV
            Direction::None => -5i32,        // SG_DXFER_NONE
        };

        let mut hdr = SgIoHdr {
            interface_id: 0x53,  // 'S'
            dxfer_direction: dxfer_dir,
            cmd_len: cdb.len() as u8,
            mx_sb_len: sense.len() as u8,
            iovec_count: 0,
            dxfer_len: if direction == Direction::None { 0 } else { data.len() as u32 },
            dxferp: if direction == Direction::None { std::ptr::null_mut() } 
                    else { data.as_mut_ptr() },
            cmdp: cdb.as_ptr(),
            sbp: sense.as_mut_ptr(),
            timeout: timeout_ms,
            flags: 0,
            ..unsafe { std::mem::zeroed() }
        };

        let ret = unsafe {
            libc::ioctl(self.fd.as_raw_fd(), SG_IO as _, &mut hdr as *mut _)
        };

        if ret < 0 {
            let err = io::Error::last_os_error();
            return match err.raw_os_error() {
                Some(libc::EPERM) | Some(libc::EACCES) => 
                    Err(TransportError::Permission(err)),
                Some(libc::ENODEV) => Err(TransportError::DeviceGone),
                _ => Err(TransportError::Transport(err)),
            };
        }

        // Check SCSI status
        if hdr.status == 0x02 {  // CHECK CONDITION
            return Err(parse_sense_error(sense, hdr.sb_len_wr));
        }

        Ok(parse_ata_return_descriptor(sense, hdr.sb_len_wr))
    }
}

/// Build APT-16 CDB สำหรับ SMART READ DATA
pub fn build_smart_read_data_cdb() -> [u8; 16] {
    let mut cdb = [0u8; 16];
    cdb = 0x85;        // APT-16 opcode
    cdb[^1] = (4 << 1);    // protocol=4 (PIO data-in), extend=0
    cdb[^2] = (1 << 5) | (1 << 3) | (1 << 2) | 2;
    //        ck_cond     t_dir       byte_block   t_length=2
    cdb[^4] = 0xD0;        // SMART READ DATA feature
    cdb[^6] = 1;           // sector_count = 1
    cdb[^10] = 0x4F;       // lba_mid magic
    cdb[^12] = 0xC2;       // lba_high magic
    cdb[^13] = 0xA0;       // device
    cdb[^14] = 0xB0;       // SMART command
    cdb
}

/// Parse IDENTIFY DEVICE — de-swap ATA strings
pub fn parse_ata_string(words: &[u16], start_word: usize, len_words: usize) -> String {
    let mut bytes = Vec::with_capacity(len_words * 2);
    for &w in &words[start_word..start_word + len_words] {
        bytes.push((w >> 8) as u8);   // high byte first (ATA byte-swap)
        bytes.push((w & 0xFF) as u8);
    }
    String::from_utf8_lossy(&bytes).trim().to_string()
}

/// Tokio integration — ioctl เป็น blocking → spawn_blocking
pub async fn collect_smart_async(
    device: Arc<SgIoDevice>,
) -> Result<SmartData, BackendError> {
    tokio::task::spawn_blocking(move || {
        let mut data = [0u8; 512];
        let mut sense = [0u8; 64];
        let cdb = build_smart_read_data_cdb();
        device.execute(&cdb, Direction::FromDevice, &mut data, &mut sense, 20_000)?;
        parse_smart_read_data(&data)
    })
    .await
    .map_err(|e| BackendError::Join(e))?
}
```

**[ARCHITECTURAL RECOMMENDATION]** เรื่อง `repr(C)` และ `#[repr(C, packed)]`: สำหรับ `sg_io_hdr_t` ใช้ `repr(C)` เท่านั้น (ไม่ใช่ packed) เพราะ kernel expects natural alignment; สำหรับ binary response parsing ให้ใช้ helper functions แทน packed struct เพื่อหลีกเลี่ยง UB จาก unaligned access ใน Rust

***

## L. Error, Sense และ ATA Status Decoding

### L.1 SCSI Sense Key Interpretation

| Sense Key | Hex | ATA Context |
|-----------|-----|-------------|
| NO SENSE | 0x0 | Success (หรือ recovered error) |
| RECOVERED ERROR | 0x1 | ATA return descriptor available |
| NOT READY | 0x2 | Drive not ready / spun down |
| MEDIUM ERROR | 0x3 | Read/write error on media |
| ILLEGAL REQUEST | 0x5 | Command not supported / bridge rejected |
| ABORTED COMMAND | 0xB | ATA Aborted Command (ERR bit in status) |

**[DOCUMENTED FACT]** เมื่อ `ck_cond=1` และ command สำเร็จ ATA return descriptor จะอยู่ใน sense data ด้วย sense key = RECOVERED ERROR — ต้อง handle กรณีนี้เป็น success ไม่ใช่ error[^26]

### L.2 ATA Status Register Bits

| Bit | Name | Meaning |
|-----|------|---------|
| 7 | BSY | Device busy (ถ้า set, bits อื่น invalid) |
| 6 | DRDY | Device ready |
| 5 | DF | Device fault |
| 4 | SERV | Service |
| 3 | DRQ | Data request |
| 0 | ERR | Error occurred (ดู Error register) |

**[DOCUMENTED FACT]** SMART success pattern: status = 0x40 (BSY=0, DRDY=1, ERR=0)[^26]

### L.3 USB Bridge Failure Modes

**[DOCUMENTED FACT]** จาก smartmontools wiki:[^21]
- JMicron JM20336: บางรุ่น return `0x01` เสมอสำหรับ SMART STATUS — ต้อง detect และ skip
- Cypress CY7C68300A: ไม่รองรับ passthrough (รองรับเฉพาะ B/C revision)
- UAS mode ใน Linux kernel อาจ reject SAT ATA PASS-THROUGH ด้วย "unsupported field in scsi command" สำหรับ device บางรุ่น — ลอง `-d sat,12` (APT-12) แทน

**[INFERENCE]** Safe probe sequence สำหรับ USB: ส่ง INQUIRY ก่อน → ตรวจ VendorID → ถ้าเป็น "ATA " ลอง APT-16 → ถ้าตอบด้วย ILLEGAL REQUEST ลอง APT-12 → ถ้ายังไม่ได้ลอง vendor-specific probe → log warning แล้ว skip SMART

***

## M. Privilege and Security Model

**[DOCUMENTED FACT]** `SG_IO` บน whole disk device ต้องการ `CAP_SYS_RAWIO` Process ที่มี `CAP_SYS_RAWIO` สามารถส่ง SG_IO ไปยัง partition device ได้เช่นกัน แต่ logged เป็น warning ในเก่า kernel versions[^25][^24]

**[ARCHITECTURAL RECOMMENDATION]** Security model ที่แนะนำ:

```
┌─────────────────────────────────────────────────────────┐
│  User-facing process (no privilege)                     │
│  ├── Sends requests via Unix socket / pipe              │
│  └── Receives HealthSnapshot (serialized)               │
├─────────────────────────────────────────────────────────┤
│  Privileged helper binary (separate executable)         │
│  ├── CAP_SYS_RAWIO (or setuid root for simplicity)      │
│  ├── Command allowlist: IDENTIFY, SMART READ only       │
│  ├── Rejects all write/destructive ATA opcodes          │
│  ├── Validates device path (/dev/sd*, /dev/sg* only)    │
│  └── Timeout enforced (max 30s per command)             │
└─────────────────────────────────────────────────────────┘
```

**[ARCHITECTURAL RECOMMENDATION]** API ต้องไม่รับ arbitrary ATA taskfile จาก caller — ต้องรับเฉพาะ high-level request เช่น `ReadSmartData { device: "/dev/sda" }` แล้ว helper construct CDB เอง

**[DOCUMENTED FACT]** `SG_IO` บน partition device (เช่น `/dev/sda1`) ถูก block โดย kernel สำหรับ non-CAP_SYS_RAWIO process — ต้องใช้ whole disk path[^24]

***

## N. Fixture, Fuzzing และ Hardware Test Plan

### N.1 Fixture-Based Unit Tests

**[ARCHITECTURAL RECOMMENDATION]** inject `root: PathBuf` ให้กับ transport layer เพื่อ point ไปยัง fixture directory:

```
fixtures/
├── sda_identify.bin          # 512 bytes จาก IDENTIFY DEVICE
├── sda_smart_data.bin        # 512 bytes จาก SMART READ DATA
├── sda_smart_thresh.bin      # 512 bytes จาก SMART READ THRESHOLDS
├── vendors/
│   ├── samsung_860_evo/
│   ├── intel_d3_s4520/
│   ├── micron_1100/
│   ├── wd_blue_sa510/
│   └── seagate_barracuda_120/
└── error_cases/
    ├── check_condition_ata_error.bin
    ├── usb_jmicron_bogus_status.bin
    └── truncated_identify.bin     # ทดสอบ partial response
```

**ได้ fixtures จากไหน:** `sg_raw` หรือ `sg_sat_identify` จาก `sg3_utils` สามารถ dump binary response ได้โดยตรง — ใช้เป็น development oracle เท่านั้น

### N.2 Fuzzing

**[ARCHITECTURAL RECOMMENDATION]** fuzz parser ด้วย `cargo-fuzz` หรือ `afl.rs`:

```rust
// fuzz target สำหรับ IDENTIFY parser
fuzz_target!(|data: &[u8]| {
    if data.len() >= 512 {
        let words: [u16; 256] = bytemuck::pod_read_unaligned(data);
        let _ = parse_identify_device(&words);  // ต้องไม่ panic
    }
});
```

target ที่ต้อง fuzz:
1. `parse_identify_device` — 512-byte input
2. `parse_smart_read_data` — 512-byte input
3. `parse_sense_descriptor` — 64-byte input
4. `parse_ata_return_descriptor` — variable-length descriptor

### N.3 Integration Test Matrix

| Test Scenario | Setup | Expected Result | Requires |
|---------------|-------|-----------------|---------|
| Native SATA SSD | `/dev/sdX` (AHCI) | Full SMART + IDENTIFY | Real hardware or VM with passthrough |
| Native SATA HDD | `/dev/sdX` (AHCI) | rotation_rpm > 0 | Real hardware |
| USB SAT bridge (ASMedia) | USB enclosure | SMART via APT-16 | USB hardware |
| USB JMicron legacy | JM20335 enclosure | Quirk path activated | USB hardware |
| No permission | run without CAP_SYS_RAWIO | TransportError::Permission | Normal user |
| Partition device | `/dev/sda1` | ENOIOCTLCMD or error | Any block device |
| Device removed | hot-unplug during read | TransportError::DeviceGone | Hot-plug hardware |
| Degraded SMART (threshold exceeded) | Fixture | Pre-failure flag detected | Fixture only |

***

## O. Implementation Roadmap

### MVP (3–4 weeks)

1. **Week 1:** `SgIoDevice` + `build_apt16_cdb()` + `sg_io_hdr_t` binding
   - IDENTIFY DEVICE command + parser (serial, model, firmware, SSD detection)
   - Unit tests ด้วย 3–5 vendor fixtures

2. **Week 2:** SMART READ DATA + SMART READ THRESHOLDS parsers
   - Attribute struct + threshold crossing detection
   - `SmartStatus` via SMART RETURN STATUS

3. **Week 3:** Device discovery + transport detection
   - Scan `/dev/sd*` + `/dev/sg*`
   - INQUIRY VPD 0x89 probe
   - USB bridge probe sequence

4. **Week 4:** Tokio integration + error model + fixture tests
   - `spawn_blocking` wrapper[^28]
   - Full error enum coverage

### Production Hardening (2–3 weeks)

5. **Week 5:** GPL log pages (READ LOG EXT) — error log + self-test log
   - APT-16 48-bit command path
   - Log directory discovery from address 0x00

6. **Week 6:** Vendor database runtime + SSD endurance normalization
   - Samsung / Intel / Micron / WD / Seagate attribute specs
   - WAF calculation for Micron (attr 247 + 248)[^9]

7. **Week 7:** Fuzzing + security hardening + privilege separation helper

***

## P. Primary Source Links

> ⚠️ T13 ACS standards (ACS-3, ACS-4, ACS-5) ต้องซื้อจาก INCITS หรือ ANSI แต่ working drafts อยู่ที่ t13.org

| Source | URL / Reference |
|--------|----------------|
| Linux SG_IO (sg v3 interface) | https://sg.danny.cz/sg/sg_io.html [^1] |
| Linux sg.h UAPI header | https://github.com/torvalds/linux/blob/master/include/scsi/sg.h [^23] |
| Linux SCSI ioctl kernel source | https://github.com/torvalds/linux/blob/master/drivers/scsi/scsi_ioctl.c [^29] |
| Linux HDIO_ ioctl docs (deprecated) | https://docs.kernel.org/userspace-api/ioctl/hdio.html [^30] |
| T13 ACS-5 Last Draft | https://www.t13.org/project-last-drafts [^31] |
| SAT standard info (T10) | https://www.t10.org (drafts: sat-r09.pdf, SAT-2, SAT-3) [^32] |
| SAT APT-16 CDB layout | smartmontools scsiata.cpp [^26] |
| T10 SAT-2 sense data | https://www.t10.org/ftp/t10/document.08/08-344r0.pdf [^33] |
| sg3_utils sg_sat_identify(8) | https://man.archlinux.org/man/extra/sg3_utils/sg_sat_identify.8.en [^32] |
| sg_vpd VPD 0x89 ATA Info | https://linux.die.net/man/8/sg_vpd [^4] |
| Micron SMART TN-FD-22 | https://device.report/m/2495f5... (official Micron doc) [^9] |
| Samsung SMART App Note | https://semiconductor.samsung.com/resources/others/SSD_Application_Note_SMART_final.pdf [^14] |
| Backblaze SSD SMART Comparison | https://f001.backblazeb2.com/file/Backblaze_Blog/SSD+SMART+Stats+Comparison+Table.pdf [^15] |
| smartmontools GPL source | https://www.smartmontools.org (GPL-2.0-or-later — reference only)[^26] |
| hdd-rs Rust crate (MPL-2.0) | https://github.com/vthriller/hdd-rs [^2] |
| tokio::task::spawn_blocking | https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html [^28] |
| HPE hpsa driver (CCISS ioctl) | https://man.archlinux.org/man/hpsa.4.en [^19] |
| smartmontools RAID controllers | https://www.smartmontools.org/wiki/Supported_RAID-Controllers [^20] |
| smartmontools USB bridge list | https://www.smartmontools.org/wiki/USB [^21] |
| Linux SG_IO partition security | kernel commit: block: fail SCSI passthrough ioctls on partition [^24] |

***

## ข้อสรุปที่ยังต้องยืนยันด้วย Hardware หรือ kernel source

1. **[NEEDS HARDWARE TEST]** USB bridge probe order — behavior ที่แน่นอนของ ASMedia, Realtek, JMicron รุ่นใหม่กับ APT-16 vs APT-12 ต้องทดสอบกับ hardware จริง
2. **[NEEDS HARDWARE TEST]** timeout ที่เหมาะสมสำหรับ SMART READ LOG บน USB — 20 seconds อาจสั้นเกินไปสำหรับ some bridges
3. **[NEEDS KERNEL SOURCE VERIFICATION]** `read(2)` บน `/dev/sgX` vs `SG_IO` ioctl — ควรใช้ ioctl เสมอ แต่ behavior บน UAS mode ต้องยืนยันกับ kernel source `drivers/usb/storage/`
4. **[NEEDS TEST]** SMART READ THRESHOLDS (0xD1) ใน ACS-5 compliant drives รุ่นใหม่ — บางรุ่นอาจ return error แทนที่จะเป็น empty thresholds
5. **[NEEDS VERIFICATION]** Hardware RAID passthrough (MegaRAID 0xC2 ioctl) ยังคง function ใน kernel ≥ 6.x — ข้อมูลที่มีอ้างอิงถึง driver version เก่า

---

## References

1. [Linux SG_IO ioctl in the 2.6 series](https://sg.danny.cz/sg/sg_io.html)

2. [GitHub - vthriller/hdd-rs: [WIP] instruments for querying ATA and SCSI disks](https://github.com/vthriller/hdd-rs) - [WIP] instruments for querying ATA and SCSI disks. Contribute to vthriller/hdd-rs development by cre...

3. [[smartmontools-support] WD30EFRX - Read SMART Thresholds ...](https://sourceforge.net/p/smartmontools/mailman/message/30159539/)

4. [sg_vpd(8) - Linux man page](https://linux.die.net/man/8/sg_vpd) - This utility fetches a Vital Product Data page and decodes it or outputs it in ASCII hexadecimal or ...

5. [What may cause a limit on SG_IO ioctl maximum sector count of a transfer?](https://stackoverflow.com/questions/42621227/what-may-cause-a-limit-on-sg-io-ioctl-maximum-sector-count-of-a-transfer) - I need to pass a direct ATA request to a hard drive (0x25, READ DMA EXT), to disobey max sector coun...

6. [ata package - github.com/MarkusFreitag/smart/ ...](https://pkg.go.dev/github.com/MarkusFreitag/smart/ata)

7. [Using SAT to access SATA drives](http://www.scsitoolbox.com/pdfs/UsingSAT.pdf) - SAT (SCSI->ATA Translation) is a mechanism whereby ATA task register commands may be sent to a devic...

8. [SSD vs HDD: Does S.M.A.R.T. Monitoring Still Matter? - PANTERASoft](https://panterasoft.com/blog/ssd-vs-hdd-smart-monitoring) - SSDs have replaced HDDs in most modern PCs. But S.M.A.R.T. monitoring is still relevant — the metric...

9. [TN-FD-22: Client SATA SSD SMART Attribute Reference](https://device.report/m/2495f535851191273034a114ab2dcfab828b744ebe85b7353fe2baeae51fb880.pdf)

10. [Where can I find the SMART Threshold values of a HDD?](https://stackoverflow.com/questions/44650030/where-can-i-find-the-smart-threshold-values-of-a-hdd) - I am writing a SMART monitor tool and I managed to get SMART attributes [Current, Worst, Raw Data] w...

11. [[smartmontools-support] odd selftest log on an OCZ ...](https://sourceforge.net/p/smartmontools/mailman/smartmontools-support/thread/CAKBmWJ6-QSGP-KUwL=a2-xEimhj5xcyTfov=qRfv8Z2kLDZZVg@mail.gmail.com/)

12. [SMART error on WD Red 3TB in FreeNAS](https://community.wd.com/t/smart-error-on-wd-red-3tb-in-freenas/148305) - 0x03 GPL R/O 6 Ext. Comprehensive SMART error log 0x06 SL R/O 1 SMART self-test log 0x07 GPL R/O 1 E...

13. [SMART/HDD/Toshiba/DT01/DT01ACA100/784148857DD7 at master · linuxhw/SMART](https://github.com/linuxhw/SMART/blob/master/HDD/Toshiba/DT01/DT01ACA100/784148857DD7) - Estimate reliability of desktop-class HDD/SSD drives - linuxhw/SMART

14. [S.M.A.R.T.](https://semiconductor.samsung.com/resources/others/SSD_Application_Note_SMART_final.pdf) - SMART (also written S.M.A.R.T.), which stands for Self-Monitoring, Analysis and Reporting Technology...

15. [[PDF] SSD+SMART+Stats+Comparison+Table.pdf](https://f001.backblazeb2.com/file/Backblaze_Blog/SSD+SMART+Stats+Comparison+Table.pdf)

16. [SMART Values in Use  - various drives.xlsx  -  2](https://www.backblaze.com/blog/wp-content/uploads/2023/06/SSDSMARTStatsComparisonTable.pdf)

17. [SSD – smartctl – Status](https://superuser.com/questions/637450/ssd-smartctl-status) - I have a SAMSUNG SSD 830 Series SSD in my MacBook Pro. In my work, I need to analyse huges amounts o...

18. [WD WDS SSDs show incorrect Media Wearout Indicator ...](https://www.smartmontools.org/ticket/1620) - SMART Attributes Data Structure revision number: 4 Vendor Specific SMART Attributes ... Intel also u...

19. [hpsa(4)](https://man.archlinux.org/man/hpsa.4.en)

20. [Checking disks behind RAID controllers](https://www.smartmontools.org/wiki/Supported_RAID-Controllers)

21. [USB – smartmontools](https://www.smartmontools.org/wiki/USB) - The USB bridge provides an ATA or NVMe pass-through command. · This command is supported by smartmon...

22. [SG_IO ioctl, sg_io_hdr_t structure and to_scsi_device](http://deeplylovemac.blogspot.com/2008/01/sgiohdrt-structure.html) - evpd(enable vital product data): the server shall return VPD specified by Page Code when evpd=1 The ...

23. [linux/include/scsi/sg.h at master · torvalds/linux - GitHub](https://github.com/torvalds/linux/blob/master/include/scsi/sg.h) - SG_DXFER_FROM_DEV with the additional property than during indirect IO the user buffer is copied int...

24. [block: fail SCSI passthrough ioctls on partition devices - linux-rng](https://git.zx2c4.com/linux-rng/commit/block/scsi_ioctl.c?id=0bfc96cb77224736dfa35c3c555d37b3646ef35e)

25. [scsi: Silence unnecessary warnings about ioctl to partition - linux-rng](https://git.zx2c4.com/linux-rng/commit/?id=6d9359280753d2955f86d6411047516a9431eb51)

26. [scsiata.cpp Source File - smartmontools](https://www.smartmontools.org/static/doxygen/scsiata_8cpp_source.html) - ... ATA PASS THROUGH (16) SCSI command opcode byte (0x85). 168// cdb[1]: multiple_count, protocol + ...

27. [proc/diskstats](https://www.kernel.org/doc/Documentation/ABI/testing/procfs-diskstats)

28. [spawn_blocking in tokio::task - Rust - Docs.rs](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) - Runs the provided closure on a thread where blocking is acceptable.

29. [linux/drivers/scsi/scsi_ioctl.c at master · torvalds/linux](https://github.com/torvalds/linux/blob/master/drivers/scsi/scsi_ioctl.c) - Linux kernel source tree. Contribute to torvalds/linux development by creating an account on GitHub.

30. [Summary of HDIO_ ioctl calls](https://docs.kernel.org/userspace-api/ioctl/hdio.html)

31. [Projects - Last Drafts](https://www.t13.org/project-last-drafts)

32. [sg_sat_identify(8) - Arch manual pages](https://man.archlinux.org/man/extra/sg3_utils/sg_sat_identify.8.en) - The SAT standard (SAT ANSI INCITS 431-2007, prior draft: sat-r09.pdf at www.t10.org) defines two SCS...

33. [08-344r0 SAT-2 ATA PASS-THROUGH sense data format](https://www.t10.org/ftp/t10/document.08/08-344r0.pdf)

