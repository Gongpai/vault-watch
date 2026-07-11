# Native Linux NVMe SSD Health Monitoring ด้วย Rust — Technical Research Report

> **วิธีอ่านรายงาน:**
> - **[SPEC]** = ระบุอยู่ใน NVMe Base Specification หรือ NVMe Command Set Specification
> - **[LINUX]** = พฤติกรรมของ Linux kernel ที่ document แล้ว (UAPI, kernel source, Documentation/)
> - **[INFERENCE]** = อนุมานจากหลักฐานที่มี ยังต้องยืนยัน
> - **[ARCH-REC]** = คำแนะนำสถาปัตยกรรมจากผู้วิจัย

***

## A. Feasibility Verdict

**[LINUX]** การสร้าง native NVMe health monitoring โดยไม่เรียก `nvme-cli` หรือ `smartctl` เป็น runtime dependency เป็นไปได้อย่างสมบูรณ์ Linux kernel expose ioctl interface ผ่าน `/dev/nvmeN` character device โดยใช้ `NVME_IOCTL_ADMIN_CMD` ซึ่งเป็น stable documented UAPI ใน `include/uapi/linux/nvme_ioctl.h`[^1][^2]

**[SPEC]** NVMe SMART/Health Information (Log Page 0x02) และ Identify Controller/Namespace เป็น mandatory commands ใน NVMe Base Specification ทุกเวอร์ชัน — ใช้งานได้กับ consumer และ enterprise SSD ทุกรุ่นที่ NVMe-compliant[^3][^4]

**ขอบเขตที่ทำได้:**
- PCIe NVMe (AHCI alternative): ✅ สมบูรณ์ — direct `NVME_IOCTL_ADMIN_CMD` บน `/dev/nvme0`
- NVMe-oF (TCP/RDMA): ✅ ส่วนใหญ่ — ioctl เดียวกัน แต่ต้องจัดการ reconnect
- NVMe multipath: ⚠️ ต้องอ่านจาก underlying paths ไม่ใช่ merged block device
- Enterprise features (Endurance Group, Telemetry): ✅ ถ้า controller รองรับ NVMe 1.4+[^5]

***

## B. Linux NVMe Interface Comparison

**[LINUX]** จาก `include/uapi/linux/nvme_ioctl.h`:[^1]

| Interface | Ioctl | Device | Min Kernel | Use Case | Notes |
|-----------|-------|--------|------------|----------|-------|
| `NVME_IOCTL_ADMIN_CMD` | `_IOWR('N',0x41)` | `/dev/nvmeN` | 3.3 | Identify, Get Log Page | **Primary — ใช้สำหรับ health** |
| `NVME_IOCTL_ADMIN64_CMD` | `_IOWR('N',0x47)` | `/dev/nvmeN` | 5.14 | Admin commands returning 64-bit result | ใช้เมื่อต้องการ result64 |
| `NVME_IOCTL_ID` | `_IO('N',0x40)` | `/dev/nvmeNnM` | 3.3 | Get Namespace ID | ง่ายแต่ใช้ได้แค่บน ns device |
| `NVME_IOCTL_IO_CMD` | `_IOWR('N',0x43)` | `/dev/nvmeNnM` | 3.19 | IO passthrough | **ห้ามใช้สำหรับ health monitoring** |
| `NVME_URING_CMD_ADMIN` | `_IOWR('N',0x81)` | `/dev/nvmeN` | 6.0 | io_uring async admin | Optional async path |
| `NVME_IOCTL_RESET` | `_IO('N',0x44)` | `/dev/nvmeN` | 3.19 | Controller reset | CAP_SYS_ADMIN required[^6] |

**[LINUX]** `NVME_IOCTL_ADMIN_CMD` เป็น alias ของ `struct nvme_passthru_cmd` — ชื่อเก่า `nvme_admin_cmd` ยังคงใช้ได้เป็น `#define nvme_admin_cmd nvme_passthru_cmd` ใน UAPI header[^2][^1]

**[ARCH-REC]** ใช้ `NVME_IOCTL_ADMIN_CMD` (32-bit result) สำหรับ health monitoring เกือบทุกกรณี เปลี่ยนเป็น `NVME_IOCTL_ADMIN64_CMD` เฉพาะเมื่อต้องการ 64-bit result value

**[ARCH-REC]** `NVME_URING_CMD_ADMIN` (io_uring, kernel 6.0+) ให้ async path ที่แท้จริงโดยไม่ต้อง `spawn_blocking` แต่ต้องรองรับ kernel เก่ากว่า 6.0 ด้วย — แนะนำให้รองรับทั้งสองทางผ่าน feature flag

***

## C. Discovery and Identity Graph

### C.1 Sysfs Hierarchy

**[LINUX]** จาก kernel documentation และ sysfs driver source:[^7][^8][^9]

```
/sys/class/nvme/
└── nvme0 → /sys/devices/pci.../nvme/nvme0/
    ├── model           # model name (trimmed string from Identify Controller)
    ├── serial          # serial number
    ├── firmware_rev    # firmware revision
    ├── state           # "live" | "resetting" | "connecting" | "deleting" | "dead"
    ├── subsysnqn       # NVMe subsystem NQN (unique identifier)
    ├── hostnqn         # host NQN
    ├── transport       # "pcie" | "tcp" | "rdma" | "fc" | "loop"
    ├── address         # PCIe BDF or fabric address
    ├── cntlid          # controller ID
    ├── numa_node       # NUMA node number
    ├── hwmon0/
    │   ├── temp1_input # composite temperature (°C × 1000)
    │   ├── temp2_input # sensor 1 temperature
    │   └── temp[N]_input  # sensors 2-8 (if supported)
    └── nvme0n1/ (namespace block device)
        ├── nsid        # namespace ID
        ├── nguid       # Namespace Globally Unique Identifier
        ├── eui64       # 64-bit IEEE Extended Unique Identifier
        └── uuid        # NVMe 1.3+ NS UUID

/sys/class/nvme-subsystem/
└── nvme-subsys0/
    ├── subsysnqn       # subsystem NQN (matches across paths/multipath)
    ├── iopolicy        # "numa" | "round-robin" | "queue-depth"
    └── nvme0, nvme1... # multiple controllers = multipath
```

**[LINUX]** temperature ใน `/sys/class/nvme/nvmeN/hwmon*/temp_input` มีหน่วยเป็น °C × 1000 (เช่น `30850` = 30.85°C) — ไม่ใช่ Kelvin[^9]

**[LINUX]** `/sys/class/nvme-subsystem/nvme-subsysN/` เป็น key สำหรับ identify multipath: controllers หลายตัวใน subsystem เดียวกัน (subsysnqn เหมือนกัน) = multipath paths ไปยัง storage เดียว[^10]

### C.2 Persistent Device Identifiers (Priority Order)

**[SPEC]** ตามลำดับความ globally-unique:

