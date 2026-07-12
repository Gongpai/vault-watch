#![no_main]

use libfuzzer_sys::fuzz_target;
use vault_watch::ata::{
    apply_thresholds, parse_identify_device, parse_log_directory, parse_smart_attributes,
    parse_smart_thresholds,
};

fuzz_target!(|data: &[u8]| {
    let _ = parse_identify_device(data);
    let _ = parse_log_directory(data);

    if let Ok(mut attributes) = parse_smart_attributes(data) {
        if let Ok(thresholds) = parse_smart_thresholds(data) {
            apply_thresholds(&mut attributes, &thresholds);
            for attribute in attributes {
                let _ = attribute.threshold_state();
            }
        }
    }
});
