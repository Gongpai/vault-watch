#![no_main]

use libfuzzer_sys::fuzz_target;
use vault_watch::ata::{
    AtaMedium, IdentifyDevice, MetricDirection, RawValueDecoder, SmartAttribute,
    VendorAttributeRule, VendorSchema, VendorSchemaSource, interpret_vendor_attributes,
};

fuzz_target!(|data: &[u8]| {
    let mut raw = [0u8; 6];
    let copied = data.len().min(raw.len());
    raw[..copied].copy_from_slice(&data[..copied]);
    let decoder = match data.get(6).copied().unwrap_or_default() % 3 {
        0 => RawValueDecoder::LittleEndianU48,
        1 => RawValueDecoder::LittleEndianU32,
        _ => RawValueDecoder::Byte(data.get(7).copied().unwrap_or_default()),
    };
    let multiplier = data
        .get(8..16)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u64::from_le_bytes)
        .unwrap_or(1);
    let rule = [VendorAttributeRule {
        attribute_id: 1,
        metric: "synthetic_metric",
        unit: "synthetic_unit",
        decoder,
        multiplier,
        direction: MetricDirection::Increasing,
    }];
    let schema = [VendorSchema {
        schema_id: "synthetic-fuzz-v1",
        model_prefix: "SYNTH-",
        firmware_prefix: "FW-",
        source: VendorSchemaSource {
            document_id: "SYNTH-FUZZ",
            revision: "1",
            url: "https://invalid.example/synthetic-fuzz",
        },
        rules: &rule,
    }];
    let identify = IdentifyDevice {
        serial: String::new(),
        firmware: "FW-FUZZ".to_owned(),
        model: "SYNTH-FUZZ".to_owned(),
        lba28_sectors: 0,
        lba48_sectors: None,
        logical_sector_bytes: None,
        physical_sector_bytes: None,
        capacity_bytes: None,
        medium: AtaMedium::Unknown,
        smart_supported: true,
        general_purpose_logging_supported: false,
    };
    let attributes = [SmartAttribute {
        id: 1,
        flags: 0,
        current: None,
        worst: None,
        raw,
        threshold: None,
    }];
    let _ = interpret_vendor_attributes(&schema, &identify, &attributes);
});
