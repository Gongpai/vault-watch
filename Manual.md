# VaultWatch — Installation Manual

Per-distro guide for building, installing, and running VaultWatch on each supported Linux distribution.

> **Note — build vs install:** `cargo`/`rustup` is installed per-user and is not available to root.
> Always **build as your normal user** (`make build`), then **install as root** (`sudo make install`).
> Never run `sudo make build` — it will fail because root has no Rust toolchain.

---

## Table of Contents

- [Ubuntu / Debian](#ubuntu--debian)
- [Fedora / RHEL / CentOS](#fedora--rhel--centos)
- [Arch Linux / Manjaro](#arch-linux--manjaro)
- [openSUSE](#opensuse)
- [Alpine Linux (musl / static binary)](#alpine-linux)
- [Privilege Setup](#privilege-setup)
- [Configuration](#configuration)
- [Running as a Service](#running-as-a-service)

---

## Ubuntu / Debian

### 1. Install build dependencies

```bash
sudo apt update
sudo apt install -y curl build-essential pkg-config libssl-dev
sudo apt install -y smartmontools sysstat
```

### 2. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 3. Build and install

```bash
git clone https://github.com/YOUR_USERNAME/vault-watch.git
cd vault-watch
make build          # compile as normal user
sudo make install   # copy binary to /usr/local/bin
```

### 4. Run

```bash
sudo vault-watch
```

Or configure NOPASSWD sudo for smartctl (see [Privilege Setup](#privilege-setup)) and run without `sudo`.

### 5. Optional: systemd service

```bash
sudo make install-service
sudo systemctl enable --now vault-watch
```

---

## Fedora / RHEL / CentOS

### 1. Install build dependencies

```bash
sudo dnf install -y curl gcc openssl-devel pkg-config
sudo dnf install -y smartmontools sysstat
```

> **RHEL / CentOS**: enable EPEL first if `smartmontools` is not found:
> ```bash
> sudo dnf install -y epel-release
> ```

### 2. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 3. Build and install

```bash
git clone https://github.com/YOUR_USERNAME/vault-watch.git
cd vault-watch
make build          # compile as normal user
sudo make install   # copy binary to /usr/local/bin
```

### 4. Run

```bash
sudo vault-watch
```

### 5. Optional: systemd service

```bash
sudo make install-service
sudo systemctl enable --now vault-watch
```

---

## Arch Linux / Manjaro

### 1. Install build dependencies

```bash
sudo pacman -S --needed base-devel curl openssl pkg-config
sudo pacman -S smartmontools sysstat
```

> **AUR users**: Rust is also available via `rustup` from the AUR or directly from `base-devel`.

### 2. Install Rust

```bash
# Via rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Or via pacman
sudo pacman -S rust
```

### 3. Build and install

```bash
git clone https://github.com/YOUR_USERNAME/vault-watch.git
cd vault-watch
make build          # compile as normal user
sudo make install   # copy binary to /usr/local/bin
```

### 4. Run

```bash
sudo vault-watch
```

### 5. Optional: systemd service

```bash
sudo make install-service
sudo systemctl enable --now vault-watch
```

---

## openSUSE

### 1. Install build dependencies

```bash
sudo zypper install -y curl gcc libopenssl-devel pkg-config
sudo zypper install -y smartmontools sysstat
```

### 2. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 3. Build and install

```bash
git clone https://github.com/YOUR_USERNAME/vault-watch.git
cd vault-watch
make build          # compile as normal user
sudo make install   # copy binary to /usr/local/bin
```

### 4. Run

```bash
sudo vault-watch
```

### 5. Optional: systemd service

```bash
sudo make install-service
sudo systemctl enable --now vault-watch
```

---

## Alpine Linux

Alpine uses musl libc, so a **static binary** is required. You can either build it on Alpine directly or cross-compile on a glibc system.

### Option A — Build on Alpine itself

#### 1. Install build dependencies

```bash
sudo apk add curl build-base openssl-dev pkgconf musl-dev
sudo apk add smartmontools sysstat
```

#### 2. Install Rust + musl target

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup target add x86_64-unknown-linux-musl
```

#### 3. Build static binary and install

```bash
git clone https://github.com/YOUR_USERNAME/vault-watch.git
cd vault-watch
make build-static
sudo make install-static
```

#### 4. Run

```bash
# As root (recommended on Alpine)
vault-watch

# Or use doas (configure /etc/doas.conf first):
# permit nopass :wheel as root cmd /usr/sbin/smartctl
# Then set in ~/.config/hdd-monitor/config.toml:
#   [system]
#   smartctl_prefix = "doas"
vault-watch
```

#### 5. Optional: OpenRC service

```bash
sudo make install-service
sudo rc-update add vault-watch default
sudo rc-service vault-watch start
```

---

### Option B — Cross-compile on Ubuntu/Debian for Alpine

Use this when you want to build on a fast glibc machine and deploy the binary to Alpine.

#### 1. On the build machine (Ubuntu/Debian):

```bash
# Install musl cross-compiler
sudo apt install -y musl-tools

# Add musl target to Rust
rustup target add x86_64-unknown-linux-musl

# Build
cd vault-watch
make build-static
```

The static binary is at `target/x86_64-unknown-linux-musl/release/vault-watch`.

#### 2. Copy binary to Alpine target:

```bash
scp target/x86_64-unknown-linux-musl/release/vault-watch user@alpine-host:/tmp/
ssh user@alpine-host
sudo install -Dm755 /tmp/vault-watch /usr/local/bin/vault-watch
```

---

## Privilege Setup

`smartctl` requires raw access to disk devices. VaultWatch auto-detects the best method:

| Scenario | Behaviour |
|:---------|:----------|
| Running as root | Calls `smartctl` directly |
| Running as normal user | Prepends `sudo` automatically |
| `smartctl_prefix` set in config | Uses the configured value |

### Recommended: NOPASSWD sudo

Add to `/etc/sudoers` using `visudo`:

```
your_user ALL=(root) NOPASSWD: /usr/sbin/smartctl
```

Then run `vault-watch` as your normal user — no `sudo` needed.

### doas (Alpine / OpenBSD-style)

Install `doas`:

```bash
# Alpine
sudo apk add doas

# Configure /etc/doas.conf
permit nopass your_user as root cmd /usr/sbin/smartctl
```

Set in `~/.config/hdd-monitor/config.toml`:

```toml
[system]
smartctl_prefix = "doas"
```

### setcap (no sudo or doas)

Grant capability directly to the binary:

```bash
sudo setcap cap_sys_rawio+ep /usr/sbin/smartctl
```

Then set empty prefix to run without any escalation:

```toml
[system]
smartctl_prefix = ""
```

---

## Configuration

```bash
mkdir -p ~/.config/hdd-monitor
cp contrib/config.example.toml ~/.config/hdd-monitor/config.toml
$EDITOR ~/.config/hdd-monitor/config.toml
```

Full example with all options: [`contrib/config.example.toml`](contrib/config.example.toml)

### Discord alerts

```toml
[discord]
webhook_url = "https://discord.com/api/webhooks/YOUR_ID/YOUR_TOKEN"
```

Triggers: disk SMART FAIL · RAID degraded · temperature > 60°C · 1-hour cooldown per alert.

---

## Running as a Service

### systemd (Ubuntu, Debian, Fedora, Arch, openSUSE)

```bash
sudo make install-service
sudo systemctl enable --now vault-watch

# View logs
sudo journalctl -fu vault-watch
```

The service runs as root so smartctl can access raw devices without sudo.
Unit file: [`contrib/vault-watch.service`](contrib/vault-watch.service)

### OpenRC (Alpine)

```bash
sudo make install-service
sudo rc-update add vault-watch default
sudo rc-service vault-watch start

# View logs
sudo rc-service vault-watch status
```

Init script: [`contrib/vault-watch.openrc`](contrib/vault-watch.openrc)

---

## Troubleshooting

**`smartctl: Permission denied`**
→ Run as root, or configure NOPASSWD sudo / doas / setcap (see above).

**`smartctl: command not found`**
→ Install `smartmontools`. If installed in a non-standard path, set `smartctl_path` in config.

**`iostat: command not found`**
→ Install `sysstat`. Set `iostat_path` in config if installed elsewhere.

**`sudo make install` fails with "rustup could not choose a version of cargo"**
→ Never run `sudo make install` to build — rustup installs cargo per-user, so root has no Rust toolchain.
Run `make build` first as your normal user, then `sudo make install` separately.

**Build fails: `linker 'cc' not found`**
→ Install `build-essential` (Debian), `base-devel` (Arch), `gcc` (Fedora), `build-base` (Alpine).

**Build fails on Alpine: `error: linker x86_64-linux-musl-gcc not found`**
→ Install `musl-dev`: `sudo apk add musl-dev`.

**Binary runs on build machine but crashes on Alpine**
→ You built a glibc binary. Use `make build-static` to produce a musl static binary.

**Temperature shows `--`**
→ The disk does not report temperature via the SCSI interface. Verify with `sudo smartctl -a -d scsi /dev/sdX`.

**RAID panel shows "No RAID detected"**
→ Check `/proc/mdstat`. VaultWatch reads this file directly — mdadm tools do not need to be installed.
