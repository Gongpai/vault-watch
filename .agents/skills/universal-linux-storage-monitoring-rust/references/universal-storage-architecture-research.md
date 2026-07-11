# Universal Storage Discovery และ Backend Architecture สำหรับ Linux Storage Monitoring ด้วย Rust

## สรุปภาพรวมและเมทริกซ์ความสามารถ

**Executive summary**

**ข้อเท็จจริงจากเอกสาร:** บน Linux นั้น block topology ที่ userspace มองเห็นได้ไม่ได้เป็นต้นไม้ดิสก์จริงแบบหนึ่งต่อหนึ่ง แต่เป็นชุดของ block devices และลิงก์ใน sysfs ที่รวมทั้ง “whole disks”, partitions, stacked devices, และ virtual devices ไว้ใน namespace เดียวกัน โดย `/sys/class/block` เป็นรายการแบบ flat ที่มีทั้งดิสก์และพาร์ทิชันปะปนกัน และการอาศัยโครงสร้างชื่อหรือการจัดวาง path แบบตายตัวถือเป็นบั๊กตามกฎ sysfs เอง ขณะเดียวกัน sysfs ถูกออกแบบมาเพื่อส่งออกทั้ง attributes และ linkages ของ kernel objects ให้ userspace ใช้งาน ส่วน block layer มี stable ABI สำหรับ `diskseq`, `hidden`, และ `stat`; device-mapper, FC, USB, และ subsystem-specific attributes จำนวนมากยังอยู่ในกลุ่ม testing ABI; และ `/sys/devices/...` สะท้อน device hierarchy จริงแต่ชื่อ path ย่อยจำนวนมากยังเป็น implementation detail มากกว่าจะเป็น ABI ที่ควรผูกติดแบบตายตัว. citeturn34search8turn34search5turn35search3turn32view4turn32view5turn25search2turn8search6turn27search9

**Inference:** ดังนั้น software ที่ต้อง “discover ทุกอย่าง” อย่างปลอดภัยควรแยกปัญหาออกเป็นอย่างน้อยสามชั้นตั้งแต่แรก คือ `identity`, `health scope`, และ `throughput scope` แทนที่จะถามว่า “/dev/X คือดิสก์ลูกไหน” เพราะคำถามนั้นใช้ไม่ได้กับ dm, md, multipath, remote LUN, hardware RAID logical volume, NVMe namespaces, virtio-blk, loop/NBD, และ USB bridges หลายชนิด. การพยายามบังคับ map block device ทุกตัวไป physical disk หนึ่งตัวจะทำให้ topology, metrics, และ UX ผิดพลาดอย่างเป็นระบบ. citeturn34search8turn42search12turn25search0turn39search6turn31search2

**ข้อเสนอแนะเชิงสถาปัตยกรรม:** โมเดลที่เหมาะสมที่สุดคือ “graph-first, backend-late”. กล่าวคือให้ discovery phase สร้าง graph มาตรฐานจาก kernel interfaces ก่อน โดยใช้ stable ABI เป็นแกน, ใช้ testing ABI เป็น enrichment แบบมี feature gate, และถือชื่อ path ใต้ `/sys/devices`, by-id symlink shape, และ vendor leaf attributes เป็นเพียง hints. หลังจากจัดชั้น node สำเร็จแล้วจึงให้ backend router เลือก health backend ที่เหมาะสมตาม “capability probing + safety score” ไม่ใช่จากชื่ออุปกรณ์อย่างเดียว.

**แนวทางแยกประเภท interface**

| ระดับ | ควรใช้ทำอะไร | ตัวอย่าง |
|---|---|---|
| Stable ABI | เป็น source of truth หลักสำหรับ discovery และ counters | `/sys/block/*`, `diskseq`, `hidden`, `stat`, queue limits |
| Testing ABI | enrich topology/identity เมื่อมีอยู่ และต้อง probe แบบ optional | `/sys/block/dm-*/dm/*`, FC class, USB attrs, controller-specific sysfs |
| Implementation detail | ใช้ได้เป็น hint หรือ cache key ชั่วคราวเท่านั้น | ชื่อ path ใต้ `/sys/devices`, ลำดับเลข device, symlink naming pattern |
| Recommendation | policy ของโปรแกรม | backend scoring, generation IDs, metric scope labels |

ตารางนี้เป็น architectural synthesis จากเอกสาร ABI/sysfs ของ kernel และ subsystem docs. citeturn35search3turn25search2turn34search8turn34search5

**Storage-type capability matrix**

ตารางนี้เป็น **inference + recommendation** สำหรับ product design โดยยึดตามสิ่งที่ kernel/subsystems เปิดเผยให้ userspace ได้จริงในปัจจุบัน ไม่ใช่สัญญาว่าอุปกรณ์ทุกตัวจะให้ health metrics ได้เสมอ

