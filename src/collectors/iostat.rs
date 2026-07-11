use crate::app::{IoMetricScope, IoMetricSource, IoStats};

fn parse_iostat_output(output: &str, devices: &[String]) -> Vec<IoStats> {
    // Split output into blocks separated by blank lines
    let mut all_blocks: Vec<Vec<&str>> = Vec::new();
    let mut current: Vec<&str> = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                all_blocks.push(std::mem::take(&mut current));
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        all_blocks.push(current);
    }

    // Use the last block that contains a "Device" header line
    let block = match all_blocks
        .iter()
        .rfind(|b| b.iter().any(|l| l.trim_start().starts_with("Device")))
    {
        Some(b) => b,
        None => return Vec::new(),
    };

    let mut results = Vec::new();
    let mut in_data = false;

    for line in block {
        let trimmed = line.trim();
        if trimmed.starts_with("Device") {
            in_data = true;
            continue;
        }
        if !in_data {
            continue;
        }
        let fields: Vec<&str> = trimmed.split_whitespace().collect();
        // field[0]=device, field[1]=tps, field[2]=kB_read/s, field[3]=kB_wrtn/s
        if fields.len() < 4 {
            continue;
        }
        if !devices.iter().any(|d| d == fields[0]) {
            continue;
        }
        let read_kb_s: f64 = fields[2].parse().unwrap_or(0.0);
        let write_kb_s: f64 = fields[3].parse().unwrap_or(0.0);
        results.push(IoStats {
            device: fields[0].to_string(),
            read_mb_s: read_kb_s / 1024.0,
            write_mb_s: write_kb_s / 1024.0,
            read_iops: 0.0,
            write_iops: 0.0,
            utilization_percent: 0.0,
            average_read_latency_ms: None,
            average_write_latency_ms: None,
            average_queue_depth: 0.0,
            ios_in_progress: 0,
            source: IoMetricSource::LegacyIostatOracle,
            scope: IoMetricScope::DirectWholeDevice,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn devices() -> Vec<String> {
        vec!["sdc".to_string(), "sdd".to_string(), "sde".to_string()]
    }

    const SINGLE_BLOCK: &str = "\
Device             tps    kB_read/s    kB_wrtn/s    kB_dscd/s    kB_read    kB_wrtn    kB_dscd
sdc               0.00         0.00         0.00         0.00          0          0          0
sdd             178.00    182304.00         0.00         0.00    1090263          0          0
sde               0.00         0.00    182304.00         0.00          0    1090263          0
";

    const TWO_BLOCKS: &str = "\
Device             tps    kB_read/s    kB_wrtn/s    kB_dscd/s    kB_read    kB_wrtn    kB_dscd
sdc               0.00         0.00         0.00         0.00          0          0          0

Device             tps    kB_read/s    kB_wrtn/s    kB_dscd/s    kB_read    kB_wrtn    kB_dscd
sdc             178.00    182304.00         0.00         0.00    1090263          0          0
";

    #[test]
    fn test_single_block() {
        let results = parse_iostat_output(SINGLE_BLOCK, &devices());
        assert_eq!(results.len(), 3);

        let sdd = results.iter().find(|r| r.device == "sdd").unwrap();
        assert!((sdd.read_mb_s - 182304.0 / 1024.0).abs() < 0.01);
        assert!(sdd.write_mb_s.abs() < 0.01);

        let sde = results.iter().find(|r| r.device == "sde").unwrap();
        assert!(sde.read_mb_s.abs() < 0.01);
        assert!((sde.write_mb_s - 182304.0 / 1024.0).abs() < 0.01);
    }

    #[test]
    fn test_uses_last_block() {
        let single = vec!["sdc".to_string()];
        let results = parse_iostat_output(TWO_BLOCKS, &single);
        assert_eq!(results.len(), 1);
        // Last block has sdc read = 182304 kB/s
        assert!((results[0].read_mb_s - 182304.0 / 1024.0).abs() < 0.01);
    }

    #[test]
    fn test_empty_output() {
        assert!(parse_iostat_output("", &devices()).is_empty());
    }
}
