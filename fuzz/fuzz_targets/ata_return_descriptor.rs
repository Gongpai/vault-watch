#![no_main]

use libfuzzer_sys::fuzz_target;
use vault_watch::ata::{parse_ata_return_descriptor, smart_return_status};

fuzz_target!(|data: &[u8]| {
    if let Ok(registers) = parse_ata_return_descriptor(data) {
        let _ = smart_return_status(registers);
    }
});