1. **Subsystem NQN** (`subsysnqn`) — globally unique NVMe subsystem name, เช่น `nqn.2014-08.com.samsung:nvme:...`
2. **NGUID** (Namespace Globally Unique Identifier) — 128-bit, optional ใน NVMe 1.3+
3. **EUI-64** — 64-bit IEEE, optional ใน NVMe 1.0+
4. **Namespace UUID** — RFC 4122 UUID, optional ใน NVMe 1.3+
5. **Serial + Model** — fallback (ไม่ globally unique)

**[INFERENCE]** ควรพยายาม NGUID/EUI-64/UUID ก่อน แล้ว fallback ไป serial+model — ไม่ควรใช้ kernel-assigned device name (`/dev/nvme0`) เป็น persistent identifier เพราะ reassign ได้หลัง reboot

***

## D. Safe Admin-Command Allowlist

**[LINUX]** `NVME_IOCTL_ADMIN_CMD` ต้องการ `CAP_SYS_ADMIN` (root หรือ process ที่มี capability นี้) `/dev/nvmeN` มี permissions `crw------- root root` โดย default — unprivileged process เข้าไม่ถึง[^11][^12]

### Allowed (Read-Only Health Monitoring)

| Opcode | Command | CDW10 Key | NSID | Response Size | Min NVMe Ver | Safety |
|--------|---------|-----------|------|---------------|--------------|--------|
| 0x06 | Identify Controller | CNS=0x01 | 0 | 4096 B | 1.0 | ✅ Read-only |
| 0x06 | Identify Namespace | CNS=0x00 | target NSID | 4096 B | 1.0 | ✅ Read-only |
| 0x06 | Active NS List | CNS=0x02 | 0 | 4096 B | 1.1 | ✅ Read-only |
| 0x06 | NS ID Descriptor List | CNS=0x03 | target NSID | 4096 B | 1.3 | ✅ Read-only |
| 0x06 | Controller List | CNS=0x13 | 0 | 4096 B | 1.2 | ✅ Read-only |
| 0x02 | Get Log Page 0x02 (SMART) | LID=0x02 | 0xFFFFFFFF | 512 B | 1.0 | ✅ Read-only |
| 0x02 | Get Log Page 0x01 (Error) | LID=0x01 | 0xFFFFFFFF | N×64 B | 1.0 | ✅ Read-only |
| 0x02 | Get Log Page 0x03 (FW Slot) | LID=0x03 | 0 | 512 B | 1.0 | ✅ Read-only |
| 0x02 | Get Log Page 0x00 (Supported) | LID=0x00 | 0 | 4096 B | 1.3 | ✅ Read-only |
| 0x02 | Get Log Page 0x09 (Endurance Group) | LID=0x09, LSI=EGID | 0 | 512 B | 1.4 | ✅ Read-only |
| 0x02 | Get Log Page 0x0D (Persistent Event) | LID=0x0D | 0 | variable | 1.4 | ✅ Read-only |
| 0x0E | Get Features (limited) | FID safe subset | — | variable | 1.0 | ⚠️ FID-specific |

### ⛔ Forbidden Commands (ห้าม expose ใน production API)

| Opcode | Command | เหตุผล |
|--------|---------|-------|
| 0x80 | Format NVM | Destroys data |
| 0x84 | Sanitize | Wipes NVM |
| 0x10 | Create I/O Submission Queue | Changes controller state |
| 0x09 | Firmware Download | Modifies firmware |
| 0x10 | Firmware Commit | Activates firmware / reboot |
| 0x0D | Namespace Management | Creates/deletes namespaces |
| 0x15 | Security Send | Credential/encryption changes |
| 0x14 | Security Receive | Risky if misused |
| 0x11 | Device Self-Test (initiate) | Can pause I/O — analyze only, never initiate automatically |

**[ARCH-REC]** API ต้องรับ high-level request เช่น `GetSmartLog { controller: "/dev/nvme0" }` เท่านั้น ไม่รับ arbitrary CDW หรือ opcode จาก caller

***

## E. Identify Field Tables

### E.1 Identify Controller (CNS=0x01) — Key Fields

**[SPEC+LINUX]** จาก `struct nvme_id_ctrl` ใน `linux/nvme.h` และ NVMe Base Specification:[^13]

| Offset (bytes) | Field | Type | Description | Min Ver |
|----------------|-------|------|-------------|---------|
| 0–1 | `vid` | `__le16` | PCI Vendor ID | 1.0 |
| 4–23 | `sn` | `char[^20]` | Serial Number (ASCII, space-padded) | 1.0 |
| 24–63 | `mn` | `char[^40]` | Model Number (ASCII, space-padded) | 1.0 |
| 64–71 | `fr` | `char[^8]` | Firmware Revision (ASCII) | 1.0 |
| 72 | `rab` | `__u8` | Recommended Arbitration Burst | 1.0 |
| 73–75 | `ieee` | `__u8[^3]` | IEEE OUI Identifier | 1.0 |
| 76 | `cmic` | `__u8` | bit 1=multipath, bit 2=SR-IOV, bit 3=ANA | 1.3 |
| 77 | `mdts` | `__u8` | Max Data Transfer Size (2^n × 4096 bytes; 0=unlimited) | 1.0 |
| 78–79 | `cntlid` | `__le16` | Controller ID | 1.0 |
| 80–83 | `ver` | `__le32` | NVMe version (e.g., 0x00010400 = 1.4.0) | 1.2 |
| 88–91 | `oaes` | `__le32` | Optional Async Events Supported (bit 8=ANA, etc.) | 1.3 |
| 92–95 | `ctratt` | `__le32` | Controller Attributes | 1.4 |
| 111 | `cntrltype` | `__u8` | 1=I/O, 2=Discovery, 3=Admin | 2.0 |
| 112–127 | `fguid` | `__u8[^16]` | FRU GUID (128-bit) | 1.3 |
| 256–257 | `oacs` | `__le16` | Optional Admin Command Support (bit 3=NS Mgmt) | 1.0 |
| 259 | `aerl` | `__u8` | Async Event Request Limit (0-based, e.g., 3 = max 4 outstanding) | 1.0 |
| 260 | `frmw` | `__u8` | Firmware Updates (bit 4=activate without reset) | 1.0 |
| 261 | `lpa` | `__u8` | Log Page Attributes (bit 0=per-NS SMART, bit 2=telemetry, bit 4=persistent event) | 1.2 |
| 262 | `elpe` | `__u8` | Error Log Page Entries (0-based) | 1.0 |
| 264–265 | `wctemp` | `__le16` | Warning Composite Temperature Threshold (Kelvin) | 1.1 |
| 266–267 | `cctemp` | `__le16` | Critical Composite Temperature Threshold (Kelvin) | 1.1 |
| 280–295 | `tnvmcap[^16]` | `__u8[^16]` | Total NVM Capacity (bytes, 128-bit LE) | 1.4 |
| 296–311 | `unvmcap[^16]` | `__u8[^16]` | Unallocated NVM Capacity (bytes, 128-bit LE) | 1.4 |
| 520–521 | `nn` | `__le32` | Number of Namespaces | 1.0 |

