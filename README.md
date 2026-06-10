# VaultWatch — SAS HDD & RAID Monitor

Terminal UI for real-time monitoring of SAS hard drives and mdadm RAID arrays. Shows temperature, SMART health, I/O throughput, and RAID status in a single dashboard. Sends Discord alerts on critical conditions.

---

## Requirements

| Tool | Purpose | Minimum version |
|:-----|:--------|:----------------|
| `smartctl` | SMART data collection | 7.0 |
| `iostat` | Disk I/O throughput | (any) |
| mdadm (optional) | RAID status | (any) |

### Install dependencies by distro

| Distro | smartmontools | sysstat (iostat) |
|:-------|:-------------|:----------------|
| Ubuntu / Debian | `sudo apt install smartmontools` | `sudo apt install sysstat` |
| Fedora / RHEL | `sudo dnf install smartmontools` | `sudo dnf install sysstat` |
| Arch Linux | `sudo pacman -S smartmontools` | `sudo pacman -S sysstat` |
| openSUSE | `sudo zypper install smartmontools` | `sudo zypper install sysstat` |
| Alpine Linux | `sudo apk add smartmontools` | `sudo apk add sysstat` |

---

## Installation

### From source (glibc)

```bash
cargo build --release
sudo install -Dm755 target/release/vault-watch /usr/local/bin/vault-watch
```

### Static binary for Alpine / musl

Requires `musl-tools` (Debian/Ubuntu) or `musl-cross` (Alpine):

```bash
# Add the musl target once
rustup target add x86_64-unknown-linux-musl

# Build static binary
make build-static
sudo make install-static
```

### Makefile targets

| Target | Action |
|:-------|:-------|
| `make build` | Debug-optimised glibc binary |
| `make build-static` | Static musl binary |
| `make install` | Build + install to `/usr/local/bin` |
| `make install-service` | Install + register systemd / OpenRC service |
| `make uninstall` | Remove binary and service files |

---

## Privilege setup

`smartctl` requires root access to read SMART data from raw disk devices.
VaultWatch auto-detects the appropriate method at startup:

| How you run it | Behaviour |
|:---------------|:----------|
| As root (`sudo vault-watch`) | Calls `smartctl` directly — no prefix |
| As normal user | Prepends `sudo` to `smartctl` calls |
| Config override | Uses `smartctl_prefix` from config |

### Recommended: NOPASSWD for smartctl

Add to `/etc/sudoers` (via `visudo`):

```
your_user ALL=(root) NOPASSWD: /usr/sbin/smartctl
```

### Alternative: `doas` (OpenBSD / Alpine)

```toml
# ~/.config/hdd-monitor/config.toml
[system]
smartctl_prefix = "doas"
```

### Alternative: `setcap` (no sudo needed)

```bash
sudo setcap cap_sys_rawio+ep /usr/sbin/smartctl
```

Then set an empty prefix in config:

```toml
[system]
smartctl_prefix = ""
```

---

## Configuration

Copy the example config and edit:

```bash
mkdir -p ~/.config/hdd-monitor
cp contrib/config.example.toml ~/.config/hdd-monitor/config.toml
$EDITOR ~/.config/hdd-monitor/config.toml
```

See [`contrib/config.example.toml`](contrib/config.example.toml) for all options with descriptions.

### Discord alerts

```toml
[discord]
webhook_url = "https://discord.com/api/webhooks/YOUR_ID/YOUR_TOKEN"
```

Alerts fire for: disk SMART FAIL, RAID array degraded, temperature > 60°C. Each alert has a 1-hour cooldown.

---

## Running as a service

### systemd

```bash
sudo make install-service
sudo systemctl enable --now vault-watch
sudo journalctl -fu vault-watch
```

### OpenRC (Alpine)

```bash
sudo make install-service
sudo rc-update add vault-watch default
sudo rc-service vault-watch start
```

---

## Keyboard shortcuts

| Key | Action |
|:----|:-------|
| `q` / `Ctrl-C` | Quit |
| `r` | Force refresh |
| `g` | Toggle Table / Graph view |
| `Tab` / `Shift+Tab` | Cycle panel focus |
| `↑` / `↓` / `j` / `k` | Scroll focused panel |
| `PgUp` / `PgDn` | Scroll 5 rows |
| `Home` / `End` | Jump to top / bottom |
| Mouse wheel | Scroll panel under cursor |
| Left click | Focus panel |

---

## Troubleshooting

**"Permission denied" on smartctl**
→ See privilege setup above. Run `sudo vault-watch` to test, then configure NOPASSWD.

**Temperature shows `--`**
→ `smartctl` is working but the disk doesn't expose temperature via SCSI. Check `sudo smartctl -a -d scsi /dev/sdX` manually.

**RAID panel shows "No RAID detected"**
→ Check `/proc/mdstat` exists and contains an array. mdadm is read-only; vault-watch doesn't need mdadm installed.

**Build fails on Alpine with "linker not found"**
→ Install musl cross-compiler: `sudo apk add musl-dev`. The `.cargo/config.toml` points to `x86_64-linux-musl-gcc`.

**iostat shows zeros after startup**
→ Normal. `iostat -y 1 1` reports activity *since the last sample*; the first measurement after a fresh boot may be near zero.