| ประเภท storage | Discovery หลัก | Health backend ที่ควรลองก่อน | Scope ที่คาดหวังได้ | สถานะที่ควรรายงาน |
|---|---|---|---|---|
| Native SAS/SCSI HDD/SSD | SCSI + block sysfs + VPD | SCSI SG_IO | physical device หรือ logical unit | มักรองรับ |
| SATA ผ่าน AHCI/libata | block sysfs ภายใต้ SCSI/libata | ATA identity จาก cached identify; ถ้าต้อง probe ใช้ SG_IO/SAT เมื่อปลอดภัย | physical device | มักรองรับ |
| SATA ผ่าน SAT/SAS HBA | SCSI device + SAT capability | SAT/ATA passthrough ผ่าน SG_IO | physical SATA device หลัง bridge/HBA | รองรับบางตัว |
| NVMe SSD | NVMe controller/namespace + nvme ioctls | NVMe admin ioctl | controller / subsystem / namespace | มักรองรับ |
| NVMe over TCP/RDMA/FC | NVMe-oF subsystem/controller/namespace | NVMe admin ioctl | namespace/subsystem; physical media มักไม่โปร่งใส | รองรับแต่ scope จำกัด |
| USB-to-SATA | USB parent + SCSI/UAS + SAT probe | SAT ผ่าน SG_IO ถ้า bridge รองรับ | physical device หลัง USB bridge | รองรับบาง bridge |
| USB-to-NVMe | USB parent + มักโผล่เป็น SCSI/UAS ไม่ใช่ native NVMe | vendor/bridge backend ถ้ามี; ไม่เช่นนั้น none | logical device หลัง bridge | มัก unsupported |
| SD/microSD native | mmcblk + MMC sysfs/UAPI | MMC ioctl | card/device scope จำกัด | รองรับบาง metrics |
| eMMC | mmcblk + device partitions | MMC ioctl | device/card scope | รองรับบาง metrics |
| virtio-blk | virtio + block sysfs | ไม่มี generic physical health | logical virtual disk | health physical ไม่รองรับ |
| SCSI virtual disk | SCSI + hypervisor/controller | SCSI VPD/standard inquiry เท่าที่มี | logical LUN | สุขภาพ physical มักซ่อน |
| dm, LVM, dm-crypt | dm sysfs + slaves/holders | ไม่มี direct physical health; aggregate topology only | logical/transform layer | health direct ไม่รองรับ |
| multipath | dm + hidden paths + WWID/paths | path-level SCSI/NVMe backends + multipath aggregate state | path / namespace-LUN / multipath device | ต้องระวังนับซ้ำ |
| Linux MD RAID | md sysfs | MD sysfs + member backends | array + member | รองรับดี |
| hardware RAID logical volume | SCSI/block/PCI controller path | vendor/controller backend ถ้ามี | logical volume / controller | physical members มักซ่อน |
| iSCSI / FC LUNs | SCSI transport + FC/iSCSI classes | SCSI SG_IO | logical unit / path / target port | physical media ไม่โปร่งใส |
| loop / NBD / other virtual block | loop/rnbd/virt/ublk sysfs | ไม่มี physical health | virtual/logical backing | unsupported สำหรับ health |

สารตั้งต้นของ matrix นี้มาจาก libata ที่ทำให้ SATA จำนวนมากถูกนำเสนอผ่าน SCSI, SCSI transport classes และ sysfs, NVMe UAPI, MMC UAPI, MD sysfs, dm sysfs, USB tree model, virtio spec, และ loop/rnbd sysfs. citeturn6view5turn29view1turn29view2turn21view0turn23search0turn35search8turn25search4turn27search0turn27search9turn31search1turn31search2turn30search1

## โมเดล topology มาตรฐานและอัลกอริทึมค้นหา

**Canonical topology graph model**

**ข้อเท็จจริงจากเอกสาร:** sysfs ส่งออกทั้ง attributes และความเชื่อมโยงของ kernel objects; block subsystem เป็น flat list; holder/slave symlinks ถูกสร้างขึ้นสำหรับ stacking drivers เช่น DM/MD; MD มีทั้ง view ของ array และสมาชิกใน `/sys/block/mdX/md/...`; device-mapper มี dm-specific UUID/name; Thunderbolt, USB, PCI, และ FC มี parent hierarchies ของตนเองใน sysfs; FC transport class ยัง export remote ports พร้อม WWPN/WWNN/port state; และ NVMe-oF specification มองระบบเป็น `subsystem -> controller(s) -> namespace(s) -> port(s)` ไม่ใช่ “disk ลูกเดียวต่อ block device”. citeturn34search5turn34search8turn42search12turn35search8turn25search4turn28search1turn27search9turn33search13turn29view0turn39search6

**Recommendation:** graph ควรมีอย่างน้อย node types ดังนี้

- `BlockDevice`
- `Partition`
- `DmMap`
- `MdArray`
- `NvmeSubsystem`
- `NvmeController`
- `NvmeNamespace`
- `ScsiHost`
- `ScsiTarget`
- `ScsiLun`
- `UsbDevice`
- `PciFunction`
- `ThunderboltDevice`
- `TransportEndpoint`
- `PhysicalMediumCandidate`
- `VirtualBackingObject`

และ edge types ควรแยก semantic ให้ชัด เช่น `parent_bus`, `contains_partition`, `maps_to`, `member_of`, `exports_block`, `path_to`, `same_wwid_group`, `controller_of`, `namespace_of`, และ `backed_by_file`.

**ข้อเสนอแนะเชิงสถาปัตยกรรม:** อย่าพยายามใช้ graph เป็นต้นไม้ เพราะ storage บน Linux มีทั้ง fan-in และ fan-out. ตัวอย่างเช่น

- dm-multipath หนึ่งตัวมีหลาย path ลงไปยังหลาย SCSI LUN paths
- NVMe multipath อาจมีหลาย controller/path ไปยัง namespace เดียว
- MD RAID หนึ่ง array fan-in จากหลายสมาชิก
- partitions fan-out จาก whole disk
- logical volume fan-in/fan-out ผ่าน stacked transforms หลายชั้น