### E.2 Identify Namespace (CNS=0x00) — Key Fields

**[SPEC+LINUX]** จาก NVMe Base Specification และ libnvme headers:

| Offset | Field | Description | Notes |
|--------|-------|-------------|-------|
| 0–7 | `nsze` | Namespace Size (in logical blocks) | Total capacity |
| 8–15 | `ncap` | Namespace Capacity (current usable) | May < nsze for thin-provision |
| 16–23 | `nuse` | Namespace Utilization | |
| 24 | `nsfeat` | NS Features (bit 4=NPWG alignment, bit 0=thin provision) | |
| 28–31 | `flbas` | Formatted LBA Size (bits 3:0 = LBA format index) | |
| 56–71 | `nguid[^16]` | Namespace GUID (128-bit) | NVMe 1.2+ |
| 72–79 | `eui64[^8]` | EUI-64 | NVMe 1.0+ |
| 128–384 | `lbaf[^64]` | LBA Format Support (each 4 bytes: RP, LBADS, MS) | LBADS = log2(sector_size) |

**[SPEC]** NSID ใช้ `0xFFFFFFFF` เมื่อ Get Log Page สำหรับ controller-level log เช่น SMART/Health ใช้ specific NSID เมื่อต้องการ namespace-level health (ถ้า LPA bit 0 ใน Identify Controller = 1)[^14][^4]

***

## F. SMART/Health Log Field Table

**[SPEC+LINUX]** `struct nvme_smart_log` (512 bytes, Log Identifier 0x02):[^4][^3]

| Offset | Field | Type | Description | Unit | Notes |
|--------|-------|------|-------------|------|-------|
| 0 | `critical_warning` | `u8` | Critical warning bitmask | — | ดู bit table ด้านล่าง |
| 1–2 | `temperature` | `u8[^2]` | Composite Temperature | **Kelvin** (LE 16-bit) | `u16_le - 273` = °C |
| 3 | `avail_spare` | `u8` | Available Spare | % (0–100) | ลดลงเมื่อ spare blocks ถูกใช้ |
| 4 | `spare_thresh` | `u8` | Spare Threshold | % (0–100) | AER trigger เมื่อ avail < thresh |
| 5 | `percent_used` | `u8` | **Percentage Used (Endurance)** | % | **0–100 normal; 101–254 = over-used; 255 = ≥255%**[^15][^16] |
| 6 | `endu_grp_crit_warn_sumry` | `u8` | Endurance Group Critical Warning | — | NVMe 1.4+ |
| 7–31 | `rsvd7[^25]` | — | Reserved | — | |
| 32–47 | `data_units_read[^16]` | `u8[^16]` | Data Units Read | **×1000 × 512 bytes** | 128-bit LE[^17][^4] |
| 48–63 | `data_units_written[^16]` | `u8[^16]` | **Data Units Written** | **×1000 × 512 bytes** | 128-bit LE[^17][^4] |
| 64–79 | `host_reads[^16]` | `u8[^16]` | Host Read Commands | count | 128-bit LE |
| 80–95 | `host_writes[^16]` | `u8[^16]` | Host Write Commands | count | 128-bit LE |
| 96–111 | `ctrl_busy_time[^16]` | `u8[^16]` | Controller Busy Time | **minutes** | 128-bit LE[^4] |
| 112–127 | `power_cycles[^16]` | `u8[^16]` | Power Cycles | count | 128-bit LE |
| 128–143 | `power_on_hours[^16]` | `u8[^16]` | Power-On Hours | hours | 128-bit LE |
| 144–159 | `unsafe_shutdowns[^16]` | `u8[^16]` | Unsafe Shutdowns | count | 128-bit LE[^4] |
| 160–175 | `media_errors[^16]` | `u8[^16]` | Media & Data Integrity Errors | count | 128-bit LE |
| 176–191 | `num_err_log_entries[^16]` | `u8[^16]` | Error Log Entries (lifetime) | count | 128-bit LE |
| 192–195 | `warning_temp_time` | `__le32` | Time at Warning Temperature | **minutes** | NVMe 1.2+[^4] |
| 196–199 | `critical_comp_time` | `__le32` | Time at Critical Temperature | **minutes** | NVMe 1.2+ |
| 200–215 | `temp_sensor[^8]` | `__le16[^8]` | Temperature Sensors 1–8 | **Kelvin** | 0x0000 = not implemented[^4] |
| 216–219 | `thm_temp1_trans_count` | `__le32` | TMT1 Transition Count | count | NVMe 1.4+[^4] |
| 220–223 | `thm_temp2_trans_count` | `__le32` | TMT2 Transition Count | count | NVMe 1.4+ |
| 224–227 | `thm_temp1_total_time` | `__le32` | Time in TMT1 | **seconds** | NVMe 1.4+[^4] |
| 228–231 | `thm_temp2_total_time` | `__le32` | Time in TMT2 | **seconds** | NVMe 1.4+ |
| 232–511 | `rsvd232[^280]` | — | Reserved | — | |

### critical_warning Bit Map — [SPEC]

| Bit | Name | Meaning |
|-----|------|---------|
| 0 | Spare Below Threshold | Available Spare < Available Spare Threshold |
| 1 | Temperature Warning | Composite Temp ≥ WCTEMP threshold |
| 2 | NVM Subsystem Reliability Degraded | Significant media errors / internal errors |
| 3 | Media Read-Only | NVM placed in read-only mode |
| 4 | Volatile Memory Backup Failed | Backup device failure (volatile write cache) |
| 5 | Persistent Memory Region Read-Only | NVMe 1.4+ PMR read-only |

***

## G. Enterprise/Endurance Feature Table

**[SPEC]** Endurance Group Information Log (Log Page 0x09) — NVMe 1.4+:[^18][^5]

| Field | Type | Description | Unit |
|-------|------|-------------|------|
| `available_spare_threshold` | `u8` | Spare threshold for this endurance group | % |
| `percent_used` | `u8` | Life consumed estimate (vendor-specific) | % (×1 billion) |
| `endurance_estimate[^16]` | `u8[^16]` | Total write capacity estimate for this endurance group | bytes (128-bit × 1 billion)[^5] |
| `data_units_read[^16]` | `u8[^16]` | Total host reads to endurance group | bytes (128-bit × 1 billion) |
| `data_units_written[^16]` | `u8[^16]` | Total host writes to endurance group | bytes (128-bit × 1 billion) |
| `media_units_written[^16]` | `u8[^16]` | Total media writes (host + controller) | bytes (128-bit × 1 billion) |

**[SPEC]** Log Page 0x09 ต้องการ CDW11 bits 31:16 = Endurance Group ID (EGID) — ต้อง enumerate endurance groups จาก Identify Controller field `endgid` ก่อน[^19]

**[SPEC]** ความแตกต่างของ metric scope:

