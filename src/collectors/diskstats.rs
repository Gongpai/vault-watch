use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;
use std::time::{Duration, Instant};

const SECTOR_BYTES: f64 = 512.0;
const MIB_BYTES: f64 = 1_048_576.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskStat {
    pub major: u32,
    pub minor: u32,
    pub name: String,
    pub reads_completed: u64,
    pub reads_merged: u64,
    pub sectors_read: u64,
    pub time_reading_ms: u64,
    pub writes_completed: u64,
    pub writes_merged: u64,
    pub sectors_written: u64,
    pub time_writing_ms: u64,
    pub ios_in_progress: u64,
    pub io_ticks_ms: u64,
    pub weighted_time_in_queue_ms: u64,
    pub discards_completed: Option<u64>,
    pub discards_merged: Option<u64>,
    pub sectors_discarded: Option<u64>,
    pub time_discarding_ms: Option<u64>,
    pub flush_requests: Option<u64>,
    pub time_flushing_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiskMetrics {
    pub read_mib_per_sec: f64,
    pub write_mib_per_sec: f64,
    pub read_iops: f64,
    pub write_iops: f64,
    pub utilization_percent: f64,
    pub average_read_latency_ms: Option<f64>,
    pub average_write_latency_ms: Option<f64>,
    pub average_queue_depth: f64,
    pub ios_in_progress: u64,
}

#[derive(Debug)]
pub enum DiskstatsError {
    Read(std::io::Error),
    Parse { line: usize, reason: String },
}

impl fmt::Display for DiskstatsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "cannot read diskstats: {error}"),
            Self::Parse { line, reason } => {
                write!(formatter, "invalid diskstats line {line}: {reason}")
            }
        }
    }
}

impl std::error::Error for DiskstatsError {}

#[derive(Debug, Default)]
pub struct DiskstatsSampler {
    previous: HashMap<String, (DiskStat, Instant)>,
}

impl DiskstatsSampler {
    /// Collect a native batch sample. The first observation for each device is
    /// only a baseline; rates appear from the next monotonic observation.
    pub fn sample(
        &mut self,
        path: &Path,
        now: Instant,
    ) -> Result<Vec<(String, DiskMetrics)>, DiskstatsError> {
        let content = std::fs::read_to_string(path).map_err(DiskstatsError::Read)?;
        let snapshot = parse_snapshot(&content)?;
        let present: HashSet<&str> = snapshot.iter().map(|stat| stat.name.as_str()).collect();
        self.previous
            .retain(|name, _| present.contains(name.as_str()));

        let mut metrics = Vec::new();
        for current in snapshot {
            if let Some((previous, observed_at)) = self.previous.get(&current.name)
                && let Some(value) = calculate_metrics(
                    previous,
                    &current,
                    now.saturating_duration_since(*observed_at),
                )
            {
                metrics.push((current.name.clone(), value));
            }
            self.previous.insert(current.name.clone(), (current, now));
        }
        Ok(metrics)
    }
}

/// Parse all devices from one batch snapshot. After major/minor/name, valid
/// Linux layouts contain 11 base, 15 discard, or at least 17 flush counters.
/// Future trailing counters are ignored.
pub fn parse_snapshot(content: &str) -> Result<Vec<DiskStat>, DiskstatsError> {
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| parse_line(line, index + 1))
        .collect()
}