representation ที่ปลอดภัยที่สุดคือ **directed multigraph** พร้อม typed edges และ explicit layer numbers ที่คำนวณภายหลังจาก graph traversal ไม่ใช่เก็บเป็น parent pointer เดียว

**Discovery and classification algorithm**

**ข้อเท็จจริงจากเอกสาร:** `/sys/class/block` มีทั้ง disks และ partitions ในระดับเดียวกัน; partitions มี stats แยก; `hidden` ใช้กับองค์ประกอบ underlying ของ multipath; `diskseq` เป็นเลขเพิ่มขึ้น monotonically และ loop อาจ refresh ค่าเมื่อ backing file เปลี่ยน; block stats ถูก export แยกต่อ device; MD state/UUID สามารถอ่านจาก sysfs; DM มี UUID/name; และ USB devpath/attributes ช่วยบอกตำแหน่งใน tree. citeturn34search8turn35search4turn32view4turn32view5turn6view4turn41view1turn25search4turn27search9

**Recommendation:** canonical discovery ควรทำตามลำดับนี้

**ระยะอ่าน snapshot**
อ่านรายการทุก entry ใต้ `/sys/class/block`, canonicalize syspath ไปยัง `/sys/devices/...`, เก็บ `dev_t`, `diskseq`, `hidden`, `stat`, queue limits, และ block size data เอาไว้ใน snapshot เดียว ก่อนทำ classification เพื่อหลีกเลี่ยง race จาก hotplug.

**ระยะสร้าง block-layer relations**
สร้าง node ของ whole-disk/partition ก่อน แล้วค่อยตาม `holders/` และ `slaves/` เพื่อได้ stacked graph. สำหรับ MD ให้อ่าน `/sys/block/mdX/md/*`; สำหรับ DM ให้อ่าน `/sys/block/dm-X/dm/{name,uuid}`; สำหรับ loop/rnbd ให้ดู sysfs-specific leaves เช่น backing file หรือ mapping path. citeturn42search12turn35search8turn25search4turn30search1

**ระยะยกขึ้นสู่ bus/transport**
ไต่ parent chain ใน `/sys/devices` เพื่อหาว่า block node นี้สืบมาจาก SCSI, NVMe, MMC, virtio, USB, PCI, หรือ Thunderbolt อะไรบ้าง เพราะ hierarchy ใน sysfs ถูกออกแบบมาให้สะท้อน device hierarchy จริง. สำหรับ SATA/libata และ USB storage ต้องยอมรับว่าผู้ใช้จะเห็น SCSI bridge objects ก่อน physical ATA semantics. citeturn27search3turn27search8turn29view1turn6view5turn27search0

**ระยะจัดชั้น classification**
ให้ติด label หลายมิติกับแต่ละ node พร้อมกันแทน enum เดียว เช่น

- `placement`: local / remote
- `materialization`: physical / logical / virtual / stacked / partition
- `protocol_view`: scsi / ata-via-sat / nvme / mmc / virtio / unknown
- `exposure`: direct / translated / hidden
- `removability`: fixed / removable / media-absent / unknown

วิธีนี้ช่วยไม่ให้ SATA ผ่าน USB หรือ SATA ผ่าน SAS HBA ถูกจัดผิดเป็น “แค่ SCSI” หรือ “แค่ USB”.

**กฎ practical ที่ควรใช้**
พาร์ทิชันคือ block node ที่ผูกกับ whole-disk node เดิม ไม่ใช่ physical disk ใหม่; dm-crypt, thin, linear, snapshot, cache, verity คือ stacked transforms; md คือ array layer; multipath คือ logical path-group layer; loop/NBD/virtio/zram/ublk เป็น virtual/logical devices; block devices หลัง hardware RAID มักเป็น controller logical volumes; และ SCSI/FC/iSCSI/NVMe-oF มักเป็น remote namespace/LUN ไม่ใช่ physical spindle ที่ host เห็นตรง ๆ. ข้อสรุปส่วนนั้นเป็น inference จาก kernel transport/topology docs และ storage specs. citeturn7search10turn7search11turn7search6turn7search18turn25search0turn26search2turn29view0turn39search6turn31search2turn30search1

## ลำดับชั้นตัวตนถาวรและ decision tree ของ backend

**Persistent identity hierarchy**

**ข้อเท็จจริงจากเอกสาร:** สำหรับ SCSI นั้น VPD pages ใช้ระบุตัวตนได้ โดย page `0x80` คือ Unit Serial Number และ page `0x83` คือ Device Identification; T10 เองใช้กรอบคิดว่า page `0x83` ให้ logical-unit/target-device related identifiers ที่เข้มแข็งกว่า และเอกสาร persistent naming ของ Red Hat แนะนำให้ใช้ WWID เพราะ path-independent; DM มี `dm/uuid`; MD มี array UUID; FC remote ports ส่งออก WWPN/WWNN; NVMe ระบุว่า namespace มี NSID และยังมี globally unique namespace identifiers เช่น EUI-64, NGUID, และ UUID รวมถึง subsystem NQN; USB sysfs ส่งออก `devpath`, vendor/product, manufacturer และบน Thunderbolt มี `unique_id` ใน sysfs. citeturn40search4turn16search0turn40search9turn26search10turn25search4turn41view1turn29view0turn39search5turn39search0turn27search9turn28search1

**Inference:** serial-based identity เพียงตัวเดียวไม่พอ เพราะ