| Scope | Metric Source | NSID ใน Get Log Page |
|-------|--------------|----------------------|
| Controller-wide | SMART log 0x02 | 0xFFFFFFFF |
| Per-Namespace SMART | SMART log 0x02 (ถ้า LPA bit 0 = 1) | specific NSID |
| Endurance Group | Log 0x09 | 0 (EGID ใน CDW11) |
| NVM Set | ดูจาก Identify NVM Set (NVMe 1.4+) | N/A |

***

## H. NVMe Multipath และ NVMe-oF Considerations

### H.1 Native NVMe Multipath (ANA)

**[LINUX]** Linux NVMe multipath รวม namespaces ที่มี NGUID/EUI-64 เดียวกันจากหลาย controllers เป็น single block device (`/dev/nvmeXnY`) เมื่อเปิด `nvme_core.multipath=Y`[^10]

**[LINUX]** ANA (Asymmetric Namespace Access) States ที่ kernel รองรับ:[^10]
- `optimized` — path preferred, lowest latency
- `non-optimized` — path available but suboptimal
- `inaccessible` — path down/unreachable
- `persistent-loss` — permanent path failure
- `change` — ANA state transitioning

**[ARCH-REC]** สำหรับ health monitoring บน multipath: ต้องส่ง Get Log Page ไปยังแต่ละ underlying controller (`/dev/nvme0`, `/dev/nvme1`) แยกกัน ไม่ใช่ผ่าน merged block device — merged device ไม่ support `NVME_IOCTL_ADMIN_CMD`

**[INFERENCE]** ตรวจ multipath โดยอ่าน `/sys/class/nvme-subsystem/nvme-subsysN/` และนับ controllers ที่ link อยู่ — ถ้ามีมากกว่า 1 controller ใน subsystem เดียวกัน = multipath

### H.2 NVMe-oF (Fabrics)

**[LINUX]** NVMe-oF controllers ใช้ ioctl เดียวกันทุกอย่าง — abstract ด้วย transport layer ใน kernel driver แต่มีข้อแตกต่าง:[^20]
- `state` อาจเป็น `connecting` ระหว่าง reconnect — ต้อง poll state ก่อนส่ง ioctl
- `transport` field ใน sysfs บอกว่าเป็น `tcp`, `rdma`, หรือ `fc`
- Timeout ที่เหมาะสมอาจนานกว่า PCIe เพราะ network latency

***

## I. Event-Driven AER Design

### I.1 How AER Works

**[LINUX]** Linux NVMe driver handles AER internally — kernel submits `Asynchronous Event Request` command ไปยัง controller โดยอัตโนมัติ เมื่อ AER completes kernel จะ:
1. ส่ง `uevent` ประเภท `KOBJ_CHANGE` บน `/sys/class/nvme/nvmeN`[^21][^22]
2. Set environment variable `NVME_AEN` = AER result (32-bit hex)
3. Re-submit AER command ให้ controller (re-arm)

**[LINUX]** User space ไม่ควรส่ง AER command เองผ่าน `NVME_IOCTL_ADMIN_CMD` เพราะ kernel driver จัดการอยู่แล้ว — ส่ง AER ซ้ำอาจทำให้ controller response เสียหายหรือ driver state หลุด[^23][^21]

### I.2 Recommended AER + Polling Design

**[ARCH-REC]** รูปแบบที่แนะนำ:

```
udev rule:
ACTION=="change", SUBSYSTEM=="nvme", ENV{NVME_AEN}!="", \
  RUN+="/path/to/nvme-health-helper $env{DEVPATH} $env{NVME_AEN}"

User daemon:
┌─────────────────────────────────────────────────────┐
│  udev netlink listener (event-driven trigger)       │
│  + periodic reconciliation timer (every 5–10 min)  │
├─────────────────────────────────────────────────────┤
│  On AEN trigger: read affected log page immediately │
│  On timer: read SMART log for all controllers       │
└─────────────────────────────────────────────────────┘
```

**[SPEC]** AEN Type bits 23:16 ใน result บอก log page ที่เปลี่ยน:
- `0x00` = Error status
- `0x01` = SMART/Health (bit สอดคล้องกับ critical_warning bits)[^19]
- `0x02` = Notice (namespace change, firmware activation, ANA change)

**[SPEC]** `RAE` (Retain Asynchronous Event) bit ใน CDW10 ของ Get Log Page — ถ้า RAE=0 การอ่าน log จะ clear event และ unmask future AER ของ type เดียวกัน; ถ้า RAE=1 event ยังค้างอยู่ สำหรับ AER-triggered read ต้องใช้ RAE=0 ใน final read[^23][^19]

### I.3 AERL Limit

**[SPEC]** `AERL` field ใน Identify Controller = maximum outstanding AER commands (0-based, 0 = max 1) — kernel ส่ง AER เพียง 1 command เสมอ ไม่ saturate limit

***

## J. Rust Architecture และ Traits

### J.1 Layer Diagram

```
┌────────────────────────────────────────────────────────────┐
│  HealthAggregator                                          │
│  ├── periodic_scan() → merge multi-path reports           │
│  └── on_aer_event(aen: u32) → targeted log read           │
├────────────────────────────────────────────────────────────┤
│  NvmeHealthBackend trait (per controller)                  │
│  ├── identify_controller() → IdentifyController            │
│  ├── identify_namespace(nsid) → IdentifyNamespace          │
│  ├── smart_health_log(nsid) → SmartHealthLog               │
│  ├── error_log(count) → Vec<ErrorLogEntry>                 │
│  ├── firmware_log() → FirmwareSlotLog                      │
│  └── endurance_group_log(egid) → EnduranceGroupLog         │
├────────────────────────────────────────────────────────────┤
│  NvmeTransport trait                                       │
│  ├── admin_cmd(cmd: PassthruCmd) → Result<Vec<u8>, Error>  │
│  └── controller_path() → &Path                             │
├────────────────┬───────────────┬──────────────────────────┤
│  NvmeIoctlTransport│MockTransport │UringTransport (k6.0+)  │
│  (primary)     │  (testing)    │  (optional async)        │
└────────────────┴───────────────┴──────────────────────────┘
```

### J.2 Core Types

```rust
/// From linux/nvme_ioctl.h — stable UAPI [cite: web:478]
#[repr(C)]
pub struct NvmePassthruCmd {
    pub opcode: u8,
    pub flags: u8,
    pub rsvd1: u16,
    pub nsid: u32,
    pub cdw2: u32,
    pub cdw3: u32,
    pub metadata: u64,
    pub addr: u64,
    pub metadata_len: u32,
    pub data_len: u32,
    pub cdw10: u32,
    pub cdw11: u32,
    pub cdw12: u32,
    pub cdw13: u32,
    pub cdw14: u32,
    pub cdw15: u32,
    pub timeout_ms: u32,
    pub result: u32,
}

/// NVME_IOCTL_ADMIN_CMD = _IOWR('N', 0x41, struct nvme_passthru_cmd)
/// = 0xC0484E41 on x86_64 (ioctl number computed from struct size)
/// Must verify at compile time with static assertion
const NVME_IOCTL_ADMIN_CMD: u64 = 0xC0484E41;

#[derive(Debug, Clone)]
pub enum NvmeTransportError {
    Permission(std::io::Error),     // EACCES — missing CAP_SYS_ADMIN
    NotSupported,                   // ENOTTY — device doesn't support ioctl
    DeviceGone,                     // ENODEV — controller disappeared
    CommandAborted { sct: u8, sc: u8 }, // NVMe status code
    Timeout,
    MalformedResponse(String),
    Io(std::io::Error),
}
```