fn parse_line(line: &str, line_number: usize) -> Result<DiskStat, DiskstatsError> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 14 {
        return Err(parse_error(
            line_number,
            "requires major, minor, name and 11 base counters",
        ));
    }
    let counter_count = fields.len() - 3;
    if matches!(counter_count, 12..=14 | 16) {
        return Err(parse_error(
            line_number,
            format!("incomplete optional counter group ({counter_count} counters)"),
        ));
    }

    let major = parse_field::<u32>(&fields, 0, "major", line_number)?;
    let minor = parse_field::<u32>(&fields, 1, "minor", line_number)?;
    let counter =
        |index: usize, name: &str| parse_field::<u64>(&fields, index + 3, name, line_number);

    Ok(DiskStat {
        major,
        minor,
        name: fields[2].to_owned(),
        reads_completed: counter(0, "reads_completed")?,
        reads_merged: counter(1, "reads_merged")?,
        sectors_read: counter(2, "sectors_read")?,
        time_reading_ms: counter(3, "time_reading_ms")?,
        writes_completed: counter(4, "writes_completed")?,
        writes_merged: counter(5, "writes_merged")?,
        sectors_written: counter(6, "sectors_written")?,
        time_writing_ms: counter(7, "time_writing_ms")?,
        ios_in_progress: counter(8, "ios_in_progress")?,
        io_ticks_ms: counter(9, "io_ticks_ms")?,
        weighted_time_in_queue_ms: counter(10, "weighted_time_in_queue_ms")?,
        discards_completed: optional_counter(&fields, counter_count, 11, line_number)?,
        discards_merged: optional_counter(&fields, counter_count, 12, line_number)?,
        sectors_discarded: optional_counter(&fields, counter_count, 13, line_number)?,
        time_discarding_ms: optional_counter(&fields, counter_count, 14, line_number)?,
        flush_requests: optional_counter(&fields, counter_count, 15, line_number)?,
        time_flushing_ms: optional_counter(&fields, counter_count, 16, line_number)?,
    })
}

fn optional_counter(
    fields: &[&str],
    counter_count: usize,
    index: usize,
    line_number: usize,
) -> Result<Option<u64>, DiskstatsError> {
    (counter_count > index)
        .then(|| parse_field(fields, index + 3, "optional counter", line_number))
        .transpose()
}

fn parse_field<T: std::str::FromStr>(
    fields: &[&str],
    index: usize,
    name: &str,
    line_number: usize,
) -> Result<T, DiskstatsError> {
    fields[index]
        .parse()
        .map_err(|_| parse_error(line_number, format!("{name} is not an unsigned integer")))
}

fn parse_error(line: usize, reason: impl Into<String>) -> DiskstatsError {
    DiskstatsError::Parse {
        line,
        reason: reason.into(),
    }
}

/// Calculate metrics for one monotonic interval. Any decreasing mandatory
/// counter indicates reset/replacement, so the entire interval is unavailable.
pub fn calculate_metrics(
    previous: &DiskStat,
    current: &DiskStat,
    elapsed: Duration,
) -> Option<DiskMetrics> {
    let elapsed_seconds = elapsed.as_secs_f64();
    let elapsed_ms = elapsed.as_secs_f64() * 1_000.0;
    if elapsed_seconds <= 0.0 || previous.major != current.major || previous.minor != current.minor
    {
        return None;
    }

    let reads = delta(current.reads_completed, previous.reads_completed)?;
    let sectors_read = delta(current.sectors_read, previous.sectors_read)?;
    let read_time = delta(current.time_reading_ms, previous.time_reading_ms)?;
    let writes = delta(current.writes_completed, previous.writes_completed)?;
    let sectors_written = delta(current.sectors_written, previous.sectors_written)?;
    let write_time = delta(current.time_writing_ms, previous.time_writing_ms)?;
    let io_ticks = delta(current.io_ticks_ms, previous.io_ticks_ms)?;
    let weighted_time = delta(
        current.weighted_time_in_queue_ms,
        previous.weighted_time_in_queue_ms,
    )?;

    Some(DiskMetrics {
        read_mib_per_sec: sectors_read as f64 * SECTOR_BYTES / MIB_BYTES / elapsed_seconds,
        write_mib_per_sec: sectors_written as f64 * SECTOR_BYTES / MIB_BYTES / elapsed_seconds,
        read_iops: reads as f64 / elapsed_seconds,
        write_iops: writes as f64 / elapsed_seconds,
        utilization_percent: io_ticks as f64 / elapsed_ms * 100.0,
        average_read_latency_ms: (reads > 0).then_some(read_time as f64 / reads as f64),
        average_write_latency_ms: (writes > 0).then_some(write_time as f64 / writes as f64),
        average_queue_depth: weighted_time as f64 / elapsed_ms,
        ios_in_progress: current.ios_in_progress,
    })
}

