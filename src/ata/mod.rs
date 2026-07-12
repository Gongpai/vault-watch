//! Pure ATA/SAT health protocol foundation.
//!
//! No device paths, ioctls, arbitrary taskfiles, data-out commands, self-test,
//! firmware, security, sanitize, or write operations are exposed here.

const ATA_PASS_THROUGH_16: u8 = 0x85;
const ATA_IDENTIFY_DEVICE: u8 = 0xec;
const ATA_SMART: u8 = 0xb0;
const ATA_READ_LOG_EXT: u8 = 0x2f;
const SMART_READ_DATA: u8 = 0xd0;
const SMART_READ_THRESHOLDS: u8 = 0xd1;
const SMART_RETURN_STATUS: u8 = 0xda;
const SMART_LBA_MID: u8 = 0x4f;
const SMART_LBA_HIGH: u8 = 0xc2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtaReadCommand {
    IdentifyDevice,
    SmartReadData,
    SmartReadThresholds,
    SmartReturnStatus,
    ReadGplDirectory,
}

impl AtaReadCommand {
    pub const fn data_len(self) -> usize {
        match self {
            Self::SmartReturnStatus => 0,
            _ => 512,
        }
    }

    pub const fn cdb(self) -> [u8; 16] {
        let (protocol, extend, transfer, feature, count, lba_mid, lba_high, command) = match self {
            Self::IdentifyDevice => (4, false, 0x2e, 0, 1, 0, 0, ATA_IDENTIFY_DEVICE),
            Self::SmartReadData => (
                4,
                false,
                0x2e,
                SMART_READ_DATA,
                1,
                SMART_LBA_MID,
                SMART_LBA_HIGH,
                ATA_SMART,
            ),
            Self::SmartReadThresholds => (
                4,
                false,
                0x2e,
                SMART_READ_THRESHOLDS,
                1,
                SMART_LBA_MID,
                SMART_LBA_HIGH,
                ATA_SMART,
            ),
            Self::SmartReturnStatus => (
                3,
                false,
                0x20,
                SMART_RETURN_STATUS,
                0,
                SMART_LBA_MID,
                SMART_LBA_HIGH,
                ATA_SMART,
            ),
            Self::ReadGplDirectory => (4, true, 0x2e, 0, 1, 0, 0, ATA_READ_LOG_EXT),
        };
        [
            ATA_PASS_THROUGH_16,
            (protocol << 1) | extend as u8,
            transfer,
            0,
            feature,
            0,
            count,
            0,
            0,
            0,
            lba_mid,
            0,
            lba_high,
            0xa0,
            command,
            0,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtaParseError {
    WrongLength { expected: usize, actual: usize },
    InvalidChecksum,
    TruncatedDescriptor,
    MissingAtaReturnDescriptor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtaLogPageSupport {
    pub address: u8,
    pub sectors: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtaLogDirectory {
    pub version: u16,
    pub supported_pages: Vec<AtaLogPageSupport>,
}

/// Parses the 512-byte GPL log directory without interpreting vendor pages.
pub fn parse_log_directory(data: &[u8]) -> Result<AtaLogDirectory, AtaParseError> {
    let words = ata_words(data)?;
    let supported_pages = words[1..]
        .iter()
        .enumerate()
        .filter_map(|(index, sectors)| {
            (*sectors != 0).then_some(AtaLogPageSupport {
                address: u8::try_from(index + 1).expect("GPL directory has 255 page entries"),
                sectors: *sectors,
            })
        })
        .collect();
    Ok(AtaLogDirectory {
        version: words[0],
        supported_pages,
    })
}

/// Builds only READ LOG EXT page 0 (the directory), and only after IDENTIFY
/// advertised General Purpose Logging. No caller-controlled address is exposed.
pub const fn read_log_directory_cdb(identify: &IdentifyDevice) -> Option<[u8; 16]> {
    if !identify.general_purpose_logging_supported {
        return None;
    }
    Some([
        ATA_PASS_THROUGH_16,
        (4 << 1) | 1,
        0x2e,
        0,
        0,
        0,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
        0xa0,
        ATA_READ_LOG_EXT,
        0,
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtaMedium {
    Unknown,
    SolidState,
    RotationalRpm(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifyDevice {
    pub serial: String,
    pub firmware: String,
    pub model: String,
    pub lba28_sectors: u32,
    pub lba48_sectors: Option<u64>,
    pub logical_sector_bytes: Option<u32>,
    pub physical_sector_bytes: Option<u64>,
    pub capacity_bytes: Option<u128>,
    pub medium: AtaMedium,
    pub smart_supported: bool,
    pub general_purpose_logging_supported: bool,
}

pub fn parse_identify_device(data: &[u8]) -> Result<IdentifyDevice, AtaParseError> {
    let words = ata_words(data)?;
    let lba28_sectors = u32::from(words[60]) | (u32::from(words[61]) << 16);
    let command_set_1_valid = validity_bits_set(words[83]);
    let command_set_2_valid = validity_bits_set(words[87]);
    let lba48_valid = command_set_1_valid && words[83] & (1 << 10) != 0;
    let lba48_sectors = lba48_valid.then(|| {
        u64::from(words[100])
            | (u64::from(words[101]) << 16)
            | (u64::from(words[102]) << 32)
            | (u64::from(words[103]) << 48)
    });
    let medium = match words[217] {
        0x0001 => AtaMedium::SolidState,
        rpm @ 0x0401..=0xfffe => AtaMedium::RotationalRpm(rpm),
        _ => AtaMedium::Unknown,
    };
    let (logical_sector_bytes, physical_sector_bytes) = sector_sizes(&words);
    let sectors = lba48_sectors
        .filter(|sectors| *sectors != 0)
        .or_else(|| (lba28_sectors != 0).then_some(u64::from(lba28_sectors)));
    let capacity_bytes = sectors
        .zip(logical_sector_bytes)
        .map(|(sectors, bytes)| u128::from(sectors) * u128::from(bytes));
    Ok(IdentifyDevice {
        serial: ata_string(data, 10, 10),
        firmware: ata_string(data, 23, 4),
        model: ata_string(data, 27, 20),
        lba28_sectors,
        lba48_sectors,
        logical_sector_bytes,
        physical_sector_bytes,
        capacity_bytes,
        medium,
        smart_supported: command_set_1_valid && words[82] & 1 != 0,
        general_purpose_logging_supported: command_set_2_valid && words[84] & (1 << 5) != 0,
    })
}

const fn validity_bits_set(word: u16) -> bool {
    word & 0xc000 == 0x4000
}

fn sector_sizes(words: &[u16; 256]) -> (Option<u32>, Option<u64>) {
    let geometry_valid = validity_bits_set(words[106]);
    let logical = if geometry_valid && words[106] & (1 << 12) != 0 {
        let logical_words = u32::from(words[117]) | (u32::from(words[118]) << 16);
        logical_words.checked_mul(2).filter(|bytes| *bytes >= 512)
    } else {
        Some(512)
    };
    let physical = logical.and_then(|logical| {
        if geometry_valid && words[106] & (1 << 13) != 0 {
            u64::from(logical).checked_shl(u32::from(words[106] & 0x000f))
        } else {
            Some(u64::from(logical))
        }
    });
    (logical, physical)
}

fn ata_words(data: &[u8]) -> Result<[u16; 256], AtaParseError> {
    if data.len() != 512 {
        return Err(AtaParseError::WrongLength {
            expected: 512,
            actual: data.len(),
        });
    }
    let mut words = [0u16; 256];
    for (index, chunk) in data.chunks_exact(2).enumerate() {
        words[index] = u16::from_le_bytes([chunk[0], chunk[1]]);
    }
    Ok(words)
}

fn ata_string(data: &[u8], first_word: usize, word_count: usize) -> String {
    let start = first_word * 2;
    let end = start + word_count * 2;
    let mut value = Vec::with_capacity(word_count * 2);
    for chunk in data[start..end].chunks_exact(2) {
        value.extend_from_slice(&[chunk[1], chunk[0]]);
    }
    String::from_utf8_lossy(&value)
        .trim_matches([' ', '\0'])
        .to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartAttribute {
    pub id: u8,
    pub flags: u16,
    pub current: Option<u8>,
    pub worst: Option<u8>,
    /// Vendor-specific bytes. No unit or semantic meaning is assigned here.
    pub raw: [u8; 6],
    pub threshold: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawValueDecoder {
    LittleEndianU48,
    LittleEndianU32,
    Byte(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricDirection {
    Increasing,
    Decreasing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorAttributeRule<'a> {
    pub attribute_id: u8,
    pub metric: &'a str,
    pub unit: &'a str,
    pub decoder: RawValueDecoder,
    pub multiplier: u64,
    pub direction: MetricDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorSchemaSource<'a> {
    pub document_id: &'a str,
    pub revision: &'a str,
    pub url: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorSchema<'a> {
    pub schema_id: &'a str,
    pub model_prefix: &'a str,
    pub firmware_prefix: &'a str,
    pub source: VendorSchemaSource<'a>,
    pub rules: &'a [VendorAttributeRule<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpretedVendorMetric<'a> {
    pub attribute_id: u8,
    pub metric: &'a str,
    pub value: u64,
    pub unit: &'a str,
    pub direction: MetricDirection,
    pub source: VendorSchemaSource<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownSchemaReason {
    NoMatch,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorInterpretation<'a> {
    UnknownSchema(UnknownSchemaReason),
    Matched {
        schema_id: &'a str,
        metrics: Vec<InterpretedVendorMetric<'a>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorSchemaError {
    InvalidSchema,
    DuplicateAttribute(u8),
    InvalidByteIndex(u8),
    ValueOverflow(u8),
}

pub fn interpret_vendor_attributes<'a>(
    schemas: &'a [VendorSchema<'a>],
    identify: &IdentifyDevice,
    attributes: &[SmartAttribute],
) -> Result<VendorInterpretation<'a>, VendorSchemaError> {
    for schema in schemas {
        validate_vendor_schema(schema)?;
    }
    let mut matches = schemas.iter().filter(|schema| {
        identify.model.starts_with(schema.model_prefix)
            && identify.firmware.starts_with(schema.firmware_prefix)
    });
    let Some(schema) = matches.next() else {
        return Ok(VendorInterpretation::UnknownSchema(
            UnknownSchemaReason::NoMatch,
        ));
    };
    if matches.next().is_some() {
        return Ok(VendorInterpretation::UnknownSchema(
            UnknownSchemaReason::Ambiguous,
        ));
    }

    let mut metrics = Vec::new();
    for rule in schema.rules {
        let Some(attribute) = attributes
            .iter()
            .find(|value| value.id == rule.attribute_id)
        else {
            continue;
        };
        let raw = decode_vendor_raw(attribute.raw, rule.decoder)?;
        let value = raw
            .checked_mul(rule.multiplier)
            .ok_or(VendorSchemaError::ValueOverflow(rule.attribute_id))?;
        metrics.push(InterpretedVendorMetric {
            attribute_id: rule.attribute_id,
            metric: rule.metric,
            value,
            unit: rule.unit,
            direction: rule.direction,
            source: schema.source,
        });
    }
    Ok(VendorInterpretation::Matched {
        schema_id: schema.schema_id,
        metrics,
    })
}

fn validate_vendor_schema(schema: &VendorSchema<'_>) -> Result<(), VendorSchemaError> {
    if schema.schema_id.is_empty()
        || schema.model_prefix.is_empty()
        || schema.firmware_prefix.is_empty()
        || schema.source.document_id.is_empty()
        || schema.source.revision.is_empty()
        || schema.source.url.is_empty()
        || schema.rules.is_empty()
        || schema
            .rules
            .iter()
            .any(|rule| rule.metric.is_empty() || rule.unit.is_empty() || rule.multiplier == 0)
    {
        return Err(VendorSchemaError::InvalidSchema);
    }
    for (index, rule) in schema.rules.iter().enumerate() {
        if schema.rules[..index]
            .iter()
            .any(|other| other.attribute_id == rule.attribute_id)
        {
            return Err(VendorSchemaError::DuplicateAttribute(rule.attribute_id));
        }
        if matches!(rule.decoder, RawValueDecoder::Byte(byte) if byte >= 6) {
            return Err(VendorSchemaError::InvalidByteIndex(match rule.decoder {
                RawValueDecoder::Byte(byte) => byte,
                _ => unreachable!(),
            }));
        }
    }
    Ok(())
}

fn decode_vendor_raw(raw: [u8; 6], decoder: RawValueDecoder) -> Result<u64, VendorSchemaError> {
    Ok(match decoder {
        RawValueDecoder::LittleEndianU48 => {
            u64::from(raw[0])
                | (u64::from(raw[1]) << 8)
                | (u64::from(raw[2]) << 16)
                | (u64::from(raw[3]) << 24)
                | (u64::from(raw[4]) << 32)
                | (u64::from(raw[5]) << 40)
        }
        RawValueDecoder::LittleEndianU32 => {
            u64::from(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
        }
        RawValueDecoder::Byte(index) => u64::from(
            *raw.get(usize::from(index))
                .ok_or(VendorSchemaError::InvalidByteIndex(index))?,
        ),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdState {
    Unavailable,
    NotApplicable,
    Passing,
    Exceeded,
}

impl SmartAttribute {
    pub const fn threshold_state(&self) -> ThresholdState {
        match (self.current, self.threshold) {
            (_, Some(0)) => ThresholdState::NotApplicable,
            (Some(current), Some(threshold)) if current <= threshold => ThresholdState::Exceeded,
            (Some(_), Some(_)) => ThresholdState::Passing,
            _ => ThresholdState::Unavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtaBackendRoute {
    NativeSat,
    QualifiedUsbSat,
    NativeScsi,
    ControllerHidden,
    UnsupportedUsbBridge,
    UnsupportedDevice,
    Ambiguous,
    InsufficientEvidence,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AtaRoutingEvidence {
    pub direct_access_block_device: bool,
    pub libata_managed: bool,
    pub ata_information_vpd: bool,
    pub usb_transport: bool,
    pub qualified_usb_sat: bool,
    pub controller_logical_volume: bool,
    pub native_scsi_evidence: bool,
}

/// Selects a backend from discovery evidence only; it never probes a device.
pub const fn route_ata_backend(evidence: AtaRoutingEvidence) -> AtaBackendRoute {
    if evidence.controller_logical_volume {
        return AtaBackendRoute::ControllerHidden;
    }
    if !evidence.direct_access_block_device {
        return AtaBackendRoute::UnsupportedDevice;
    }
    if evidence.usb_transport {
        return if evidence.qualified_usb_sat {
            AtaBackendRoute::QualifiedUsbSat
        } else {
            AtaBackendRoute::UnsupportedUsbBridge
        };
    }
    let sat = evidence.libata_managed || evidence.ata_information_vpd;
    if sat && evidence.native_scsi_evidence {
        AtaBackendRoute::Ambiguous
    } else if sat {
        AtaBackendRoute::NativeSat
    } else if evidence.native_scsi_evidence {
        AtaBackendRoute::NativeScsi
    } else {
        AtaBackendRoute::InsufficientEvidence
    }
}

pub fn parse_smart_attributes(data: &[u8]) -> Result<Vec<SmartAttribute>, AtaParseError> {
    validate_smart_page(data)?;
    let mut attributes = Vec::new();
    for entry in data[2..362].chunks_exact(12) {
        if entry[0] == 0 {
            continue;
        }
        attributes.push(SmartAttribute {
            id: entry[0],
            flags: u16::from_le_bytes([entry[1], entry[2]]),
            current: normalized(entry[3]),
            worst: normalized(entry[4]),
            raw: entry[5..11].try_into().expect("fixed SMART raw width"),
            threshold: None,
        });
    }
    Ok(attributes)
}

pub fn parse_smart_thresholds(data: &[u8]) -> Result<Vec<(u8, u8)>, AtaParseError> {
    validate_smart_page(data)?;
    Ok(data[2..362]
        .chunks_exact(12)
        .filter_map(|entry| (entry[0] != 0).then_some((entry[0], entry[1])))
        .collect())
}

pub fn apply_thresholds(attributes: &mut [SmartAttribute], thresholds: &[(u8, u8)]) {
    for attribute in attributes {
        attribute.threshold = thresholds
            .iter()
            .find_map(|(id, threshold)| (*id == attribute.id).then_some(*threshold));
    }
}

fn validate_smart_page(data: &[u8]) -> Result<(), AtaParseError> {
    if data.len() != 512 {
        return Err(AtaParseError::WrongLength {
            expected: 512,
            actual: data.len(),
        });
    }
    if data.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)) != 0 {
        return Err(AtaParseError::InvalidChecksum);
    }
    Ok(())
}

const fn normalized(value: u8) -> Option<u8> {
    match value {
        1..=253 => Some(value),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtaReturnDescriptor {
    pub extend: bool,
    pub error: u8,
    pub sector_count: u16,
    pub lba_mid: u16,
    pub lba_high: u16,
    pub device: u8,
    pub status: u8,
}

pub fn parse_ata_return_descriptor(sense: &[u8]) -> Result<AtaReturnDescriptor, AtaParseError> {
    if sense.len() < 8 || !matches!(sense[0] & 0x7f, 0x72 | 0x73) {
        return Err(AtaParseError::TruncatedDescriptor);
    }
    let end = 8usize.saturating_add(sense[7] as usize).min(sense.len());
    let mut offset = 8;
    while offset < end {
        if offset + 2 > end {
            return Err(AtaParseError::TruncatedDescriptor);
        }
        let length = sense[offset + 1] as usize;
        let next = offset + 2 + length;
        if next > end {
            return Err(AtaParseError::TruncatedDescriptor);
        }
        if sense[offset] == 0x09 {
            if length < 0x0c {
                return Err(AtaParseError::TruncatedDescriptor);
            }
            return Ok(AtaReturnDescriptor {
                extend: sense[offset + 2] & 1 != 0,
                error: sense[offset + 3],
                sector_count: u16::from_be_bytes([sense[offset + 4], sense[offset + 5]]),
                lba_mid: u16::from_be_bytes([sense[offset + 8], sense[offset + 9]]),
                lba_high: u16::from_be_bytes([sense[offset + 10], sense[offset + 11]]),
                device: sense[offset + 12],
                status: sense[offset + 13],
            });
        }
        offset = next;
    }
    Err(AtaParseError::MissingAtaReturnDescriptor)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartStatus {
    Passed,
    PredictingFailure,
    Unknown,
}

pub const fn smart_return_status(registers: AtaReturnDescriptor) -> SmartStatus {
    let mid = registers.lba_mid as u8;
    let high = registers.lba_high as u8;
    match (mid, high) {
        (0x4f, 0xc2) => SmartStatus::Passed,
        (0xf4, 0x2c) => SmartStatus::PredictingFailure,
        _ => SmartStatus::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_word(data: &mut [u8; 512], index: usize, value: u16) {
        data[index * 2..index * 2 + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn set_ata_string(data: &mut [u8; 512], first_word: usize, words: usize, value: &str) {
        let mut padded = vec![b' '; words * 2];
        padded[..value.len()].copy_from_slice(value.as_bytes());
        for (index, chunk) in padded.chunks_exact(2).enumerate() {
            data[(first_word + index) * 2] = chunk[1];
            data[(first_word + index) * 2 + 1] = chunk[0];
        }
    }

    fn checksum(page: &mut [u8; 512]) {
        page[511] = 0u8.wrapping_sub(page[..511].iter().fold(0u8, |s, b| s.wrapping_add(*b)));
    }

    #[test]
    fn typed_sat_builders_expose_only_allowlisted_data_in_or_non_data_commands() {
        let identify = AtaReadCommand::IdentifyDevice.cdb();
        assert_eq!(identify[0], 0x85);
        assert_eq!(identify[1], 4 << 1);
        assert_eq!(identify[2], 0x2e);
        assert_eq!(identify[14], 0xec);
        let status = AtaReadCommand::SmartReturnStatus.cdb();
        assert_eq!(status[1], 3 << 1);
        assert_eq!(status[2], 0x20);
        assert_eq!(status[4], 0xda);
        assert_eq!(AtaReadCommand::SmartReturnStatus.data_len(), 0);
        let directory = AtaReadCommand::ReadGplDirectory.cdb();
        assert_eq!(directory[1], (4 << 1) | 1);
        assert_eq!(directory[14], ATA_READ_LOG_EXT);
        assert_eq!(AtaReadCommand::ReadGplDirectory.data_len(), 512);
    }

    #[test]
    fn parses_identify_strings_capacity_smart_and_rotation() {
        let mut data = [0u8; 512];
        set_ata_string(&mut data, 10, 10, "SYNTHETIC");
        set_ata_string(&mut data, 23, 4, "FW1");
        set_ata_string(&mut data, 27, 20, "FIXTURE MODEL");
        set_word(&mut data, 60, 0x5678);
        set_word(&mut data, 61, 0x1234);
        set_word(&mut data, 82, 1);
        set_word(&mut data, 83, (1 << 14) | (1 << 10));
        set_word(&mut data, 84, 1 << 5);
        set_word(&mut data, 87, 1 << 14);
        set_word(&mut data, 100, 1);
        set_word(&mut data, 106, (1 << 14) | (1 << 13) | (1 << 12) | 3);
        set_word(&mut data, 117, 2048);
        set_word(&mut data, 217, 1);
        let identify = parse_identify_device(&data).unwrap();
        assert_eq!(identify.model, "FIXTURE MODEL");
        assert_eq!(identify.lba28_sectors, 0x1234_5678);
        assert_eq!(identify.lba48_sectors, Some(1));
        assert_eq!(identify.logical_sector_bytes, Some(4096));
        assert_eq!(identify.physical_sector_bytes, Some(32768));
        assert_eq!(identify.capacity_bytes, Some(4096));
        assert_eq!(identify.medium, AtaMedium::SolidState);
        assert!(identify.smart_supported);
        assert!(identify.general_purpose_logging_supported);
    }

    #[test]
    fn invalid_capability_words_do_not_enable_features_or_trust_extended_geometry() {
        let mut data = [0u8; 512];
        set_word(&mut data, 60, 2);
        set_word(&mut data, 82, 1);
        set_word(&mut data, 84, 1 << 5);
        set_word(&mut data, 106, (1 << 15) | (1 << 12));
        set_word(&mut data, 117, 2048);
        let identify = parse_identify_device(&data).unwrap();
        assert!(!identify.smart_supported);
        assert!(!identify.general_purpose_logging_supported);
        assert_eq!(identify.logical_sector_bytes, Some(512));
        assert_eq!(identify.physical_sector_bytes, Some(512));
        assert_eq!(identify.capacity_bytes, Some(1024));
        assert_eq!(read_log_directory_cdb(&identify), None);
    }

    #[test]
    fn gpl_directory_command_is_fixed_read_only_and_capability_gated() {
        let mut data = [0u8; 512];
        set_word(&mut data, 84, 1 << 5);
        set_word(&mut data, 87, 1 << 14);
        let identify = parse_identify_device(&data).unwrap();
        let cdb = read_log_directory_cdb(&identify).unwrap();
        assert_eq!(cdb[0], ATA_PASS_THROUGH_16);
        assert_eq!(cdb[1], (4 << 1) | 1);
        assert_eq!(cdb[2], 0x2e);
        assert_eq!(cdb[6], 1);
        assert_eq!(cdb[8], 0);
        assert_eq!(cdb[14], ATA_READ_LOG_EXT);
    }

    #[test]
    fn parses_gpl_directory_and_preserves_unknown_page_addresses() {
        let mut data = [0u8; 512];
        set_word(&mut data, 0, 1);
        set_word(&mut data, 3, 4);
        set_word(&mut data, 4, 2);
        set_word(&mut data, 0x80, 7);
        let directory = parse_log_directory(&data).unwrap();
        assert_eq!(directory.version, 1);
        assert_eq!(
            directory.supported_pages,
            vec![
                AtaLogPageSupport {
                    address: 3,
                    sectors: 4
                },
                AtaLogPageSupport {
                    address: 4,
                    sectors: 2
                },
                AtaLogPageSupport {
                    address: 0x80,
                    sectors: 7
                },
            ]
        );
        assert!(matches!(
            parse_log_directory(&data[..511]),
            Err(AtaParseError::WrongLength { .. })
        ));
    }

    #[test]
    fn smart_attributes_keep_raw_vendor_bytes_and_match_thresholds_by_id() {
        let mut data = [0u8; 512];
        data[2..14].copy_from_slice(&[5, 1, 0, 90, 80, 1, 2, 3, 4, 5, 6, 0]);
        checksum(&mut data);
        let mut attributes = parse_smart_attributes(&data).unwrap();
        assert_eq!(attributes[0].raw, [1, 2, 3, 4, 5, 6]);
        let mut thresholds = [0u8; 512];
        thresholds[2] = 5;
        thresholds[3] = 10;
        checksum(&mut thresholds);
        apply_thresholds(
            &mut attributes,
            &parse_smart_thresholds(&thresholds).unwrap(),
        );
        assert_eq!(attributes[0].threshold, Some(10));
        assert_eq!(attributes[0].threshold_state(), ThresholdState::Passing);
        attributes[0].current = Some(10);
        assert_eq!(attributes[0].threshold_state(), ThresholdState::Exceeded);
        attributes[0].current = None;
        assert_eq!(attributes[0].threshold_state(), ThresholdState::Unavailable);
        attributes[0].threshold = Some(0);
        assert_eq!(
            attributes[0].threshold_state(),
            ThresholdState::NotApplicable
        );
    }

    #[test]
    fn vendor_schema_requires_exact_family_firmware_and_provenance() {
        let mut data = [0u8; 512];
        set_ata_string(&mut data, 23, 4, "FW-A1");
        set_ata_string(&mut data, 27, 20, "SYNTH-ENTERPRISE-1");
        let identify = parse_identify_device(&data).unwrap();
        let attributes = [SmartAttribute {
            id: 241,
            flags: 0,
            current: Some(100),
            worst: Some(100),
            raw: [2, 0, 0, 0, 0, 0],
            threshold: None,
        }];
        let rules = [VendorAttributeRule {
            attribute_id: 241,
            metric: "synthetic_host_writes",
            unit: "bytes",
            decoder: RawValueDecoder::LittleEndianU48,
            multiplier: 512,
            direction: MetricDirection::Increasing,
        }];
        let schema = VendorSchema {
            schema_id: "synthetic-fixture-v1",
            model_prefix: "SYNTH-ENTERPRISE-",
            firmware_prefix: "FW-A",
            source: VendorSchemaSource {
                document_id: "SYNTH-DOC",
                revision: "1",
                url: "https://invalid.example/synthetic",
            },
            rules: &rules,
        };
        let schemas = [schema];
        assert_eq!(
            interpret_vendor_attributes(&schemas, &identify, &attributes).unwrap(),
            VendorInterpretation::Matched {
                schema_id: "synthetic-fixture-v1",
                metrics: vec![InterpretedVendorMetric {
                    attribute_id: 241,
                    metric: "synthetic_host_writes",
                    value: 1024,
                    unit: "bytes",
                    direction: MetricDirection::Increasing,
                    source: schema.source,
                }],
            }
        );
    }

    #[test]
    fn vendor_schema_mismatch_and_ambiguity_never_guess_a_metric() {
        let mut data = [0u8; 512];
        set_ata_string(&mut data, 23, 4, "OTHER");
        set_ata_string(&mut data, 27, 20, "SYNTH-MODEL");
        let identify = parse_identify_device(&data).unwrap();
        let rules = [VendorAttributeRule {
            attribute_id: 1,
            metric: "fixture",
            unit: "count",
            decoder: RawValueDecoder::Byte(0),
            multiplier: 1,
            direction: MetricDirection::Increasing,
        }];
        let source = VendorSchemaSource {
            document_id: "SYNTH-DOC",
            revision: "1",
            url: "https://invalid.example/synthetic",
        };
        let schema = VendorSchema {
            schema_id: "schema-a",
            model_prefix: "SYNTH-",
            firmware_prefix: "FW-",
            source,
            rules: &rules,
        };
        let mismatched_schemas = [schema];
        assert_eq!(
            interpret_vendor_attributes(&mismatched_schemas, &identify, &[]).unwrap(),
            VendorInterpretation::UnknownSchema(UnknownSchemaReason::NoMatch)
        );

        let matching = VendorSchema {
            firmware_prefix: "OTHER",
            ..schema
        };
        let duplicate_match = VendorSchema {
            schema_id: "schema-b",
            ..matching
        };
        let ambiguous_schemas = [matching, duplicate_match];
        assert_eq!(
            interpret_vendor_attributes(&ambiguous_schemas, &identify, &[]).unwrap(),
            VendorInterpretation::UnknownSchema(UnknownSchemaReason::Ambiguous)
        );
    }

    #[test]
    fn invalid_vendor_schema_and_conversion_overflow_are_explicit() {
        let identify = parse_identify_device(&[0u8; 512]).unwrap();
        let invalid_rules = [VendorAttributeRule {
            attribute_id: 1,
            metric: "fixture",
            unit: "count",
            decoder: RawValueDecoder::Byte(6),
            multiplier: 1,
            direction: MetricDirection::Increasing,
        }];
        let schema = VendorSchema {
            schema_id: "invalid",
            model_prefix: "MODEL",
            firmware_prefix: "FW",
            source: VendorSchemaSource {
                document_id: "SYNTH-DOC",
                revision: "1",
                url: "https://invalid.example/synthetic",
            },
            rules: &invalid_rules,
        };
        let invalid_schemas = [schema];
        assert_eq!(
            interpret_vendor_attributes(&invalid_schemas, &identify, &[]),
            Err(VendorSchemaError::InvalidByteIndex(6))
        );

        let mut matching_data = [0u8; 512];
        set_ata_string(&mut matching_data, 23, 4, "FW");
        set_ata_string(&mut matching_data, 27, 20, "MODEL");
        let matching_identify = parse_identify_device(&matching_data).unwrap();
        let overflow_rules = [VendorAttributeRule {
            decoder: RawValueDecoder::Byte(0),
            multiplier: u64::MAX,
            ..invalid_rules[0]
        }];
        let overflow_schema = VendorSchema {
            rules: &overflow_rules,
            ..schema
        };
        let attributes = [SmartAttribute {
            id: 1,
            flags: 0,
            current: None,
            worst: None,
            raw: [2, 0, 0, 0, 0, 0],
            threshold: None,
        }];
        let overflow_schemas = [overflow_schema];
        assert_eq!(
            interpret_vendor_attributes(&overflow_schemas, &matching_identify, &attributes),
            Err(VendorSchemaError::ValueOverflow(1))
        );
    }

    #[test]
    fn routing_requires_explicit_transport_evidence_and_qualified_usb_sat() {
        let direct = AtaRoutingEvidence {
            direct_access_block_device: true,
            ..AtaRoutingEvidence::default()
        };
        assert_eq!(
            route_ata_backend(direct),
            AtaBackendRoute::InsufficientEvidence
        );
        assert_eq!(
            route_ata_backend(AtaRoutingEvidence {
                libata_managed: true,
                ..direct
            }),
            AtaBackendRoute::NativeSat
        );
        assert_eq!(
            route_ata_backend(AtaRoutingEvidence {
                usb_transport: true,
                ..direct
            }),
            AtaBackendRoute::UnsupportedUsbBridge
        );
        assert_eq!(
            route_ata_backend(AtaRoutingEvidence {
                usb_transport: true,
                qualified_usb_sat: true,
                ..direct
            }),
            AtaBackendRoute::QualifiedUsbSat
        );
        assert_eq!(
            route_ata_backend(AtaRoutingEvidence {
                libata_managed: true,
                native_scsi_evidence: true,
                ..direct
            }),
            AtaBackendRoute::Ambiguous
        );
    }

    #[test]
    fn bad_checksum_and_invalid_normalized_values_are_explicit() {
        let mut data = [0u8; 512];
        data[2..14].copy_from_slice(&[9, 0, 0, 0xff, 0, 0, 0, 0, 0, 0, 0, 0]);
        checksum(&mut data);
        let attributes = parse_smart_attributes(&data).unwrap();
        assert_eq!(attributes[0].current, None);
        data[100] = 1;
        assert_eq!(
            parse_smart_attributes(&data),
            Err(AtaParseError::InvalidChecksum)
        );
    }

    #[test]
    fn parses_ata_return_descriptor_and_smart_status_signatures() {
        let sense = [
            0x72, 1, 0, 0, 0, 0, 0, 14, 0x09, 0x0c, 0, 0, 0, 1, 0, 0, 0, 0x4f, 0, 0xc2, 0xa0, 0x50,
        ];
        let registers = parse_ata_return_descriptor(&sense).unwrap();
        assert_eq!(registers.lba_mid, 0x004f);
        assert_eq!(smart_return_status(registers), SmartStatus::Passed);
        assert_eq!(
            smart_return_status(AtaReturnDescriptor {
                lba_mid: 0xf4,
                lba_high: 0x2c,
                ..registers
            }),
            SmartStatus::PredictingFailure
        );
    }

    #[test]
    fn truncated_pages_and_missing_descriptors_never_become_success() {
        assert!(matches!(
            parse_identify_device(&[0; 511]),
            Err(AtaParseError::WrongLength { .. })
        ));
        assert_eq!(
            parse_ata_return_descriptor(&[0x72, 0, 0, 0, 0, 0, 0, 0]),
            Err(AtaParseError::MissingAtaReturnDescriptor)
        );
    }
}