***

## K. Safe Ioctl Pseudocode / Code Skeleton

```rust
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::Path;

pub struct NvmeIoctlTransport {
    fd: File,
    path: std::path::PathBuf,
}

impl NvmeIoctlTransport {
    /// Open /dev/nvme0 — requires CAP_SYS_ADMIN or root
    pub fn open(path: &Path) -> Result<Self, NvmeTransportError> {
        let fd = OpenOptions::new()
            .read(true)
            .write(false)  // O_RDONLY sufficient for admin reads
            .open(path)
            .map_err(|e| match e.raw_os_error() {
                Some(libc::EACCES) | Some(libc::EPERM) => 
                    NvmeTransportError::Permission(e),
                _ => NvmeTransportError::Io(e),
            })?;
        Ok(Self { fd, path: path.to_owned() })
    }

    /// Execute admin passthrough ioctl (read-only commands only)
    pub fn admin_passthru(
        &self,
        opcode: u8,
        nsid: u32,
        cdw10: u32,
        cdw11: u32,
        cdw12: u32,
        buf: &mut Vec<u8>,
        timeout_ms: u32,
    ) -> Result<u32, NvmeTransportError> {
        // Safety: buf must be pinned during ioctl syscall
        let mut cmd = NvmePassthruCmd {
            opcode,
            flags: 0,
            rsvd1: 0,
            nsid,
            cdw2: 0, cdw3: 0,
            metadata: 0,
            addr: buf.as_mut_ptr() as u64,
            metadata_len: 0,
            data_len: buf.len() as u32,
            cdw10,
            cdw11,
            cdw12,
            cdw13: 0, cdw14: 0, cdw15: 0,
            timeout_ms,
            result: 0,
        };

        let ret = unsafe {
            libc::ioctl(
                self.fd.as_raw_fd(),
                NVME_IOCTL_ADMIN_CMD,
                &mut cmd as *mut _,
            )
        };

        if ret < 0 {
            return Err(map_ioctl_errno(std::io::Error::last_os_error()));
        }

        // cmd.result contains NVMe completion queue DW0
        // bits 25:17 = SCT (status code type), bits 16:9 = SC (status code)
        let sc = (cmd.result >> 1) & 0xFF;
        let sct = (cmd.result >> 9) & 0x07;
        if sct != 0 || sc != 0 {
            return Err(NvmeTransportError::CommandAborted { 
                sct: sct as u8, sc: sc as u8 
            });
        }

        Ok(cmd.result)
    }
}

/// Get SMART/Health log — NSID=0xFFFFFFFF for controller-level
pub fn get_smart_health_log(
    transport: &NvmeIoctlTransport,
) -> Result<SmartHealthLog, NvmeTransportError> {
    let mut buf = vec![0u8; 512];
    // Opcode 0x02 = Get Log Page
    // CDW10: LID=0x02, NUMDL=127 (0-based DWORD count for 512 bytes)
    // CDW11: RAE=0, NUMDU=0
    let cdw10: u32 = 0x02 | (127 << 16);  // LID + NUMDL (512/4 - 1 = 127)
    transport.admin_passthru(
        0x02, 0xFFFF_FFFF, cdw10, 0, 0,
        &mut buf, 5000
    )?;
    parse_smart_health_log(&buf)
}

/// Identify Controller — CNS=0x01
pub fn identify_controller(
    transport: &NvmeIoctlTransport,
) -> Result<IdentifyController, NvmeTransportError> {
    let mut buf = vec![0u8; 4096];
    let cdw10: u32 = 0x01; // CNS=0x01
    transport.admin_passthru(0x06, 0, cdw10, 0, 0, &mut buf, 5000)?;
    parse_identify_controller(&buf)
}

/// Tokio integration: ioctl is blocking → must spawn_blocking
pub async fn get_smart_async(
    transport: Arc<NvmeIoctlTransport>,
) -> Result<SmartHealthLog, NvmeTransportError> {
    tokio::task::spawn_blocking(move || {
        get_smart_health_log(&transport)
    })
    .await
    .map_err(|e| NvmeTransportError::Io(
        std::io::Error::new(std::io::ErrorKind::Other, e)
    ))?
}
```

**[ARCH-REC]** ioctl number `0xC0484E41` คำนวณจาก `_IOWR('N', 0x41, struct_size=72_bytes)` บน x86_64 — ต้องยืนยันด้วย `static_assert!(std::mem::size_of::<NvmePassthruCmd>() == 72)` และ คำนวณ ioctl number ที่ compile time หรือใช้ `nix::ioctl_readwrite!` macro

***

## L. Parsing, Endian และ u128 Strategy

### L.1 Little-Endian 128-bit Counter Parsing

**[SPEC]** Fields เช่น `data_units_written[^16]` เป็น 16 bytes ลำดับ little-endian แทน 128-bit unsigned integer[^17][^4]

```rust
/// Parse 128-bit LE counter from 16-byte slice
pub fn parse_u128_le(bytes: &[u8; 16]) -> u128 {
    u128::from_le_bytes(*bytes)
}

/// Data Units Written → actual bytes written (approximate)
/// NVMe spec: value in units of 1000 × 512 bytes, rounded up
pub fn data_units_to_bytes(units: u128) -> u128 {
    units * 1000 * 512
}

/// Example:
/// data_units_written raw = 35_670_814
/// → 35_670_814 × 1000 × 512 = ~18.2 TB [consistent with cite: web:498]
```

**[SPEC]** `data_units_written` = จำนวน "units" โดย 1 unit = 1000 × 512 bytes (rounded up) — ค่า `1` หมายความว่าเขียนข้อมูล 1–1000 units ของ 512 bytes = 512–512,000 bytes[^17][^4]

### L.2 Temperature Parsing

```rust
/// Composite Temperature: 2 bytes LE = Kelvin
pub fn parse_composite_temp_celsius(raw: &[u8; 2]) -> Option<i16> {
    let kelvin = u16::from_le_bytes(*raw);
    if kelvin == 0 { return None; }
    Some(kelvin as i16 - 273)
}

/// Temperature sensors: same encoding, 0x0000 = not implemented
pub fn parse_temp_sensor(raw: u16) -> Option<i16> {
    if raw == 0 { None } else { Some(raw as i16 - 273) }
}
```

### L.3 NVMe Version Parsing

```rust
/// ver field in Identify Controller: __le32
/// Format: bits 31:16 = MJR, bits 15:8 = MNR, bits 7:0 = TER
pub struct NvmeVersion { pub major: u16, pub minor: u8, pub tertiary: u8 }
pub fn parse_nvme_version(ver: u32) -> NvmeVersion {
    NvmeVersion {
        major: (ver >> 16) as u16,
        minor: ((ver >> 8) & 0xFF) as u8,
        tertiary: (ver & 0xFF) as u8,
    }
}
// e.g., 0x00020000 = NVMe 2.0.0
```

