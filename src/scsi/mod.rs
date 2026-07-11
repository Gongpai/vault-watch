//! Pure, read-only SCSI protocol boundary.
//!
//! This module intentionally contains no file descriptors, ioctls, device
//! paths, or arbitrary CDB API. Runtime SG_IO integration remains gated on the
//! privileged broker story; these types only build an audited command subset
//! and parse bounded response fixtures.

const INQUIRY: u8 = 0x12;
const LOG_SENSE_10: u8 = 0x4d;
const TEST_UNIT_READY: u8 = 0x00;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataDirection {
    None,
    FromDevice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VpdPage {
    Supported = 0x00,
    UnitSerial = 0x80,
    DeviceIdentification = 0x83,
    AtaInformation = 0x89,
    BlockDeviceCharacteristics = 0xb1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogPage {
    Supported = 0x00,
    WriteErrors = 0x02,
    ReadErrors = 0x03,
    Temperature = 0x0d,
    NonMediumErrors = 0x06,
    InformationalExceptions = 0x2f,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOnlyCommand {
    TestUnitReady,
    Inquiry { allocation_len: u8 },
    InquiryVpd { page: VpdPage, allocation_len: u8 },
    LogSense { page: LogPage, allocation_len: u16 },
}

impl ReadOnlyCommand {
    pub const fn direction(self) -> DataDirection {
        match self {
            Self::TestUnitReady => DataDirection::None,
            Self::Inquiry { .. } | Self::InquiryVpd { .. } | Self::LogSense { .. } => {
                DataDirection::FromDevice
            }
        }
    }

    pub fn cdb(self) -> Vec<u8> {
        match self {
            Self::TestUnitReady => vec![TEST_UNIT_READY, 0, 0, 0, 0, 0],
            Self::Inquiry { allocation_len } => vec![INQUIRY, 0, 0, 0, allocation_len, 0],
            Self::InquiryVpd {
                page,
                allocation_len,
            } => vec![INQUIRY, 1, page as u8, 0, allocation_len, 0],
            Self::LogSense {
                page,
                allocation_len,
            } => {
                let [hi, lo] = allocation_len.to_be_bytes();
                vec![LOG_SENSE_10, 0, page as u8, 0, 0, 0, 0, hi, lo, 0]
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Truncated { needed: usize, actual: usize },
    UnexpectedPage { expected: u8, actual: u8 },
    DeclaredLengthExceedsPayload { declared: usize, actual: usize },
    InvalidPeripheralQualifier(u8),
    TruncatedParameter { offset: usize, declared: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardInquiry {
    pub peripheral_device_type: u8,
    pub removable: bool,
    pub version: u8,
}

pub fn parse_standard_inquiry(data: &[u8]) -> Result<StandardInquiry, ParseError> {
    require_len(data, 5)?;
    let qualifier = data[0] >> 5;
    if qualifier > 3 {
        return Err(ParseError::InvalidPeripheralQualifier(qualifier));
    }
    let total = 5usize.saturating_add(data[4] as usize);
    if total > data.len() {
        return Err(ParseError::DeclaredLengthExceedsPayload {
            declared: total,
            actual: data.len(),
        });
    }
    Ok(StandardInquiry {
        peripheral_device_type: data[0] & 0x1f,
        removable: data[1] & 0x80 != 0,
        version: data[2],
    })
}

pub fn parse_supported_vpd_pages(data: &[u8]) -> Result<Vec<u8>, ParseError> {
    require_len(data, 4)?;
    if data[1] != VpdPage::Supported as u8 {
        return Err(ParseError::UnexpectedPage {
            expected: VpdPage::Supported as u8,
            actual: data[1],
        });
    }
    let declared = u16::from_be_bytes([data[2], data[3]]) as usize;
    let end = 4usize.saturating_add(declared);
    if end > data.len() {
        return Err(ParseError::DeclaredLengthExceedsPayload {
            declared: end,
            actual: data.len(),
        });
    }
    Ok(data[4..end].to_vec())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Temperature {
    pub current_c: Option<u8>,
    pub reference_c: Option<u8>,
}

pub fn parse_temperature_log(data: &[u8]) -> Result<Temperature, ParseError> {
    let parameters = parse_log_parameters(data, LogPage::Temperature as u8)?;
    let mut temperature = Temperature {
        current_c: None,
        reference_c: None,
    };
    for parameter in parameters {
        let value = parameter
            .value
            .get(1)
            .copied()
            .filter(|value| *value != 0xff);
        match parameter.code {
            0x0000 => temperature.current_c = value,
            0x0001 => temperature.reference_c = value,
            _ => {}
        }
    }
    Ok(temperature)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LogParameter<'a> {
    code: u16,
    value: &'a [u8],
}

fn parse_log_parameters(
    data: &[u8],
    expected_page: u8,
) -> Result<Vec<LogParameter<'_>>, ParseError> {
    require_len(data, 4)?;
    let actual_page = data[0] & 0x3f;
    if actual_page != expected_page {
        return Err(ParseError::UnexpectedPage {
            expected: expected_page,
            actual: actual_page,
        });
    }
    let declared = u16::from_be_bytes([data[2], data[3]]) as usize;
    let end = 4usize.saturating_add(declared);
    if end > data.len() {
        return Err(ParseError::DeclaredLengthExceedsPayload {
            declared: end,
            actual: data.len(),
        });
    }

    let mut parameters = Vec::new();
    let mut offset = 4;
    while offset < end {
        if offset.saturating_add(4) > end {
            return Err(ParseError::TruncatedParameter {
                offset,
                declared: end - offset,
            });
        }
        let length = data[offset + 3] as usize;
        let value_start = offset + 4;
        let next = value_start.saturating_add(length);
        if next > end {
            return Err(ParseError::TruncatedParameter {
                offset,
                declared: length,
            });
        }
        parameters.push(LogParameter {
            code: u16::from_be_bytes([data[offset], data[offset + 1]]),
            value: &data[value_start..next],
        });
        offset = next;
    }
    Ok(parameters)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenseFormat {
    Fixed,
    Descriptor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SenseData {
    pub format: SenseFormat,
    pub sense_key: u8,
    pub asc: u8,
    pub ascq: u8,
}

pub fn parse_sense(data: &[u8]) -> Result<SenseData, ParseError> {
    require_len(data, 1)?;
    match data[0] & 0x7f {
        0x70 | 0x71 => {
            require_len(data, 14)?;
            Ok(SenseData {
                format: SenseFormat::Fixed,
                sense_key: data[2] & 0x0f,
                asc: data[12],
                ascq: data[13],
            })
        }
        0x72 | 0x73 => {
            require_len(data, 4)?;
            Ok(SenseData {
                format: SenseFormat::Descriptor,
                sense_key: data[1] & 0x0f,
                asc: data[2],
                ascq: data[3],
            })
        }
        code => Err(ParseError::UnexpectedPage {
            expected: 0x70,
            actual: code,
        }),
    }
}

fn require_len(data: &[u8], needed: usize) -> Result<(), ParseError> {
    if data.len() < needed {
        Err(ParseError::Truncated {
            needed,
            actual: data.len(),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_surface_contains_only_none_or_data_in_operations() {
        let commands = [
            ReadOnlyCommand::TestUnitReady,
            ReadOnlyCommand::Inquiry { allocation_len: 96 },
            ReadOnlyCommand::InquiryVpd {
                page: VpdPage::DeviceIdentification,
                allocation_len: 255,
            },
            ReadOnlyCommand::LogSense {
                page: LogPage::Temperature,
                allocation_len: 512,
            },
        ];
        assert!(commands.iter().all(|command| matches!(
            command.direction(),
            DataDirection::None | DataDirection::FromDevice
        )));
        assert_eq!(commands[0].cdb()[0], TEST_UNIT_READY);
        assert_eq!(commands[1].cdb()[0], INQUIRY);
        assert_eq!(commands[3].cdb(), [0x4d, 0, 0x0d, 0, 0, 0, 0, 2, 0, 0]);
    }

    #[test]
    fn parses_standard_inquiry_without_retaining_identity_text() {
        let fixture = [0x00, 0x80, 0x06, 0x02, 0x03, 0, 0, 0];
        assert_eq!(
            parse_standard_inquiry(&fixture),
            Ok(StandardInquiry {
                peripheral_device_type: 0,
                removable: true,
                version: 6,
            })
        );
    }

    #[test]
    fn parses_supported_vpd_pages_and_rejects_truncation() {
        let fixture = [0, 0, 0, 4, 0x80, 0x83, 0x89, 0xb1];
        assert_eq!(
            parse_supported_vpd_pages(&fixture),
            Ok(vec![0x80, 0x83, 0x89, 0xb1])
        );
        assert!(matches!(
            parse_supported_vpd_pages(&fixture[..6]),
            Err(ParseError::DeclaredLengthExceedsPayload { .. })
        ));
    }

    #[test]
    fn temperature_sentinel_is_unavailable_not_zero() {
        let fixture = [0x0d, 0, 0, 12, 0, 0, 0, 2, 0, 42, 0, 1, 0, 2, 0, 0xff];
        assert_eq!(
            parse_temperature_log(&fixture),
            Ok(Temperature {
                current_c: Some(42),
                reference_c: None,
            })
        );
    }

    #[test]
    fn malformed_log_parameter_is_not_silently_dropped() {
        let fixture = [0x0d, 0, 0, 5, 0, 0, 0, 2, 42];
        assert!(matches!(
            parse_temperature_log(&fixture),
            Err(ParseError::TruncatedParameter { .. })
        ));
    }

    #[test]
    fn decodes_fixed_descriptor_and_short_sense() {
        let mut fixed = [0u8; 14];
        fixed[0] = 0x70;
        fixed[2] = 0x05;
        fixed[12] = 0x24;
        assert_eq!(
            parse_sense(&fixed),
            Ok(SenseData {
                format: SenseFormat::Fixed,
                sense_key: 5,
                asc: 0x24,
                ascq: 0,
            })
        );
        assert_eq!(
            parse_sense(&[0x72, 0x06, 0x29, 0]),
            Ok(SenseData {
                format: SenseFormat::Descriptor,
                sense_key: 6,
                asc: 0x29,
                ascq: 0,
            })
        );
        assert!(matches!(
            parse_sense(&[0x70, 0, 5]),
            Err(ParseError::Truncated { .. })
        ));
    }
}