- page `0x80` อาจเป็น serial ของ “device หรือ logical unit” ตามอุปกรณ์และ translation layer
- USB bridges อาจตัดทอนหรือสร้าง serial ใหม่
- hardware RAID มัก expose serial/ID ของ logical volume ไม่ใช่สมาชิกจริง
- NVMe `NSID` เป็น handle ภายใน controller ไม่ใช่ globally stable identity เสมอ
- filesystem UUID เป็น identity ของ filesystem instance ไม่ใช่ hardware identity

การใช้ serial เพียงค่าเดียวเป็น primary key จะพลาดทั้งกรณี collision, replacement, และ shared/remote namespace/LUN. citeturn40search12turn27search9turn26search10turn39search0turn39search5

**ข้อเสนอแนะเชิงสถาปัตยกรรม:** ลำดับชั้น identity ที่ควรใช้คือ

1. **Protocol-level globally scoped ID**
   - SCSI VPD `0x83`
   - multipath WWID
   - NVMe Namespace UUID / NGUID / EUI-64
   - NVMe subsystem NQN
   - DM UUID
   - MD UUID

2. **Transport-level persistent locator**
   - FC WWPN/WWNN + LUN
   - iSCSI IQN/session target + LUN
   - USB physical port path (`devpath`) + bridge serial
   - PCI BDF / Thunderbolt `unique_id`

3. **Vendor/model/serial descriptive identity**
   - ATA model/serial/WWN
   - SCSI page `0x80`
   - USB product/manufacturer/serial
   - MMC card identifiers

4. **Ephemeral generation discriminators**
   - `diskseq`
   - current `dev_t`
   - discovery generation timestamp

**Stable identity strategy**
ให้ `DeviceId` ของโปรแกรมเป็นโครงสร้างสองชั้น

- `logical_identity`: identity ที่คาดว่าจะ survive reboot/path change
- `generation_identity`: `(diskseq, dev_t, seen_at_boot_id)` เพื่อแยก old/new incarnations ของ object เดียวกัน

ถ้า hardware ถูกเปลี่ยนแต่ path เดิมยังอยู่ โปรแกรมจะเห็นว่า locator เดิมแต่ global ID ใหม่; ถ้า device ถูกรีประกอบใหม่โดย loop/NBD/dm transient จะเห็น `diskseq` เปลี่ยนแม้ชื่อเดิม. นี่เป็น recommendation ที่พึ่ง `diskseq` stable ABI โดยตรง. citeturn32view5

**Backend-selection decision tree**

**ข้อเท็จจริงจากเอกสาร:** Linux มี UAPI สำหรับ NVMe passthrough/admin commands (`NVME_IOCTL_ADMIN_CMD`, `NVME_IOCTL_ADMIN64_CMD`), MMC commands (`MMC_IOC_CMD`, `MMC_IOC_MULTI_CMD`), และ SCSI SG_IO; HDIO raw taskfile interfaces มีคำเตือน safety ชัดเจนว่าผิดพลาดแล้วสามารถทำข้อมูลเสียหายหรือทำให้ระบบ hang ได้; SCSI SG_IO ใน kernel มี allowlist สำหรับ unprivileged commands; device-mapper มี uevents และ sysfs UUID/name; MD expose sysfs array state/UUID/metadata version. citeturn21view0turn23search0turn22search2turn18search0turn7search1turn41view1turn41view2turn41view3

**Recommendation:** decision tree ที่ปลอดภัยควรเป็นดังนี้

**ถ้าเป็น native NVMe namespace/controller**
ใช้ NVMe admin ioctl ก่อนเสมอ และกำหนด allowlist เฉพาะ identify / get-log / get-features ที่เป็น read-only. ห้ามใช้ reset, rescan, firmware, format, sanitize, namespace management ใน polling path แม้ UAPI จะมี macro ให้เรียกได้. citeturn21view0

**ถ้าเป็น native mmcblk/eMMC/SD**
ใช้ MMC ioctl เฉพาะ read-oriented commands ที่จำเป็นต่อ identity/health; อย่าพยายามใช้กับ USB card reader ที่ไม่เปิดเผย native MMC stack เพราะ device เหล่านั้นมักมาทาง SCSI bridge. citeturn23search0turn29view1

**ถ้าเป็น SCSI/SAS/FC/iSCSI LUN**
ใช้ SG_IO กับ SCSI inquiry/VPD/log sense ที่อยู่ใน allowlist ของโปรแกรมเอง และอย่าตีความทุก LUN ว่าเป็น physical disk อัตโนมัติ. สำหรับ FC ควร enrich graph ด้วย rport/WWPN/WWNN เพื่อแยก path identity ออกจาก logical-unit identity. citeturn22search2turn29view0turn26search10

**ถ้าเป็น SATA ผ่าน SCSI translation**
ลอง SAT capability probe ก่อน โดยใช้ non-destructive command ชุดเล็กเท่านั้น; ถ้า bridge/HBA ไม่รองรับ ให้ถอยกลับไปยัง SCSI identity/health เท่าที่มี และรายงานว่า physical ATA metrics ถูกซ่อนไว้หลัง translation layer. SAT documents ชี้ว่ามีการ map VPD pages ระหว่าง SCSI/ATA ได้ แต่ไม่ได้แปลว่า implementation ทุก bridge จะรองรับ passthrough ที่ userspace ใช้งานได้จริง. citeturn6view5turn40search13turn22search2

**ถ้าเป็น MD**
health backend หลักคือ MD sysfs สำหรับ array health; member health ต้อง query แยกที่สมาชิกแต่ละตัว. ห้ามรวมค่าของ member devices เข้ากับ md device แล้วบอกว่าเป็น metric ชุดเดียวกัน. citeturn35search8turn41view1turn41view2

