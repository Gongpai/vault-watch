use std::sync::LazyLock;

use futures::future::join_all;
use regex::Regex;
use tokio::process::Command;

use crate::app::{DiskInfo, HealthStatus, MetricAvailability};

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

pub async fn collect_all(devices: &[String], prog: &str, base_args: &[String]) -> Vec<DiskInfo> {
    let futures: Vec<_> = devices
        .iter()
        .map(|d| collect_one(d.clone(), prog.to_string(), base_args.to_vec()))
        .collect();
    join_all(futures).await
}

async fn collect_one(device: String, prog: String, base_args: Vec<String>) -> DiskInfo {
    let dev_path = format!("/dev/{device}");
    let mut args: Vec<String> = base_args;
    args.extend([
        "-a".to_string(),
        "-d".to_string(),
        "scsi".to_string(),
        dev_path,
    ]);

    let output = Command::new(&prog).args(&args).output().await;

    let output = match output {
        Ok(output) => output,
        Err(error) => {
            let mut info = DiskInfo::unavailable(device);
            info.health_availability = match error.kind() {
                std::io::ErrorKind::NotFound => MetricAvailability::Unsupported,
                std::io::ErrorKind::PermissionDenied => MetricAvailability::PermissionDenied,
                _ => MetricAvailability::TemporarilyUnavailable,
            };
            return info;
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut info = parse_smart_output(&device, &stdout);
    if info.health == HealthStatus::Unavailable {
        info.health_availability = classify_unavailable_output(&stdout, &stderr);
    }
    info
}

fn classify_unavailable_output(stdout: &str, stderr: &str) -> MetricAvailability {
    let diagnostic = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    if diagnostic.contains("permission denied") || diagnostic.contains("operation not permitted") {
        MetricAvailability::PermissionDenied
    } else if diagnostic.contains("standby") || diagnostic.contains("sleep mode") {
        MetricAvailability::Asleep
    } else if diagnostic.contains("no such device") || diagnostic.contains("device gone") {
        MetricAvailability::DeviceGone
    } else if diagnostic.contains("unsupported")
        || diagnostic.contains("unknown device type")
        || diagnostic.contains("not supported")
    {
        MetricAvailability::Unsupported
    } else if HEALTH_RE.is_match(stdout) {
        MetricAvailability::Malformed
    } else {
        MetricAvailability::TemporarilyUnavailable
    }
}

fn parse_smart_output(device: &str, output: &str) -> DiskInfo {
    let serial = SERIAL_RE.captures(output).map(|c| c[1].to_string());

    // Some unsupported SCSI translations report 0 C. Treat that sentinel as
    // unavailable instead of presenting a physically meaningful temperature.
    let temperature_c = TEMP_RE
        .captures(output)
        .and_then(|c| c[1].parse().ok())
        .filter(|temperature| *temperature > 0);

    let health = HEALTH_RE
        .captures(output)
        .map(|capture| match capture[1].as_ref() {
            "OK" | "PASSED" => HealthStatus::Healthy,
            "FAIL" | "FAILED" | "BAD" => HealthStatus::Failed,
            _ => HealthStatus::Unavailable,
        })
        .unwrap_or(HealthStatus::Unavailable);
    let health_availability = if health == HealthStatus::Unavailable {
        classify_unavailable_output(output, "")
    } else {
        MetricAvailability::Available
    };

    let power_on_hours = HOURS_RE.captures(output).and_then(|c| c[1].parse().ok());

    let grown_defects = DEFECTS_RE.captures(output).and_then(|c| c[1].parse().ok());

    let non_medium_errors = NME_RE.captures(output).and_then(|c| c[1].parse().ok());

    let read_errors = READ_ERR_RE.captures(output).and_then(|c| c[1].parse().ok());

    let write_errors = WRITE_ERR_RE
        .captures(output)
        .and_then(|c| c[1].parse().ok());

    DiskInfo {
        device: device.to_string(),
        serial,
        temperature_c,
        health,
        health_availability,
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
        assert_eq!(info.health, HealthStatus::Healthy);
        assert_eq!(info.health_availability, MetricAvailability::Available);
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
        assert_eq!(info.health, HealthStatus::Unavailable);
        assert_eq!(
            info.health_availability,
            MetricAvailability::TemporarilyUnavailable
        );
        assert!(info.power_on_hours.is_none());
    }

    #[test]
    fn test_health_passed() {
        let info = parse_smart_output("sde", "SMART Health Status: PASSED\n");
        assert_eq!(info.health, HealthStatus::Healthy);
    }

    #[test]
    fn missing_health_and_zero_temperature_are_unavailable_not_failed() {
        let info = parse_smart_output("sda", "Current Drive Temperature:     0 C\n");

        assert_eq!(info.health, HealthStatus::Unavailable);
        assert_eq!(info.temperature_c, None);
    }

    #[test]
    fn explicit_failed_health_remains_failed() {
        let info = parse_smart_output("sda", "SMART Health Status: FAILED\n");

        assert_eq!(info.health, HealthStatus::Failed);
    }

    #[test]
    fn ambiguous_health_is_unavailable_not_failed() {
        let info = parse_smart_output("sda", "SMART Health Status: UNKNOWN\n");

        assert_eq!(info.health, HealthStatus::Unavailable);
        assert_eq!(info.health_availability, MetricAvailability::Malformed);
    }

    #[test]
    fn unavailable_diagnostics_remain_typed() {
        assert_eq!(
            classify_unavailable_output("", "Permission denied"),
            MetricAvailability::PermissionDenied
        );
        assert_eq!(
            classify_unavailable_output("Device is in STANDBY", ""),
            MetricAvailability::Asleep
        );
        assert_eq!(
            classify_unavailable_output("unsupported device", ""),
            MetricAvailability::Unsupported
        );
        assert_eq!(
            classify_unavailable_output("", "No such device"),
            MetricAvailability::DeviceGone
        );
    }
}
