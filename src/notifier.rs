use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::app::Alert;

const COOLDOWN: Duration = Duration::from_secs(3600);

#[derive(Debug, Deserialize, Default)]
struct Config {
    discord: Option<DiscordConfig>,
}

#[derive(Debug, Deserialize)]
struct DiscordConfig {
    webhook_url: String,
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("hdd-monitor")
        .join("config.toml")
}

fn load_webhook_url() -> Option<String> {
    let content = std::fs::read_to_string(config_path()).ok()?;
    let config: Config = toml::from_str(&content).ok()?;
    config.discord.map(|d| d.webhook_url)
}

fn alert_key(alert: &Alert) -> String {
    match alert {
        Alert::RaidDegraded => "raid_degraded".to_string(),
        Alert::DiskFail { device } => format!("disk_fail_{device}"),
        Alert::HighTemperature { device, .. } => format!("high_temp_{device}"),
        Alert::GrownDefects { device, .. } => format!("grown_defects_{device}"),
    }
}

// Discord threshold for temperature is 60°C (higher than the 55°C UI warning)
fn should_notify(alert: &Alert) -> bool {
    match alert {
        Alert::RaidDegraded => true,
        Alert::DiskFail { .. } => true,
        Alert::HighTemperature { temp, .. } => *temp > 60,
        Alert::GrownDefects { .. } => false,
    }
}

fn format_discord_message(alert_msg: &str) -> String {
    let hostname = std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "server".to_string());
    format!("🚨 **VaultWatch Alert** — `{hostname}`\n{alert_msg}")
}

async fn send_discord_alert(webhook_url: &str, message: &str) -> Result<(), reqwest::Error> {
    let mut body = HashMap::new();
    body.insert("content", message);
    body.insert("username", "VaultWatch");

    reqwest::Client::new()
        .post(webhook_url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

/// Check alerts against cooldowns, send Discord notifications, return updated cooldowns.
/// Takes slices rather than &mut AppState so the caller can release the mutex before the
/// async HTTP request.
pub async fn process_alerts(
    alerts: &[Alert],
    cooldowns: &HashMap<String, Instant>,
) -> HashMap<String, Instant> {
    let Some(webhook_url) = load_webhook_url() else {
        return cooldowns.clone();
    };

    let now = Instant::now();
    let mut updated = cooldowns.clone();

    for alert in alerts {
        if !should_notify(alert) {
            continue;
        }
        let key = alert_key(alert);
        let in_cooldown = cooldowns
            .get(&key)
            .map(|&t| now.duration_since(t) < COOLDOWN)
            .unwrap_or(false);
        if in_cooldown {
            continue;
        }
        let msg = format_discord_message(&alert.message());
        // Apply cooldown regardless of success to prevent spam on repeated failures
        let _ = send_discord_alert(&webhook_url, &msg).await;
        updated.insert(key, now);
    }

    updated
}