**ถ้าเป็น DM/LVM/dm-crypt/multipath**
DM node ไม่มี direct physical health โดย generic kernel ABI; ต้อง route ไปยัง slave/member/path backends และให้ metric scope เป็น `logical-volume`, `stack-transform`, หรือ `multipath-device` ตามจริง. การมี `hidden` บน underlying multipath members ยิ่งตอกย้ำว่าต้องสื่อสารชัดว่ามีชั้นซ่อนอยู่. citeturn25search4turn32view4turn26search2

**ถ้าเป็น hardware RAID logical volume**
generic path ให้ทำได้เพียง logical-volume identity และ basic transport health; physical member disks ถือเป็น `hidden/unsupported` เว้นแต่จะมี vendor/backend เฉพาะพร้อมเอกสารที่ชัดเจน. เอกสาร storage ของ Red Hat อธิบายชัดว่าฮาร์ดแวร์ RAID มักนำเสนอ logical volumes ให้ host เห็นมากกว่าสมาชิกจริง. citeturn26search9turn28search0

**ถ้าเป็น virtio-blk, loop, NBD, zram, ublk, และ virtual block อื่น**
ให้รายงาน health เป็น `unsupported` ที่ scope physical-media แต่ยังเก็บ logical identity และ throughput ได้. แบบนี้ตรงไปตรงมาที่สุดและสอดคล้องกับสิ่งที่ kernel เปิดเผยจริง. citeturn31search2turn30search1turn34search13turn33search10

## สถาปัตยกรรม Rust และนโยบาย runtime

**Rust traits and data structures**

**ข้อเท็จจริงจาก ecosystem ล่าสุด:** ecosystem Rust ปัจจุบันมี crate ที่ตรงกับโจทย์ส่วน userspace/Linux โดยไม่ต้องพึ่ง CLI เป็น primary backend อยู่แล้ว เช่น `rustix` สำหรับ syscall-like APIs, `udev` และ `tokio-udev` สำหรับ enumeration/monitoring, `nix` สำหรับ Unix bindings, `petgraph` สำหรับ graph model, และ `prometheus-client` สำหรับ OpenMetrics/Prometheus export. เอกสารล่าสุดที่ค้นพบแสดง `rustix 1.1.4`, `nix 0.31.3`, `petgraph 0.8.3`, `tokio_udev 0.10.0`, และ `prometheus_client 0.25.0`. citeturn14search5turn14search6turn14search19turn14search8turn15search0

**Recommendation:** trait layout ที่เหมาะกับงานนี้คือ

```rust
pub trait DiscoveryBackend {
    fn enumerate(&self, ctx: &DiscoverCtx) -> Result<Vec<DiscoveredNode>, DiscoverError>;
    fn reconcile(&self, ctx: &DiscoverCtx, delta: HotplugEvent) -> Result<GraphPatch, DiscoverError>;
}

pub trait HealthBackend {
    fn probe_capabilities(&self, dev: &ResolvedDevice) -> CapabilityProbe;
    fn collect(&self, dev: &ResolvedDevice, ctx: &PollCtx) -> Result<Vec<MetricSample>, HealthError>;
}

pub trait ThroughputBackend {
    fn collect_counters(&self, dev: &ResolvedDevice, ctx: &PollCtx) -> Result<Vec<MetricSample>, ThroughputError>;
}

pub trait RaidBackend {
    fn collect_array_state(&self, dev: &ResolvedDevice, ctx: &PollCtx) -> Result<Vec<MetricSample>, RaidError>;
}

pub trait IdentityBackend {
    fn resolve_identity(&self, dev: &ResolvedDevice) -> Result<IdentityBundle, IdentityError>;
}
```

และ data model หลักควรแยกเป็น

```rust
struct DeviceId {
    logical: StableIdentity,
    generation: GenerationIdentity,
}

struct StableIdentity {
    protocol_ids: Vec<ProtocolId>,
    locators: Vec<TransportLocator>,
    dm_uuid: Option<String>,
    md_uuid: Option<String>,
}

struct GenerationIdentity {
    diskseq: Option<u64>,
    dev_t: Option<(u32, u32)>,
    observed_at: std::time::SystemTime,
}
```

**Capability registry และ scoring**
backend router ควรเก็บคะแนนในรูป `BackendScore { safety, directness, scope_fit, confidence, privilege_cost }` แล้วเลือกคะแนนรวมสูงสุดภายใต้ policy ปัจจุบัน เช่น

- native NVMe admin ioctl: directness สูง, safety สูงถ้า allowlisted
- native MMC ioctl: directness สูง, safety ปานกลางถึงสูง
- SCSI SG_IO inquiry/log sense: directness ปานกลาง, safety สูงเมื่อ allowlisted
- SAT passthrough: directness สูงแต่ confidence ขึ้นกับ bridge/HBA
- vendor/controller backend: ใช้เมื่อ generic path บอกว่า logical volume only
- external CLI fallback: คะแนนต่ำสุด และต้องเป็น opt-in เท่านั้น

**feature flags ที่ควรมี**
`nvme`, `scsi`, `sat`, `mmc`, `md`, `dm`, `fc`, `usb`, `prometheus`, `tui`, `vendor-backends`, `external-fallback`.

**Polling scheduler and concurrency policy**

**ข้อเท็จจริงจากเอกสาร:** udev monitor ใช้ netlink; device-mapper มี uevents พร้อม context เพิ่มเติม; MD `array_state` รองรับ select/poll; ค่าบางอย่างอย่าง block `stat` เป็น consistent snapshot ต่อ device; และ SCSI/USB stacks เป็น bridging subsystems ที่ hotplug ได้ตลอดเวลา. citeturn24search3turn7search1turn41view2turn6view4turn29view1

