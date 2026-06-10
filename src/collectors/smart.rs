use std::sync::LazyLock;

use futures::future::join_all;
use regex::Regex;
use tokio::process::Command;

use crate::app::DiskInfo;

static SERIAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Serial number:\s+(\S+)").unwrap());
static TEMP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Current Drive Temperature:\s+(\d+) C").unwrap());
static HEALTH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"SMART Health Status:\s+(\w+)").unwrap());
static HOURS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Power_On_Hours:\s+(\d+)").unwrap());
static DEFECTS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Elements in grown defect list:\s+(\d+)").unwrap());
static NME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Non-medium error count:\s+(\d+)").unwrap());
static READ_ERR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"read:\s+\S+\s+\S+\s+\S+\s+\S+\s+(\d+)").unwrap());
static WRITE_ERR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"write:\s+\S+\s+\S+\s+\S+\s+\S+\s+(\d+)").unwrap());

pub async fn collect_all(devices: &[String]) -> Vec<DiskInfo> {
    let futures: Vec<_> = devices.iter().map(|d| collect_one(d.clone())).collect();
    join_all(futures).await
}

async fn collect_one(device: String) -> DiskInfo {
    let dev_path = format!("/dev/{device}");
    let output = Command::new("sudo")
        .args(["smartctl", "-a", "-d", "scsi", &dev_path])
        .output()
        .await;

    let stdout = match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).into_owned(),
        Err(_) => {
            return DiskInfo {
                device,
                serial: None,
                temperature_c: None,
                health_ok: false,
                power_on_hours: None,
                grown_defects: None,
                non_medium_errors: None,
                read_errors: None,
                write_errors: None,
            }
        }
    };

    parse_smart_output(&device, &stdout)
}

fn parse_smart_output(device: &str, output: &str) -> DiskInfo {
    let serial = SERIAL_RE
        .captures(output)
        .map(|c| c[1].to_string());

    let temperature_c = TEMP_RE
        .captures(output)
        .and_then(|c| c[1].parse().ok());

    let health_ok = HEALTH_RE
        .captures(output)
        .map(|c| matches!(c[1].as_ref(), "OK" | "PASSED"))
        .unwrap_or(false);

    let power_on_hours = HOURS_RE
        .captures(output)
        .and_then(|c| c[1].parse().ok());

    let grown_defects = DEFECTS_RE
        .captures(output)
        .and_then(|c| c[1].parse().ok());

    let non_medium_errors = NME_RE
        .captures(output)
        .and_then(|c| c[1].parse().ok());

    let read_errors = READ_ERR_RE
        .captures(output)
        .and_then(|c| c[1].parse().ok());

    let write_errors = WRITE_ERR_RE
        .captures(output)
        .and_then(|c| c[1].parse().ok());

    DiskInfo {
        device: device.to_string(),
        serial,
        temperature_c,
        health_ok,
        power_on_hours,
        grown_defects,
        non_medium_errors,
        read_errors,
        write_errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_OUTPUT: &str = "\
Serial number:        XXXX000000
SMART Health Status: OK
Current Drive Temperature:     53 C
Power_On_Hours:       12345
Elements in grown defect list: 7
Non-medium error count:  16373
read:          0        0         0         0         32         0.000           0
write:         0        0         0         0          0         0.000           0
";

    #[test]
    fn test_parse_full() {
        let info = parse_smart_output("sdc", SAMPLE_OUTPUT);
        assert_eq!(info.device, "sdc");
        assert_eq!(info.serial, Some("XXXX000000".to_string()));
        assert_eq!(info.temperature_c, Some(53));
        assert!(info.health_ok);
        assert_eq!(info.power_on_hours, Some(12345));
        assert_eq!(info.grown_defects, Some(7));
        assert_eq!(info.non_medium_errors, Some(16373));
        assert_eq!(info.read_errors, Some(32));
        assert_eq!(info.write_errors, Some(0));
    }

    #[test]
    fn test_parse_empty_output() {
        let info = parse_smart_output("sdd", "");
        assert_eq!(info.device, "sdd");
        assert!(info.serial.is_none());
        assert!(info.temperature_c.is_none());
        assert!(!info.health_ok);
        assert!(info.power_on_hours.is_none());
    }

    #[test]
    fn test_health_passed() {
        let info = parse_smart_output("sde", "SMART Health Status: PASSED\n");
        assert!(info.health_ok);
    }
}
