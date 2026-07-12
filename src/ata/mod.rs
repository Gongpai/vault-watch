//! Pure ATA/SAT health protocol foundation.
//!
//! No device paths, ioctls, arbitrary taskfiles, data-out commands, self-test,
//! firmware, security, sanitize, or write operations are exposed here.

const ATA_PASS_THROUGH_16: u8 = 0x85;
const ATA_IDENTIFY_DEVICE: u8 = 0xec;
const ATA_SMART: u8 = 0xb0;
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
}

impl AtaReadCommand {
    pub const fn data_len(self) -> usize {
        match self {
            Self::SmartReturnStatus => 0,
            _ => 512,
        }
    }

    pub const fn cdb(self) -> [u8; 16] {
        let (protocol, transfer, feature, count, lba_mid, lba_high, command) = match self {
            Self::IdentifyDevice => (4, 0x2e, 0, 1, 0, 0, ATA_IDENTIFY_DEVICE),
            Self::SmartReadData => (
                4,
                0x2e,
                SMART_READ_DATA,
                1,
                SMART_LBA_MID,
                SMART_LBA_HIGH,
                ATA_SMART,
            ),
            Self::SmartReadThresholds => (
                4,
                0x2e,
                SMART_READ_THRESHOLDS,
                1,
                SMART_LBA_MID,
                SMART_LBA_HIGH,
                ATA_SMART,
            ),
            Self::SmartReturnStatus => (
                3,
                0x20,
                SMART_RETURN_STATUS,
                0,
                SMART_LBA_MID,
                SMART_LBA_HIGH,
                ATA_SMART,
            ),
        };
        [
            ATA_PASS_THROUGH_16,
            protocol << 1,
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
    pub medium: AtaMedium,
    pub smart_supported: bool,
}

pub fn parse_identify_device(data: &[u8]) -> Result<IdentifyDevice, AtaParseError> {
    let words = ata_words(data)?;
    let lba28_sectors = u32::from(words[60]) | (u32::from(words[61]) << 16);
    let lba48_valid = words[83] & (1 << 10) != 0 && words[83] & (1 << 14) != 0;
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
    Ok(IdentifyDevice {
        serial: ata_string(data, 10, 10),
        firmware: ata_string(data, 23, 4),
        model: ata_string(data, 27, 20),
        lba28_sectors,
        lba48_sectors,
        medium,
        smart_supported: words[82] & 1 != 0,
    })
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
        set_word(&mut data, 100, 1);
        set_word(&mut data, 217, 1);
        let identify = parse_identify_device(&data).unwrap();
        assert_eq!(identify.model, "FIXTURE MODEL");
        assert_eq!(identify.lba28_sectors, 0x1234_5678);
        assert_eq!(identify.lba48_sectors, Some(1));
        assert_eq!(identify.medium, AtaMedium::SolidState);
        assert!(identify.smart_supported);
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