**Recommendation:** scheduler ควรเป็น **two-plane runtime**

- **event plane** รับ udev/netlink events, dm uevents, และ timer ticks
- **poll plane** ทำงานเป็น paced jobs ที่มี controller-aware semaphores

นโยบาย concurrency ที่ปลอดภัย:

- จำกัด management probe พร้อมกัน **ต่อ NVMe controller/subsystem** = 1
- จำกัด SG_IO probes **ต่อ SCSI device / ต่อ USB bridge parent** = 1
- จำกัด probes **ต่อ HBA/transport session** ด้วย token bucket
- global work limiter ตามจำนวน devices ทั้งระบบ เพื่อกัน command storms บน array ใหญ่
- ใช้ jitter และ backoff เมื่อเจอ timeout, `BUSY`, path loss, หรือ reset-like conditions

**hot-plug lifecycle**
ให้ใช้ `diskseq` + sysfs reconciliation เสมอ ไม่เชื่อ uevent แบบเดี่ยว ๆ. ออบเจ็กต์ใน cache ทุกตัวต้องมี `generation_id`; เมื่อ event เข้ามาให้คำนวณ patch จาก sysfs snapshot ใหม่ แล้วค่อย update graph แบบ transactional. วิธีนี้ทำให้ hot-remove, re-add, loop backing change, dm remap, และ path failover ไม่ทำให้ cache stale หรือ identity ปะปน. citeturn32view5turn24search3turn7search1

**Unified metric and error model**

**Recommendation:** metric ทุกตัวควรมี schema แบบนี้

```rust
struct MetricSample {
    key: MetricKey,
    value: MetricValue,
    unit: Unit,
    source: MetricSource,
    scope: MetricScope,
    availability: Availability,
    confidence: Confidence,
    observed_at: SystemTime,
    collection_latency_ms: u32,
    labels: BTreeMap<String, String>,
}
```

โดย `MetricScope` ต้องแยกอย่างน้อยเป็น

- `PhysicalDevice`
- `Controller`
- `NamespaceOrLun`
- `LogicalVolume`
- `Array`
- `Path`
- `Partition`
- `StackLayer`

และ `MetricSource` ต้องบอก origin ชัด เช่น `sysfs-block-stat`, `md-sysfs`, `scsi-vpd83`, `sat-identify`, `nvme-admin-log`, `mmc-extcsd`, `vendor-raid-backend`, `external-cli`.

**Inference สำคัญ:** throughput counters ไม่ควรถูกรวมข้าม layer แบบ additive เด็ดขาด เพราะ kernel export counters แยกต่อ block object และ stacking drivers มี object ของตนเอง; การตีความว่า `dm + md + sda` รวมกันเป็น throughput รวมจึงทำให้เลขซ้ำชั้นและชวนให้ผู้ใช้เข้าใจผิด. ทางออกที่ถูกคือ export ทุกชั้นได้ แต่ทุก sample ต้องมี `scope` และ `source` ชัดเจน พร้อมข้อความ UI ว่า “counters across stacked devices are not additive”. citeturn6view4turn35search4turn25search3turn42search12

**Error taxonomy**
แนะนำให้มีรหัสผิดพลาดหลักดังนี้: `Unsupported`, `HiddenByController`, `CapabilityMissing`, `PermissionDenied`, `ProbeBlockedByPolicy`, `Timeout`, `TransportOffline`, `DeviceGone`, `IdentityMissing`, `IdentityCollision`, `StaleGeneration`, `BackendBug`, `KernelAbiUnavailable`, `VendorBackendUnavailable`.

## Security architecture และกลยุทธ์ทดสอบ

**Security architecture**

**ข้อเท็จจริงจากเอกสาร:** raw HDIO taskfile interfaces มีคำเตือนว่าอาจทำข้อมูลเสียหายหรือทำให้ระบบค้างได้; SG_IO ใน kernel จำกัด subset ของ commands สำหรับ unprivileged users; MMC ioctl และ NVMe ioctl เปิดทางให้ userspace ส่งคำสั่งระดับ protocol ได้จริง; DM และ MD ต่างมี control/state interfaces ของตัวเอง. เอกสารเหล่านี้รวมกันบอกชัดว่าการมี UAPI ไม่ได้แปลว่า runtime ควรเปิด arbitrary commands ให้ผู้ใช้หรือ plugin ยิงได้อิสระ. citeturn18search0turn22search2turn23search0turn21view0turn7search1turn41view2

**Recommendation:** security model ที่ควรใช้คือ **universal read-only command policy**

- backend ทุกตัวมี allowlist ของ protocol commands ที่อ่านอย่างเดียว
- ห้าม arbitrary ioctl/CDB/taskfile/admin command จาก config หรือ plugin
- แยก privileged operations เข้า **command broker process** ที่มี seccomp, RLIMIT, timeout, cancellation และ allowlist ของ device patterns
- worker process ปกติถือสิทธิ์ต่ำและสื่อสารกับ broker ผ่าน typed IPC เท่านั้น
- broker ต้องรู้จัก `DeviceId` และ policy state ไม่ใช่รับเพียง `/dev/X` string