fn delta(current: u64, previous: u64) -> Option<u64> {
    current.checked_sub(previous)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "8 0 sda 10 1 2048 20 20 2 4096 40 3 500 700";
    const DISCARD: &str = "259 0 nvme0n1 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15";
    const FLUSH: &str = "253 0 dm-0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17";

    #[test]
    fn parses_base_discard_flush_and_multiple_devices() {
        let snapshot = parse_snapshot(&format!("{BASE}\n{DISCARD}\n{FLUSH}\n")).unwrap();

        assert_eq!(snapshot.len(), 3);
        assert_eq!(snapshot[0].name, "sda");
        assert_eq!(snapshot[0].discards_completed, None);
        assert_eq!(snapshot[1].discards_completed, Some(12));
        assert_eq!(snapshot[1].flush_requests, None);
        assert_eq!(snapshot[2].flush_requests, Some(16));
        assert_eq!(snapshot[2].time_flushing_ms, Some(17));
    }

    #[test]
    fn rejects_malformed_mandatory_and_incomplete_optional_fields() {
        assert!(parse_snapshot("8 0 sda 1 2 bad 4 5 6 7 8 9 10 11").is_err());
        assert!(parse_snapshot("8 0 sda 1 2 3 4 5 6 7 8 9 10 11 12").is_err());
    }

    #[test]
    fn calculates_exact_metrics_using_512_byte_sectors() {
        let previous = stat([0, 0, 0, 0, 0, 0, 0, 0]);
        let current = stat([100, 2048, 500, 50, 4096, 1_000, 750, 1_500]);

        let metrics = calculate_metrics(&previous, &current, Duration::from_secs(1)).unwrap();

        assert_eq!(metrics.read_mib_per_sec, 1.0);
        assert_eq!(metrics.write_mib_per_sec, 2.0);
        assert_eq!(metrics.read_iops, 100.0);
        assert_eq!(metrics.write_iops, 50.0);
        assert_eq!(metrics.utilization_percent, 75.0);
        assert_eq!(metrics.average_read_latency_ms, Some(5.0));
        assert_eq!(metrics.average_write_latency_ms, Some(20.0));
        assert_eq!(metrics.average_queue_depth, 1.5);
    }

    #[test]
    fn idle_interval_has_unavailable_latency_not_zero_latency() {
        let previous = stat([1, 2, 3, 4, 5, 6, 7, 8]);
        let mut current = previous.clone();
        current.ios_in_progress = 4;

        let metrics = calculate_metrics(&previous, &current, Duration::from_secs(1)).unwrap();

        assert_eq!(metrics.average_read_latency_ms, None);
        assert_eq!(metrics.average_write_latency_ms, None);
        assert_eq!(metrics.ios_in_progress, 4);
    }

    #[test]
    fn reset_zero_interval_and_dev_t_change_skip_interval() {
        let previous = stat([10, 20, 30, 40, 50, 60, 70, 80]);
        let reset = stat([9, 19, 29, 39, 49, 59, 69, 79]);
        assert!(calculate_metrics(&previous, &reset, Duration::from_secs(1)).is_none());
        assert!(calculate_metrics(&previous, &previous, Duration::ZERO).is_none());

        let mut replaced = previous.clone();
        replaced.minor = 1;
        assert!(calculate_metrics(&previous, &replaced, Duration::from_secs(1)).is_none());
    }

    fn stat(counters: [u64; 8]) -> DiskStat {
        let [
            reads,
            sectors_read,
            read_time,
            writes,
            sectors_written,
            write_time,
            io_ticks,
            weighted_time,
        ] = counters;
        DiskStat {
            major: 8,
            minor: 0,
            name: "sda".into(),
            reads_completed: reads,
            reads_merged: 0,
            sectors_read,
            time_reading_ms: read_time,
            writes_completed: writes,
            writes_merged: 0,
            sectors_written,
            time_writing_ms: write_time,
            ios_in_progress: 0,
            io_ticks_ms: io_ticks,
            weighted_time_in_queue_ms: weighted_time,
            discards_completed: None,
            discards_merged: None,
            sectors_discarded: None,
            time_discarding_ms: None,
            flush_requests: None,
            time_flushing_ms: None,
        }
    }
}
