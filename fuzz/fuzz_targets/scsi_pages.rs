#![no_main]

use libfuzzer_sys::fuzz_target;
use vault_watch::scsi::{
    LogPage, parse_block_device_characteristics_vpd, parse_device_identification_vpd,
    parse_error_counter_log, parse_informational_exception_log, parse_non_medium_error_log,
    parse_standard_inquiry, parse_supported_log_pages, parse_supported_vpd_pages,
    parse_temperature_log,
};

fuzz_target!(|data: &[u8]| {
    let _ = parse_standard_inquiry(data);
    let _ = parse_supported_vpd_pages(data);
    let _ = parse_device_identification_vpd(data);
    let _ = parse_block_device_characteristics_vpd(data);
    let _ = parse_supported_log_pages(data);
    let _ = parse_temperature_log(data);
    let _ = parse_error_counter_log(data, LogPage::ReadErrors);
    let _ = parse_error_counter_log(data, LogPage::WriteErrors);
    let _ = parse_non_medium_error_log(data);
    let _ = parse_informational_exception_log(data);
});