**allowlist ขั้นต่ำที่ปลอดภัย**
- SCSI: standard inquiry, supported VPD pages, VPD 0x80, VPD 0x83, log sense pages ที่อ่านได้
- NVMe: identify, get-log pages ที่เป็น monitor/health, get-feature แบบอ่าน
- MMC: read-oriented `MMC_IOC_CMD` ที่ใช้ดึง identifiers/EXT_CSD บางรายการ
- MD/DM/sysfs: read-only files เท่านั้น

**ข้อเสนอแนะเพิ่มเติม**
- default deny สำหรับ firmware, sanitize, format, namespace management, security send/receive, taskfile writes, start/stop, reset, rescan, and any vendor opcode unless explicitly reviewed
- ใส่ per-controller cool-down window หลัง timeout/path-loss
- ใช้ device allowlist สำหรับระบบ production ที่ต้อง monitor arrays ขนาดใหญ่หรือ SAN สำคัญ
- export audit trail ว่า metric ไหนมาจากคำสั่งใด backend ไหนและสิทธิ์ระดับไหน

**Licensing implications**

**ข้อเท็จจริงจากเอกสาร:** `smartmontools` ใช้ GPL-2.0 และ `nvme-cli` ใช้ GPL-2.0; โครงการ `libnvme` เองระบุชัดว่าตัว spec เป็น authority สำหรับความไม่ตรงกันเชิง protocol ของ library. citeturn43search8turn43search1turn43search2

**Recommendation:** ถ้าต้องมี external-tool fallback ให้ถือเป็น **out-of-process optional adapter** เท่านั้น ไม่ใช่ primary runtime backend และอย่านำโค้ดจากโครงการ GPL เข้ามา copy/translate ลงใน Rust binary แบบหยาบ ๆ โดยไม่ผ่านการ review เรื่องลิขสิทธิ์. การเรียก binary ภายนอกแบบแยก process มักมีผลลิขสิทธิ์ต่างจากการลิงก์ library หรือการคัดลอก implementation รายละเอียดเข้ามาในโปรแกรม แต่ในงาน production ควรให้ฝ่ายกฎหมายตีความก่อนถ้าต้องพึ่ง plugin vendor หรือ fallback จริง. citeturn43search8turn43search1turn43search2

**Fixture and hardware test strategy**

**ข้อเสนอแนะเชิงสถาปัตยกรรม:** strategy ที่เหมาะคือทดสอบสามชั้นพร้อมกัน

**ชั้น topology fixtures**
ใช้ synthetic sysfs trees ที่ encode กรณีต่อไปนี้อย่างน้อย:

- whole disk + partitions
- dm-linear บน SATA
- dm-crypt บน NVMe namespace
- dm-multipath บน FC LUN 4 paths
- mdraid บน SATA 3 ลูก + spare
- loop / NBD / virtio-blk
- USB-to-SATA bridge ที่มี/ไม่มี SAT
- hardware RAID logical volume ที่ไม่มีสมาชิกจริงให้ host เห็น
- replacement case ที่ `diskseq` เปลี่ยนแต่ path เดิม

**ชั้น protocol fixtures**
เก็บ binary fixtures ของ

- SCSI standard inquiry
- VPD 0x80 / 0x83
- SAT identify translation samples
- NVMe identify controller/namespace and health log
- MMC CID/CSD/EXT_CSD reads

**ชั้น behavior tests**
- mock backend scoring
- duplicate IDs / missing IDs
- path failover ใน multipath
- dm remap และ hot-remove
- array degraded/rebuild
- unknown/unsupported backend
- stale generation cache invalidation
- property-based tests สำหรับ graph invariants
- fuzzing parser ของ binary replies และ sysfs text files

**ข้อเท็จจริงจากเอกสาร:** kernel เองสะท้อนว่า DM, MD, loop, FC, USB และ block stats ต่าง export state ผ่าน sysfs/UAPI หลายแบบ และ ublk/virtio ช่วยให้ virtual-device cases ใน VM/emulator เกิดได้จริง แต่ไม่แทน physical hardware ได้ครบ โดยเฉพาะ USB bridges, SAT passthrough, และ hardware RAID firmware behavior. citeturn35search8turn30search1turn29view0turn27search9turn34search13turn31search2

**Recommendation:** hardware qualification matrix ควรมีอย่างน้อย

- AHCI SATA SSD/HDD
- SATA ผ่าน SAS HBA
- SAS HDD/SSD native
- PCIe NVMe
- NVMe-oF over TCP และอย่างน้อยหนึ่ง fabric อื่น
- USB-SATA bridge หลายชิป
- USB-NVMe bridge หลายชิป
- SD reader native vs USB reader
- eMMC บน embedded board
- virtio-blk / virtio-scsi guest
- MD RAID, DM multipath, dm-crypt, LVM thin
- อย่างน้อยหนึ่ง hardware RAID controller ที่มี vendor docs

## โรดแมป ข้อจำกัดที่รองรับไม่ได้ และแหล่งอ้างอิงหลัก

**Phased implementation roadmap**

**Phase แรก**
สร้าง discovery engine จาก stable block sysfs + parent-chain traversal + graph model + metric schema + Prometheus exporter. รองรับ classification ของ disk/partition/dm/md/loop/virtio/NVMe/MMC/SCSI ก่อน แต่ยังไม่ probe health protocol-level ทั้งหมด.

**Phase ถัดมา**
เพิ่ม identity backends ตามลำดับความปลอดภัย: SCSI VPD, NVMe identify, MMC identifiers, MD UUID, DM UUID, WWID grouping.

**Phase ถัดมา**
เพิ่ม health routing สำหรับ NVMe, SCSI/VPD/log sense, SAT allowlisted subset, MD sysfs, DM multipath aggregation.