### L.4 MDTS Enforcement

**[SPEC]** `mdts` ใน Identify Controller = log2 factor ของ minimum memory page size (4096 bytes) — ถ้า `mdts=5`, max transfer = 2^5 × 4096 = 131072 bytes — ต้องแบ่ง Get Log Page เป็นหลาย requests ถ้า log size > MDTS[^19]

```rust
pub fn max_transfer_bytes(mdts: u8, min_page_size: u32) -> Option<u32> {
    if mdts == 0 { None } else { Some(min_page_size << mdts) }
}
```

***

## M. Security Model

**[LINUX]** `/dev/nvme0` (character device) มี permissions `crw------- 1 root root` โดย default — ต้องเป็น root หรือมี `CAP_SYS_ADMIN` `/dev/nvme0n1` (block device) มี permissions `brw-rw---- 1 root disk` — accessible สำหรับ disk group members แต่ `NVME_IOCTL_ADMIN_CMD` ยังต้อง open `/dev/nvme0` (char device) ไม่ใช่ block device[^11]

**[LINUX]** kernel 6.0+ restrict management ioctls (RESET, SUBSYS_RESET, RESCAN) ให้ต้องการ `CAP_SYS_ADMIN` อย่างชัดเจน passthrough commands (`NVME_IOCTL_ADMIN_CMD`) มีการ check permission ใน kernel เช่นกัน[^6][^12]

**[LINUX]** systemd NVMe udev rule ที่แนะนำสำหรับ monitoring group:[^11]
```
SUBSYSTEM=="nvme", KERNEL=="nvme[0-9]*", GROUP="disk", MODE="0640"
```

**[ARCH-REC]** Privilege separation model:

```
┌────────────────────────────────────────────────────────┐
│  nvme-health-daemon (user space, no privilege)         │
│  ├── Reads /sys/class/nvme/*/state (no privilege)      │
│  ├── Reads hwmon/temp_input (no privilege)             │
│  └── Sends requests via Unix socket → privileged helper│
├────────────────────────────────────────────────────────┤
│  nvme-health-helper (setuid or CAP_SYS_ADMIN)          │
│  ├── Validates device path pattern: /dev/nvme[0-9]+    │
│  ├── Command allowlist: opcodes 0x02, 0x06 only        │
│  ├── Rejects CDW15 != 0 or any write-direction command │
│  ├── Timeout: 10s per command max                      │
│  └── Returns serialized response, not raw buffer      │
└────────────────────────────────────────────────────────┘
```

***

## N. Test and Hardware Validation Matrix

### N.1 Fixture Strategy

**[ARCH-REC]** inject mock transport โดย implement `NvmeTransport` trait บน `MockTransport`:

```rust
pub struct MockTransport {
    /// Keyed by (opcode, nsid, cdw10) → response bytes
    responses: HashMap<(u8, u32, u32), Vec<u8>>,
}

impl NvmeTransport for MockTransport {
    fn admin_cmd(&self, cmd: &PassthruCmd) -> Result<Vec<u8>, NvmeTransportError> {
        self.responses
            .get(&(cmd.opcode, cmd.nsid, cmd.cdw10))
            .cloned()
            .ok_or(NvmeTransportError::NotSupported)
    }
}
```

**Fixture collection:** ใช้ `nvme get-log /dev/nvme0 --log-id=2 --output-format=binary` หรือ `nvme id-ctrl /dev/nvme0 --output-format=binary` เป็น development oracle — ไม่ใช่ runtime dependency

### N.2 QEMU NVMe Emulation

**[LINUX]** QEMU รองรับ NVMe emulation ด้วย `-device nvme,drive=nvme0,serial=TEST1234` — รองรับ Identify Controller, Identify Namespace, Get Log Page (SMART log จะ return mock values)[^24][^25]

**[INFERENCE]** QEMU NVMe emulation limitations:
- SMART log fields ส่วนใหญ่เป็น 0 (ยกเว้น power cycles, power-on hours)
- Endurance Group (NVMe 1.4+) อาจ return error หรือ empty ขึ้นกับ QEMU version
- AER events ต้อง trigger manually ผ่าน qemu monitor — ยากสำหรับ automated testing

### N.3 Hardware Test Matrix

| Test Scenario | Device Type | Expected Result | Confidence |
|---------------|-------------|-----------------|-----------|
| Basic SMART read | PCIe NVMe (any) | Full SMART log parsed | High |
| percent_used > 100 | Used enterprise SSD | Handled as 101–254 (not error) | Medium |
| NVMe 1.3 (older device) | Samsung 960 EVO etc. | No `endu_grp_crit_warn_sumry` field | High |
| NVMe 2.0 | Recent enterprise | All fields present | High |
| Multipath (2 controllers) | Dual-port enterprise | Health read from each path separately | **NEEDS HARDWARE** |
| NVMe-oF TCP | Fabric target | Same ioctl, state management | **NEEDS HARDWARE** |
| Controller reset during read | Any | `ENODEV` or timeout | Medium |
| Device removal (hot-unplug) | Hot-plug capable | `DeviceGone` error | **NEEDS HARDWARE** |
| No permission (no root) | Any | `Permission` error, no panic | High (easy to test) |

### N.4 Fuzz Targets

```rust
// Fuzz SMART log parser
fuzz_target!(|data: &[u8]| {
    if data.len() >= 512 {
        let buf: [u8; 512] = data[..512].try_into().unwrap();
        let _ = parse_smart_health_log(&buf);  // must not panic
    }
});

// Fuzz Identify Controller parser
fuzz_target!(|data: &[u8]| {
    if data.len() >= 4096 {
        let _ = parse_identify_controller(&data[..4096]);
    }
});
```

**[ARCH-REC]** ต้อง fuzz อย่างน้อย: `parse_smart_health_log`, `parse_identify_controller`, `parse_identify_namespace`, `parse_error_log_entry`, `parse_u128_le` ด้วย truncated/all-zero/all-0xFF inputs

***

## O. MVP → Production Roadmap

### MVP (3–4 Weeks)

**Week 1:** Core ioctl layer
- `NvmePassthruCmd` struct (repr(C) + size assertion)
- `NvmeIoctlTransport` + `NVME_IOCTL_ADMIN_CMD` binding
- Error enum + errno mapping

**Week 2:** Identify + SMART parsers
- Identify Controller parser (model, serial, firmware, version)
- SMART Health log parser — all fields including u128 counters + temperature
- 5 vendor fixtures (Samsung, Intel, Crucial, WD, Seagate)

**Week 3:** Device discovery + namespace
- `/sys/class/nvme` scan → controller list
- Identify Namespace parser (capacity, sector size, NGUID/EUI-64)
- Active NS list query
- Persistent identifier resolution (NGUID > EUI-64 > serial)

