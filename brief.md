# Project Context: HDD Monitor for Ubuntu Server (Rust)

## Background

The server is an Ubuntu 24.04 system running on an HPE ProLiant DL560 Gen8 with:

* 4x Intel Xeon E5-4657L v2
* 256 GB RAM
* SAS HDDs connected through an LSI SAS2308 IT Mode HBA
* mdadm RAID10 array
* LVM on top of mdadm
* ext4 filesystem

Current storage layout:

```text
sdc (HP MB6000JVYZD 6TB SAS)
sdd (HP MB6000JVYZD 6TB SAS)
sde (HP MB6000JVYZD 6TB SAS)

sdc + sdd + sde
        ↓
      md0 (RAID10)
        ↓
vg_raid/lv_data
        ↓
/media/storage-one
```

The system recently experienced PSU instability.

After replacing the PSU with a Lite-On HS-S501-05 500W 80+ Gold PSU, all SAS disks became visible again and RAID rebuild was started.

Current rebuild speed:

```text
~178 MB/s
```

Observed via:

```bash
cat /proc/mdstat
iostat -xz
```

---

## Monitoring Problems Discovered

### Glances

Installed and tested:

```bash
glances
```

Pros:

* CPU usage
* RAM usage
* Network throughput
* Mounted filesystem usage

Cons:

* Does not show SAS HDD temperatures
* Does not show individual SAS disks properly
* Only shows mounted filesystems
* RAID rebuild visibility is limited

---

### Bottom (btm)

Installed:

```bash
sudo snap install bottom
```

Later switched to native package.

Pros:

* Better terminal UI
* Shows filesystem information
* Shows CPU and memory nicely

Cons:

* Temperature panel only displays sensors from lm-sensors
* HDD SAS temperatures are not shown
* Disk panel only displays mounted filesystems
* Physical disks behind mdadm/LVM are not displayed properly

Example:

```text
/dev/dm-0
/dev/sda1
/dev/sda2
```

but not:

```text
sdc
sdd
sde
```

---

## Temperature Investigation

Initially:

```bash
sensors
```

did not show HDD temperatures.

After:

```bash
sudo modprobe drivetemp
```

one drive appeared:

```text
drivetemp-scsi-0-0
```

However:

* Only one drive became visible
* Not all SAS drives exposed temperatures
* Bottom still could not display all HDD temperatures

Reliable source remained:

```bash
smartctl -a -d scsi /dev/sdX
```

Example:

```text
Current Drive Temperature: 53 C
```

---

## Current Reliable Data Sources

### RAID Status

```bash
cat /proc/mdstat
```

Needed:

* rebuild %
* rebuild speed
* ETA
* degraded status

---

### HDD SMART Information

```bash
smartctl -a -d scsi /dev/sdc
smartctl -a -d scsi /dev/sdd
smartctl -a -d scsi /dev/sde
```

Required fields:

```text
Serial number
Current Drive Temperature
SMART Health Status
Power-on hours
Grown defect list
Non-medium error count
Read errors
Write errors
```

---

### Disk Throughput

Current method:

```bash
iostat -d -k sdc sdd sde
```

Need:

```text
Read MB/s
Write MB/s
```

per drive.

---

## Goal

Create a dedicated HDD monitoring application focused on storage devices only.

This application should not attempt to replace Glances.

Instead it should focus on:

* SAS HDD monitoring
* RAID monitoring
* SMART monitoring
* Temperature monitoring
* Disk throughput monitoring

---

## Technology Choice

Language:

```text
Rust
```

Preferred stack:

```text
ratatui
crossterm
tokio
serde
regex
```

Optional:

```text
sysinfo
```

but SMART data should be obtained from:

```bash
smartctl
```

because it provides more accurate SAS information.

---

## UI Requirements

Terminal dashboard similar to:

* bottom
* btop
* htop

Single screen layout.

Example:

```text
┌──────────────────────────────────────────────┐
│ HDD Monitor                                  │
├──────────────────────────────────────────────┤
│ RAID                                          │
│ md0                                           │
│ Rebuild: 9.3%                                │
│ Speed: 178 MB/s                              │
│ ETA: 8h 16m                                  │
└──────────────────────────────────────────────┘

┌─────┬────────────┬──────┬───────┬────────────┐
│Disk │ Temp       │SMART │ Read  │ Write      │
├─────┼────────────┼──────┼───────┼────────────┤
│sdc  │ 50°C       │ OK   │ 0     │ 0          │
│sdd  │ 53°C       │ OK   │ 178M  │ 0          │
│sde  │ 48°C       │ OK   │ 0     │ 178M       │
└─────┴────────────┴──────┴───────┴────────────┘

┌──────────────────────────────────────────────┐
│ SMART Details                                │
├──────────────────────────────────────────────┤
│ sdc: Non-medium errors: 16373               │
│ sdd: Non-medium errors: 32025               │
│ sde: Grown defects: 7                       │
└──────────────────────────────────────────────┘
```

---

## Future Features

Potential future additions:

* Temperature color coding
* Audible alerts
* SMART threshold warnings
* Discord webhook notifications
* Cockpit integration
* Export JSON API
* Prometheus exporter
* Web dashboard

---

## Important Design Constraint

The application must support:

```text
SAS HDD
LSI HBA IT Mode
mdadm RAID
LVM
Ubuntu Server
```

Many existing Linux monitoring tools fail to display all SAS drive information correctly.

The purpose of this project is to create a storage-focused dashboard that uses SMART as the source of truth rather than generic filesystem monitoring.