**Phase ถัดมา**
เพิ่ม hotplug reconciliation, broker privileges, concurrency scheduler, duplicate-ID handling, and replacement handling.

**Phase สุดท้าย**
เพิ่ม vendor/controller backends แบบ feature-gated และ hardware qualification program.

ลำดับนี้สอดคล้องกับหลัก “graph-first, backend-late” และลดความเสี่ยงในการยิง protocol commands ก่อนที่ topology/identity จะนิ่งพอให้ตีความ scope ถูกต้อง.

**Known unsupported cases**

**ข้อเท็จจริงจากเอกสาร + inference:** มีหลายกรณีที่ Linux มองเห็นเพียง logical exposure ไม่ใช่ physical medium จริง เช่น hardware RAID logical volumes, SCSI virtual disks, remote SAN LUNs, USB bridges บางชนิด, และ virtual block devices. เอกสารของ Red Hat อธิบายว่า hardware RAID มักนำเสนอ logical volume ให้ host; FC transport แยก path/remote port ออกจาก logical unit; virtio และ loop/NBD เป็น virtual devices โดยธรรมชาติ. citeturn26search9turn29view0turn31search2turn30search1

**โปรแกรมนี้ควรรายงานเป็น unsupported/hidden/unknown อย่างตรงไปตรงมาในกรณีต่อไปนี้**

- physical disks ที่ซ่อนหลัง hardware RAID โดยไม่มี vendor/backend เฉพาะ
- USB-to-NVMe bridges ที่ไม่เปิดเผย native NVMe path และไม่มี vendor passthrough docs
- USB-to-SATA bridges ที่ไม่รองรับ SAT หรือบิดเบือน serial/identify data
- remote LUNs ที่ให้เฉพาะ logical-unit identity แต่ไม่ให้ media health จริง
- virtual disks ที่ไม่มีทางเชื่อมถึง physical backend จาก guest
- collisions หรือ missing IDs ที่ไม่สามารถคลี่ได้ด้วย transport locator + generation data
- counters ของ stacked devices ที่ผู้ใช้พยายามรวมข้ามชั้นแบบ additive
- ฟีเจอร์ที่อาศัย testing ABI แต่ subsystem นั้นไม่ได้ compile/enable ใน kernel ของเครื่องจริง

**คำแนะนำเชิง UX**
ทุก unsupported metric ควรออกมาเป็น sample ที่มี `availability=unsupported`, `confidence=low or none`, `source=none or topology-only`, และ message อธิบายสั้น ๆ ว่า “logical volume exposed; member-disk health hidden by controller” หรือ “USB bridge does not expose native NVMe health path”.

**Primary-source links**

ด้านล่างคือชุดแหล่งอ้างอิงหลักที่ควรใช้เป็นฐาน implementation จริง

**Linux kernel: sysfs, ABI, block layer, DM, MD**
- sysfs rules และ flat block namespace: citeturn34search8
- sysfs internals และ linkages: citeturn34search5
- stable sysfs block ABI: citeturn35search3
- block `stat` semantics: citeturn6view4
- device-mapper docs และ dm uevent: citeturn7search2turn7search1
- MD admin guide และ sysfs attributes: citeturn25search0turn35search8turn41view1turn41view2turn41view3

**SCSI, ATA/SAT, FC, USB**
- SCSI interfaces guide / transport classes: citeturn29view2
- SCSI mid-low / USB-storage bridging context: citeturn29view1
- libATA guide: citeturn6view5
- SG_IO kernel behavior and command restrictions: citeturn22search2
- HDIO raw warning: citeturn18search0
- FC transport and remote ports: citeturn29view0
- USB host/device model และ USB sysfs attrs: citeturn27search0turn27search9

**NVMe, MMC, virtio**
- NVMe Linux UAPI header: citeturn21view0
- NVMe IDs และ namespace/subsystem concepts: citeturn39search5turn39search0turn39search6
- MMC Linux UAPI header: citeturn23search0
- MMC device partitions doc: citeturn36view0
- Virtio Linux docs และ OASIS spec: citeturn31search1turn31search2turn31search22

**Identity standards**
- SCSI VPD overview / VPD 0x80 / 0x83 references: citeturn40search4turn16search0turn40search9
- DM UUID: citeturn25search4
- MD UUID: citeturn41view1
- multipath WWID / persistent naming: citeturn26search2turn26search10

**Rust ecosystem**
- `rustix`: citeturn14search5
- `nix`: citeturn14search6
- `petgraph`: citeturn14search19
- `udev` / `tokio-udev`: citeturn14search0turn14search8
- `prometheus-client`: citeturn15search0

**Licensing references**
- smartmontools GPL-2.0: citeturn43search8
- nvme-cli GPL-2.0: citeturn43search1
- libnvme project note that public spec is authoritative: citeturn43search2

**สรุปสถาปัตยกรรมที่แนะนำในประโยคเดียว**

ให้สร้าง **graph-based discovery core** จาก stable Linux kernel interfaces ก่อน, ใช้ **identity hierarchy ที่แยก global identity ออกจาก transport locator และ generation**, เลือก backend ด้วย **capability probing + safety scoring**, บังคับ **read-only allowlists ผ่าน privilege-separated broker**, และ export metrics ทุกตัวพร้อม `source`, `scope`, `timestamp`, `availability`, และ `confidence` เพื่อให้ผู้ใช้เห็นชัดว่า metric นี้มาจากชั้นไหนและเชื่อถือได้แค่ไหน.