use ratatui::style::Color;
use serde::Deserialize;
use tokio::process::Command;

use crate::app::{DepError, GraphTheme};

// ── Config structs ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Config {
    pub system: Option<SystemConfig>,
    pub discord: Option<DiscordConfig>,
    pub graph: Option<GraphConfig>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct SystemConfig {
    /// Override smartctl privilege prefix ("sudo", "doas", "" for none).
    /// When omitted, auto-detected via /proc/self/status Uid.
    pub smartctl_prefix: Option<String>,
    /// Custom path to smartctl binary. Defaults to "smartctl" (PATH lookup).
    pub smartctl_path: Option<String>,
    /// Deprecated compatibility key; native throughput does not execute it.
    pub iostat_path: Option<String>,
    /// Explicit disk device list (e.g. ["sda", "sdb"]). Auto-detected from /sys/block if omitted.
    pub devices: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DiscordConfig {
    pub webhook_url: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct GraphConfig {
    pub line_colors: Option<Vec<String>>,
    pub temp_zones: Option<Vec<GraphZoneConfig>>,
    pub io_background: Option<String>,
    pub label_offset: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GraphZoneConfig {
    pub max: f64,
    pub color: String,
}

pub struct LoadedConfig {
    pub config: Config,
    pub error: Option<String>,
}

// ── Config loading ────────────────────────────────────────────────────────────

fn config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    std::path::PathBuf::from(home)
        .join(".config")
        .join("hdd-monitor")
        .join("config.toml")
}

pub fn load_config() -> LoadedConfig {
    let path = config_path();
    let Ok(content) = std::fs::read_to_string(&path) else {
        return LoadedConfig {
            config: Config::default(),
            error: None,
        };
    };
    match toml::from_str(&content) {
        Ok(config) => match validate_config(&config) {
            Ok(()) => LoadedConfig {
                config,
                error: None,
            },
            Err(error) => LoadedConfig {
                config: Config::default(),
                error: Some(format!("Unsafe {}: {error}", path.display())),
            },
        },
        Err(error) => LoadedConfig {
            config: Config::default(),
            error: Some(format!("Invalid {}: {error}", path.display())),
        },
    }
}

fn validate_config(config: &Config) -> Result<(), String> {
    if let Some(system) = &config.system {
        if let Some(prefix) = system.smartctl_prefix.as_deref()
            && !matches!(prefix, "" | "sudo" | "doas")
        {
            return Err("smartctl_prefix must be sudo, doas, or empty".to_string());
        }
        validate_executable(system.smartctl_path.as_deref(), "smartctl")?;
        validate_executable(system.iostat_path.as_deref(), "iostat")?;
        if let Some(devices) = &system.devices {
            for device in devices {
                if device.is_empty()
                    || !device
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
                {
                    return Err(format!(
                        "invalid device name {device:?}; paths are not allowed"
                    ));
                }
            }
        }
    }
    if let Some(url) = config
        .discord
        .as_ref()
        .and_then(|discord| discord.webhook_url.as_deref())
        .filter(|url| !url.trim().is_empty())
        && !url.starts_with("https://discord.com/api/webhooks/")
    {
        return Err("webhook_url must be an HTTPS Discord webhook endpoint".to_string());
    }
    if let Some(graph) = &config.graph {
        if let Some(colors) = &graph.line_colors
            && (colors.is_empty()
                || colors.len() > 16
                || colors.iter().any(|value| parse_hex(value).is_none()))
        {
            return Err("graph.line_colors must contain 1..=16 #RRGGBB colors".into());
        }
        if graph
            .io_background
            .as_deref()
            .is_some_and(|value| parse_hex(value).is_none())
        {
            return Err("graph.io_background must be #RRGGBB".into());
        }
        if graph
            .label_offset
            .is_some_and(|value| !value.is_finite() || !(-2.0..=2.0).contains(&value))
        {
            return Err("graph.label_offset must be finite and between -2.0 and 2.0".into());
        }
        if let Some(zones) = &graph.temp_zones {
            if zones.is_empty() || zones.len() > 16 {
                return Err("graph.temp_zones must contain 1..=16 zones".into());
            }
            let mut previous = 0.0;
            for zone in zones {
                if !zone.max.is_finite()
                    || zone.max <= previous
                    || zone.max > 200.0
                    || parse_hex(&zone.color).is_none()
                {
                    return Err(
                        "graph.temp_zones require increasing max values <= 200 and #RRGGBB colors"
                            .into(),
                    );
                }
                previous = zone.max;
            }
        }
    }
    Ok(())
}

fn parse_hex(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(Color::Rgb(
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ))
}

pub fn resolve_graph_theme(config: &Config) -> GraphTheme {
    let mut theme = GraphTheme::default();
    let Some(graph) = &config.graph else {
        return theme;
    };
    if let Some(colors) = &graph.line_colors {
        theme.line_colors = colors.iter().filter_map(|value| parse_hex(value)).collect();
    }
    if let Some(background) = graph.io_background.as_deref().and_then(parse_hex) {
        theme.io_background = background;
    }
    if let Some(offset) = graph.label_offset {
        theme.label_offset = offset;
    }
    if let Some(zones) = &graph.temp_zones {
        let mut low = 0.0;
        theme.temp_zones = zones
            .iter()
            .filter_map(|zone| {
                let color = parse_hex(&zone.color)?;
                let result = (low, zone.max, color);
                low = zone.max;
                Some(result)
            })
            .collect();
    }
    theme
}

fn validate_executable(value: Option<&str>, expected_name: &str) -> Result<(), String> {
    let Some(value) = value else { return Ok(()) };
    let path = std::path::Path::new(value);
    let basename = path.file_name().and_then(|name| name.to_str());
    if basename != Some(expected_name) {
        return Err(format!(
            "configured executable must be named {expected_name}"
        ));
    }
    if path.components().count() > 1 {
        let trusted_parent = path.parent().is_some_and(|parent| {
            matches!(
                parent.to_str(),
                Some(
                    "/bin"
                        | "/sbin"
                        | "/usr/bin"
                        | "/usr/sbin"
                        | "/usr/local/bin"
                        | "/usr/local/sbin"
                )
            )
        });
        if !path.is_absolute() || !trusted_parent {
            return Err(format!(
                "{expected_name} path must be in a trusted system executable directory"
            ));
        }
    }
    Ok(())
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
        Some("") => (path, vec![]),
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
    if let Some(devs) = config.system.as_ref().and_then(|s| s.devices.as_ref())
        && !devs.is_empty()
    {
        return devs.clone();
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

    missing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_discord_table_does_not_discard_system_config() {
        let config: Config = toml::from_str("[system]\ndevices = [\"sda\"]\n[discord]\n").unwrap();
        assert_eq!(config.system.unwrap().devices.unwrap(), vec!["sda"]);
        assert!(config.discord.unwrap().webhook_url.is_none());
    }

    #[test]
    fn rejects_command_and_device_injection() {
        let config: Config = toml::from_str(
            "[system]\nsmartctl_prefix = \"sh\"\ndevices = [\"../../etc/shadow\"]\n",
        )
        .unwrap();
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn rejects_executable_from_untrusted_directory() {
        let config: Config =
            toml::from_str("[system]\nsmartctl_path = \"/tmp/smartctl\"\n").unwrap();
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn rejects_non_discord_webhook() {
        let config: Config =
            toml::from_str("[discord]\nwebhook_url = \"https://attacker.invalid/collect\"\n")
                .unwrap();
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn validates_and_resolves_graph_theme() {
        let config: Config = toml::from_str(
            "[graph]\nline_colors=['#010203']\nio_background='#0A0B0C'\nlabel_offset=-0.75\ntemp_zones=[{max=40,color='#111111'},{max=90,color='#222222'}]\n",
        ).unwrap();
        validate_config(&config).unwrap();
        let theme = resolve_graph_theme(&config);
        assert_eq!(theme.line_colors, vec![Color::Rgb(1, 2, 3)]);
        assert_eq!(theme.io_background, Color::Rgb(10, 11, 12));
        assert_eq!(theme.temp_zones.len(), 2);
        assert_eq!(theme.label_offset, -0.75);
    }

    #[test]
    fn rejects_invalid_graph_theme_without_silent_fallback() {
        let config: Config = toml::from_str("[graph]\nline_colors=['red']\n").unwrap();
        assert!(validate_config(&config).is_err());
        let config: Config = toml::from_str(
            "[graph]\ntemp_zones=[{max=50,color='#000000'},{max=40,color='#FFFFFF'}]\n",
        )
        .unwrap();
        assert!(validate_config(&config).is_err());
    }
}