**Week 4:** Tokio integration + error handling
- `spawn_blocking` wrapper สำหรับ async context
- Full error handling + fixture-based unit tests
- sysfs temperature fallback (hwmon)

### Production Hardening (3–4 Weeks)

**Week 5:** Additional log pages
- Error Information log (0x01) parser
- Firmware Slot log (0x03) parser
- Supported Log Pages (0x00) probe

**Week 6:** Enterprise features
- Endurance Group log (0x09) — NVMe 1.4+
- Persistent Event log (0x0D) — NVMe 1.4+
- Per-namespace SMART (LPA bit 0 check)

**Week 7:** AER event-driven monitoring
- udev netlink listener via `mio` or `tokio`
- RAE-aware log re-read
- udev rule deployment

**Week 8:** Security + testing
- Privilege separation helper
- Fuzzing (cargo-fuzz) สำหรับ parsers
- Hardware test matrix documentation

***

## P. Primary Source Links

> ⚠️ NVMe Base Specification ซื้อได้จาก NVM Express Industry Association (nvmexpress.org) แต่ NVMe 2.0 public review draft มีอยู่บน web

| Source | Reference |
|--------|-----------|
| Linux UAPI nvme_ioctl.h | https://github.com/torvalds/linux/blob/master/include/uapi/linux/nvme_ioctl.h [^1] |
| Linux nvme.h (kernel internal) | https://github.com/torvalds/linux/blob/master/include/linux/nvme.h [^26] |
| Linux nvme ioctl.c (kernel 6.x) | https://codebrowser.dev/linux/linux/drivers/nvme/host/ioctl.c.html [^27] |
| Linux NVMe multipath docs | https://docs.kernel.org/admin-guide/nvme-multipath.html [^10] |
| libnvme (LGPL-2.1) — struct definitions | https://github.com/linux-nvme/libnvme [^28][^29] |
| libnvme Rust bindings | https://docs.rs/libnvme/latest/libnvme/ [^30] |
| nvme_smart_log man page (Debian/Ubuntu) | https://manpages.debian.org/testing/libnvme-dev/nvme_smart_log.2.en.html [^3] |
| struct nvme_id_ctrl man page | https://manpages.ubuntu.com/manpages/jammy/man2/nvme_id_ctrl.2.html [^13] |
| NVMe permission issues (systemd) | https://github.com/systemd/systemd/issues/26009 [^11] |
| Kernel restrict management ioctl (kernel 6.0) | https://lists-ec2.96boards.org/archives/list/...IGCD6AGIFRI6TRMW6SAD7XQCET3TFVQW/ [^6] |
| AER uevent design (kernel mailing list) | http://lists.infradead.org/pipermail/linux-nvme/2017-July/011757.html [^21] |
| NVME_ENDURANCE_GROUP_LOG structure | https://learn.microsoft.com/en-us/windows/win32/api/nvme/ns-nvme-nvme_endurance_group_log [^5] |
| Get Log Page CDW10/CDW11 (NVMe 1.3+) | https://learn.microsoft.com/en-us/windows/win32/api/nvme/ns-nvme-nvme_cdw10_get_log_page [^31] |
| Data Units Written unit definition | https://github.com/linux-nvme/nvme-cli/issues/1558 [^17] |
| percent_used saturation behavior | https://forums.tomshardware.com/threads/how-is-the-remaining-ssd-life-calculated.3740758/ [^15] |
| NVMe sysfs data summary | https://utcc.utoronto.ca/~cks/space/blog/linux/NVMeSysfsData [^9] |
| QEMU NVMe emulation docs | https://qemu-project.gitlab.io/qemu/system/devices/nvme.html [^25] |
| Intel Optane SMART attributes | https://www.intel.com/content/www/us/en/support/articles/000056596/... [^16] |
| Oracle NVMe SSD spec (percent_used note) | https://docs.oracle.com/en/servers/options/hdd-ssd/ssd-specifications/... [^32] |

### Licensing Summary

| Library | License | Usage |
|---------|---------|-------|
| libnvme (linux-nvme) | **LGPL-2.1-only**[^33][^29] | Rust bindings ผ่าน `libnvme` crate ได้ — LGPL อนุญาต dynamic linking โดยไม่ต้อง GPL project |
| nvme-cli | GPL-2.0 | Reference only — ห้าม copy code เข้า MIT/Apache project |
| Linux UAPI headers | GPL-2.0 with syscall exception | ใช้ struct definitions ได้เสรีใน userspace[^1] |

**[ARCH-REC]** ถ้าต้องการ pure-Rust โดยไม่ใช้ FFI กับ libnvme: implement struct definitions ตาม UAPI headers โดยตรง (อนุญาตตาม syscall exception) — ดู approach ของ `smartmontools/linux_nvme_ioctl.h` เป็น reference[^2]

***

## ข้อสรุปที่ยังต้องยืนยันด้วย Hardware

1. **[NEEDS HARDWARE]** NVMe multipath health read behavior — ต้องทดสอบว่า admin ioctl บน merged block device return error อย่างไร บน Linux kernel versions ต่างๆ
2. **[NEEDS HARDWARE]** Endurance Group log บน real enterprise SSD (EGID enumeration flow)
3. **[NEEDS HARDWARE]** NVMe-oF TCP reconnect behavior ระหว่าง admin ioctl in-flight
4. **[NEEDS KERNEL VERSION TEST]** `io_uring` admin command (NVME_URING_CMD_ADMIN) — ต้อง kernel ≥ 6.0 ยืนยัน behavior บน 6.0, 6.1, 6.6
5. **[NEEDS TEST]** AER uevent timing — kernel มักจะ buffer AENs และส่ง uevent หลัง log read — ต้องทดสอบว่า event มาทันหรือต้องมี fallback polling

---

## References

