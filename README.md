# VaultWatch — SAS HDD & RAID Monitor

Terminal UI for real-time monitoring of SAS hard drives and mdadm RAID arrays. Shows temperature, SMART health, I/O throughput, and RAID status in a single dashboard. Sends Discord alerts on critical conditions.

---

## Quick Start (Ubuntu / Debian)

```bash
# Install dependencies
sudo apt install smartmontools sysstat

# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build (as normal user — do NOT use sudo here)
git clone https://github.com/YOUR_USERNAME/vault-watch.git
cd vault-watch
make build

# Install binary (requires root)
sudo make install

# Run
sudo vault-watch
```

For all other distros (Fedora, Arch, openSUSE, Alpine) see **[Manual.md](MANUAL.md)**.

---

## Requirements

| Tool | Purpose |
|:-----|:--------|
| `smartctl` (smartmontools) | SMART data — temperature, health, defects |
| Linux `/proc/diskstats` | Disk I/O throughput (native, no package required) |
| Linux MD sysfs | Software RAID status (native, no mdadm runtime required) |

---

## Keyboard Shortcuts

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

## Configuration

```bash
mkdir -p ~/.config/hdd-monitor
cp contrib/config.example.toml ~/.config/hdd-monitor/config.toml
$EDITOR ~/.config/hdd-monitor/config.toml
```

### Discord alerts

```toml
[discord]
webhook_url = "https://discord.com/api/webhooks/YOUR_ID/YOUR_TOKEN"
```

Alerts fire for: disk SMART FAIL · RAID degraded · temperature > 60°C. 1-hour cooldown per alert.

---

## Documentation

| File | Contents |
|:-----|:---------|
| [MANUAL.md](MANUAL.md) | Full install guide — all distros, privilege setup, service config |
| [contrib/config.example.toml](contrib/config.example.toml) | Annotated config file with all options |
| [docs/](docs/) | Architecture, system design, sprint docs |

---

## License

MIT — [Kongphai Wutthichaiya](LICENSE)
