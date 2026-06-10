use serde::Deserialize;
use tokio::process::Command;

use crate::app::DepError;

// ── Config structs ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Config {
    pub system: Option<SystemConfig>,
    pub discord: Option<DiscordConfig>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct SystemConfig {
    /// Override smartctl privilege prefix ("sudo", "doas", "" for none).
    /// When omitted, auto-detected via /proc/self/status Uid.
    pub smartctl_prefix: Option<String>,
    /// Custom path to smartctl binary. Defaults to "smartctl" (PATH lookup).
    pub smartctl_path: Option<String>,
    /// Custom path to iostat binary. Defaults to "iostat" (PATH lookup).
    pub iostat_path: Option<String>,
    /// Explicit disk device list (e.g. ["sda", "sdb"]). Auto-detected from /sys/block if omitted.
    pub devices: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DiscordConfig {
    pub webhook_url: String,
}

// ── Config loading ────────────────────────────────────────────────────────────

fn config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    std::path::PathBuf::from(home)
        .join(".config")
        .join("hdd-monitor")
        .join("config.toml")
}

pub fn load_config() -> Config {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

// ── Runtime detection ─────────────────────────────────────────────────────────

fn is_running_as_root() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Uid:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|uid| uid.parse::<u32>().ok())
        })
        .map(|uid| uid == 0)
        .unwrap_or(false)
}

// ── Command helpers ───────────────────────────────────────────────────────────

/// Returns (program, base_args) for invoking smartctl.
/// base_args are prepended before device-specific arguments.
pub fn smartctl_base_cmd(config: &Config) -> (String, Vec<String>) {
    let path = config
        .system
        .as_ref()
        .and_then(|s| s.smartctl_path.as_deref())
        .unwrap_or("smartctl")
        .to_string();

    let explicit_prefix = config
        .system
        .as_ref()
        .and_then(|s| s.smartctl_prefix.as_deref());

    match explicit_prefix {
        // Empty string = explicit "no prefix" (setcap / root already)
        Some(p) if p.is_empty() => (path, vec![]),
        Some(p) => (p.to_string(), vec![path]),
        None => {
            if is_running_as_root() {
                (path, vec![])
            } else {
                ("sudo".to_string(), vec![path])
            }
        }
    }
}

pub fn iostat_cmd(config: &Config) -> String {
    config
        .system
        .as_ref()
        .and_then(|s| s.iostat_path.as_deref())
        .unwrap_or("iostat")
        .to_string()
}

// ── Device discovery ─────────────────────────────────────────────────────────

/// Discover SAS/SATA block devices from /sys/block (sd* entries only).
/// Partitions (sda1) do not appear at /sys/block level, so no extra filtering needed.
fn detect_disk_devices() -> Vec<String> {
    let mut devices: Vec<String> = std::fs::read_dir("/sys/block")
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .filter(|name| name.starts_with("sd"))
                .collect()
        })
        .unwrap_or_default();
    devices.sort();
    devices
}

/// Return the device list to monitor: config override takes precedence over auto-detect.
pub fn resolve_devices(config: &Config) -> Vec<String> {
    if let Some(devs) = config.system.as_ref().and_then(|s| s.devices.as_ref()) {
        if !devs.is_empty() {
            return devs.clone();
        }
    }
    detect_disk_devices()
}

// ── Distro detection ──────────────────────────────────────────────────────────

enum Distro {
    Ubuntu,
    Debian,
    Fedora,
    Rhel,
    Arch,
    Opensuse,
    Alpine,
    Unknown,
}

impl Distro {
    fn smartmontools_hint(&self) -> &'static str {
        match self {
            Distro::Ubuntu | Distro::Debian => "sudo apt install smartmontools",
            Distro::Fedora | Distro::Rhel => "sudo dnf install smartmontools",
            Distro::Arch => "sudo pacman -S smartmontools",
            Distro::Opensuse => "sudo zypper install smartmontools",
            Distro::Alpine => "sudo apk add smartmontools",
            Distro::Unknown => "install smartmontools (see distro docs)",
        }
    }

    fn sysstat_hint(&self) -> &'static str {
        match self {
            Distro::Ubuntu | Distro::Debian => "sudo apt install sysstat",
            Distro::Fedora | Distro::Rhel => "sudo dnf install sysstat",
            Distro::Arch => "sudo pacman -S sysstat",
            Distro::Opensuse => "sudo zypper install sysstat",
            Distro::Alpine => "sudo apk add sysstat",
            Distro::Unknown => "install sysstat (see distro docs)",
        }
    }

}

fn detect_distro() -> Distro {
    let content = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    for line in content.lines() {
        if let Some(id) = line.strip_prefix("ID=") {
            let id = id.trim_matches('"').to_lowercase();
            return match id.as_str() {
                "ubuntu" => Distro::Ubuntu,
                "debian" => Distro::Debian,
                "fedora" => Distro::Fedora,
                "rhel" | "centos" | "rocky" | "almalinux" | "ol" => Distro::Rhel,
                "arch" | "manjaro" | "endeavouros" | "cachyos" => Distro::Arch,
                "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" | "suse" => Distro::Opensuse,
                "alpine" => Distro::Alpine,
                _ => Distro::Unknown,
            };
        }
    }
    Distro::Unknown
}

// ── Dependency check ──────────────────────────────────────────────────────────

pub async fn check_dependencies(config: &Config) -> Vec<DepError> {
    let mut missing = Vec::new();
    let distro = detect_distro();

    // Check smartctl
    let (prog, mut args) = smartctl_base_cmd(config);
    args.push("--version".to_string());
    let smartctl_ok = Command::new(&prog)
        .args(&args)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !smartctl_ok {
        missing.push(DepError {
            tool: "smartctl".to_string(),
            install_hint: distro.smartmontools_hint().to_string(),
        });
    }

    // Check iostat
    let iostat = iostat_cmd(config);
    let iostat_ok = Command::new(&iostat)
        .arg("-V")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !iostat_ok {
        missing.push(DepError {
            tool: "iostat".to_string(),
            install_hint: distro.sysstat_hint().to_string(),
        });
    }

    missing
}