1. [include/uapi/linux/nvme_ioctl.h - pub/scm/linux/kernel/git/ ...](https://kernel.googlesource.com/pub/scm/linux/kernel/git/thierry.reding/linux-pwm/+/for-next/include/uapi/linux/nvme_ioctl.h)

2. [◆ nvme_admin_cmd](https://www.smartmontools.org/static/doxygen/linux__nvme__ioctl_8h.html)

3. [nvme_smart_log(2) — libnvme-dev — Debian testing](https://manpages.debian.org/testing/libnvme-dev/nvme_smart_log.2.en.html)

4. [struct nvme_smart_log - SMART / Health ... - Ubuntu Manpage](https://manpages.ubuntu.com/manpages/questing/man2/nvme_smart_log.2.html)

5. [NVME_ENDURANCE_GROUP_...](https://learn.microsoft.com/en-us/windows/win32/api/nvme/ns-nvme-nvme_endurance_group_log) - Contains fields that specify the information in an Endurance Group Information log page that indicat...

6. [[PATCH 6.0 281/314] nvme: restrict management ioctls to admin](https://lists-ec2.96boards.org/archives/list/linux-stable-mirror@lists.linaro.org/message/IGCD6AGIFRI6TRMW6SAD7XQCET3TFVQW/)

7. [Linux v6.6.1 - drivers/nvme/host/sysfs.c](https://sbexr.rabexc.org/latest/sources/c3/45d0445a27bb87.html)

8. [nvme list: missing namespace info? · Issue #1983 · linux-nvme/nvme-cli](https://github.com/linux-nvme/nvme-cli/issues/1983) - I'm not sure whether this is normal behavior or a bug. Hoping someone will be able the shed light. O...

9. [What data about your NVMe drives Linux puts in sysfs](https://utcc.utoronto.ca/~cks/space/blog/linux/NVMeSysfsData)

10. [Linux NVMe multipath - The Linux Kernel documentation](https://docs.kernel.org/admin-guide/nvme-multipath.html)

11. [NVMe devices have inconsistent permissions · Issue #26009 · systemd/systemd](https://github.com/systemd/systemd/issues/26009) - systemd version the issue has been seen with 252.1 with NixOS patches Used distribution NixOS, follo...

12. [[PATCH] nvme: restrict management ioctls to admin - Mailing Lists](http://lists.infradead.org/pipermail/linux-nvme/2022-September/034660.html)

13. [struct nvme_id_ctrl - Identify Controller data ... - Ubuntu Manpage](https://manpages.ubuntu.com/manpages/jammy/man2/nvme_id_ctrl.2.html)

14. [libnvme/doc/man/nvme_mi_admin_get_log_smart.2 at master · linux-nvme/libnvme](https://github.com/linux-nvme/libnvme/blob/master/doc/man/nvme_mi_admin_get_log_smart.2) - C Library for NVM Express on Linux. Contribute to linux-nvme/libnvme development by creating an acco...

15. [how is the remaining SSD life calculated?? | Tom's Hardware Forum](https://forums.tomshardware.com/threads/how-is-the-remaining-ssd-life-calculated.3740758/) - someone is selling a Chia plotting SSD claiming 50% life left... Then the host writes is nearly 2PB ...

16. [Common SMART Attributes for Intel® Optane™ Technology Products](https://www.intel.com/content/www/us/en/support/articles/000056596/memory-and-storage/client-ssds.html)

17. [Get SMART attributes by id · Issue #1558 · linux-nvme/nvme-cli](https://github.com/linux-nvme/nvme-cli/issues/1558) - Hi, I'm trying to get SMART attributes 247 and 248 on my Samsung SSD 970 EVO Plus 1TB using nvme ctl...

18. [NVME_ENDURANCE_GROUP_LOG - Win32 apps](https://learn.microsoft.com/ja-jp/windows/win32/api/nvme/ns-nvme-nvme_endurance_group_log) - 耐久グループから読み取られ、耐久グループに書き込まれるデータの量を示す耐久グループ情報ログ ページの情報を指定するフィールドが含まれます。

19. [NVMe1.4 Admin Command学习（7) get log page 原创](https://blog.csdn.net/weixin_40581738/article/details/114577585) - 文章浏览阅读5.6k次，点赞3次，收藏17次。本文介绍了GetLogPage命令的工作原理及参数配置，包括如何通过指定日志页标识符获取特定日志信息，如错误日志（logid=0x1）。此外还详细解释了命...

20. [nvme: add fabrics sysfs attributes · 1a353d85b0 - linux - Gitea](https://gitea.basealt.ru/iv/linux/commit/1a353d85b02d010e9daa7bd189d203ba1f2614a1) - - delete_controller: This attribute allows to delete a controller. A driver is not obligated to supp...

21. [[PATCH 7/7] nvme: Send change uevent when AEN ...](https://lists.openwrt.org/pipermail/linux-nvme/2017-July/011757.html)

22. [include/linux/nvme.h · d8a5b80568a9cb66810e75b182018e9edb68e8ff · Dennis Giaya / linux · GitLab](https://gitlab-external-production.whoi.edu/dgiaya/linux/-/blob/d8a5b80568a9cb66810e75b182018e9edb68e8ff/include/linux/nvme.h) - forked from torvalds/linux

23. [Missing AENs · Issue #17 · linux-nvme/libnvme](https://github.com/linux-nvme/libnvme/issues/17) - Hi @keithbusch CC: @hreinecke I use libnvme to connect to a discovery controller. I create a persist...

24. [nvme-env/docker/qemu-nvme/README.md at master · ljishen/nvme-env](https://github.com/ljishen/nvme-env/blob/master/docker/qemu-nvme/README.md) - A prototyping environment that runs an emulated NVMe device - ljishen/nvme-env

25. [NVMe Emulation](https://qemu-project.gitlab.io/qemu/system/devices/nvme.html)

26. [linux/include/linux/nvme.h at master · torvalds/linux - GitHub](https://github.com/torvalds/linux/blob/master/include/linux/nvme.h) - Linux kernel source tree. Contribute to torvalds/linux development by creating an account on GitHub.

27. [ioctl.c source code [linux/drivers/nvme/host ...](https://codebrowser.dev/linux/linux/drivers/nvme/host/ioctl.c.html) - Source code of linux/drivers/nvme/host/ioctl.c linux v6.16-r on KDAB Codebrowser

28. [GitHub - linux-nvme/libnvme: C Library for NVM Express on Linux](https://github.com/linux-nvme/libnvme) - C Library for NVM Express on Linux. Contribute to linux-nvme/libnvme development by creating an acco...

29. [linux-nvme](https://github.com/orgs/linux-nvme/repositories) - linux-nvme has 8 repositories available. Follow their code on GitHub.

30. [libnvme - Rust - Docs.rs](https://docs.rs/libnvme/latest/libnvme/) - Safe, idiomatic Rust bindings for the Linux `libnvme` C library.

31. [NVME_CDW10_GET_LOG_PAGE - Win32 apps](https://learn.microsoft.com/en-us/windows/win32/api/nvme/ns-nvme-nvme_cdw10_get_log_page) - The NVME_CDW10_GET_LOG_PAGE structure contains parameters for the Get Log Page command that returns ...

32. [480GB, M.2 NVMe Solid State Drive Specification - Oracle Help Center](https://docs.oracle.com/en/servers/options/hdd-ssd/ssd-specifications/foureighty-mtwo-nvme-ssd-spec/index.html) - Percentages greater than 254 are represented as 255. Percentages greater than 254 are represented as...

33. [libnvme 1.16.1-3 (x86_64) - Arch Linux](https://archlinux.org/packages/extra/x86_64/libnvme/)

40. [Documentation ¶](https://pkg.go.dev/github.com/dswarbrick/go-nvme/nvme)

64. [NVME_LOG_PAGES - Win32 apps](https://learn.microsoft.com/en-us/windows/win32/api/nvme/ne-nvme-nvme_log_pages) - Contains values that indicate the log pages that can be retrieved by the Get Log Page **NVME_ADMIN_C...

280. [Understanding SCSI Sense](https://blog.csdn.net/kinges/article/details/49276771) - 文章浏览阅读2.6k次。This page is about decoding and interpreting the SCSI sense buffer in order totroublesho...

