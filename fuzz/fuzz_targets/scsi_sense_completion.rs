#![no_main]

use libfuzzer_sys::fuzz_target;
use vault_watch::scsi::{
    TransportCompletion, parse_sense, sense_action, validate_completion,
};

fuzz_target!(|data: &[u8]| {
    if let Ok(sense) = parse_sense(data) {
        let retry_attempted = data.get(4).is_some_and(|value| value & 1 != 0);
        let optional_command = data.get(5).is_some_and(|value| value & 1 != 0);
        let _ = sense_action(sense, optional_command, retry_attempted);
    }

    if data.len() >= 14 {
        let completion = TransportCompletion {
            ioctl_succeeded: data[0] & 1 != 0,
            scsi_status: data[1],
            host_status: u16::from_be_bytes([data[2], data[3]]),
            driver_status: u16::from_be_bytes([data[4], data[5]]),
            residual: i32::from_be_bytes([data[6], data[7], data[8], data[9]]),
            requested_len: usize::from(u16::from_be_bytes([data[10], data[11]])),
            sense_written: usize::from(data[12]),
            sense_capacity: usize::from(data[13]),
        };
        let _ = validate_completion(completion);
    }
});
