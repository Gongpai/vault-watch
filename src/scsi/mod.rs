//! Pure, read-only SCSI protocol boundary.
//!
//! This module intentionally contains no file descriptors, ioctls, device
//! paths, or arbitrary CDB API. Runtime SG_IO integration remains gated on the
//! privileged broker story; these types only build an audited command subset
//! and parse bounded response fixtures.

pub mod mapping;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposedDirection {
    None,
    FromDevice,
    ToDevice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPolicyRejection {
    RawCdb,
    DataOut,
    VendorCommand,
}

/// Broker-boundary guard for requests that did not originate from the typed
/// command API. It never converts an opcode into an executable command.
pub const fn reject_untyped_command(
    opcode: u8,
    direction: ProposedDirection,
) -> CommandPolicyRejection {
    if matches!(direction, ProposedDirection::ToDevice) {
        CommandPolicyRejection::DataOut
    } else if opcode >= 0xc0 {
        CommandPolicyRejection::VendorCommand
    } else {
        CommandPolicyRejection::RawCdb
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendRoute {
    NativeScsi,
    Sat,
    ControllerHidden,
    AmbiguousScsiMapping,
    UnsupportedPeripheral(u8),
    NoScsiGeneric,
    InsufficientEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingEvidence {
    pub scsi_generic_count: usize,
    pub inquiry: Option<StandardInquiry>,
    pub supported_vpd_pages: Vec<u8>,
    pub transport_available: bool,
    pub controller_logical_volume: bool,
}

pub fn route_backend(evidence: &RoutingEvidence) -> BackendRoute {
    if evidence.controller_logical_volume && !evidence.transport_available {
        return BackendRoute::ControllerHidden;
    }
    match evidence.scsi_generic_count {
        0 => return BackendRoute::NoScsiGeneric,
        1 => {}
        _ => return BackendRoute::AmbiguousScsiMapping,
    }
    if !evidence.transport_available {
        return BackendRoute::ControllerHidden;
    }
    let Some(inquiry) = &evidence.inquiry else {
        return BackendRoute::InsufficientEvidence;
    };
    if inquiry.peripheral_device_type != 0 {
        return BackendRoute::UnsupportedPeripheral(inquiry.peripheral_device_type);
    }
    if evidence
        .supported_vpd_pages
        .contains(&(VpdPage::AtaInformation as u8))
    {
        BackendRoute::Sat
    } else {
        BackendRoute::NativeScsi
    }
}

pub fn route_discovered_mapping(
    mapping: &mapping::ScsiGenericMapping,
    inquiry: Option<StandardInquiry>,
    supported_vpd_pages: Vec<u8>,
    controller_logical_volume: bool,
) -> BackendRoute {
    use mapping::MappingAvailability;

    match mapping.availability {
        MappingAvailability::DeviceGone | MappingAvailability::Unreadable => {
            return BackendRoute::InsufficientEvidence;
        }
        MappingAvailability::NoScsiGenericInterface if controller_logical_volume => {
            return BackendRoute::ControllerHidden;
        }
        MappingAvailability::NoScsiGenericInterface => return BackendRoute::NoScsiGeneric,
        MappingAvailability::Complete => {}
    }
    if mapping.rejected_entries > 0 {
        return BackendRoute::AmbiguousScsiMapping;
    }
    route_backend(&RoutingEvidence {
        scsi_generic_count: mapping.entries.len(),
        inquiry,
        supported_vpd_pages,
        transport_available: true,
        controller_logical_volume,
    })
}

pub fn discovered_vpd_command(
    page: VpdPage,
    supported_pages: &[u8],
    allocation_len: u8,
) -> Option<ReadOnlyCommand> {
    supported_pages
        .contains(&(page as u8))
        .then_some(ReadOnlyCommand::InquiryVpd {
            page,
            allocation_len,
        })
}

pub fn discovered_log_command(
    page: LogPage,
    supported_pages: &[u8],
    allocation_len: u16,
) -> Option<ReadOnlyCommand> {
    supported_pages
        .contains(&(page as u8))
        .then_some(ReadOnlyCommand::LogSense {
            page,
            allocation_len,
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportCompletion {
    pub ioctl_succeeded: bool,
    pub scsi_status: u8,
    pub host_status: u16,
    pub driver_status: u16,
    pub residual: i32,
    pub requested_len: usize,
    pub sense_written: usize,
    pub sense_capacity: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionOutcome {
    DataIn { payload_len: usize },
    CheckCondition { sense_len: usize },
    Busy,
    ReservationConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionError {
    IoctlFailed,
    HostStatus(u16),
    DriverStatus(u16),
    InvalidResidual(i32),
    InvalidSenseLength { written: usize, capacity: usize },
    UnexpectedScsiStatus(u8),
}

pub fn validate_completion(
    completion: TransportCompletion,
) -> Result<CompletionOutcome, CompletionError> {
    if !completion.ioctl_succeeded {
        return Err(CompletionError::IoctlFailed);
    }
    if completion.host_status != 0 {
        return Err(CompletionError::HostStatus(completion.host_status));
    }
    if completion.driver_status != 0 {
        return Err(CompletionError::DriverStatus(completion.driver_status));
    }
    if completion.residual < 0 || completion.residual as usize > completion.requested_len {
        return Err(CompletionError::InvalidResidual(completion.residual));
    }
    if completion.sense_written > completion.sense_capacity {
        return Err(CompletionError::InvalidSenseLength {
            written: completion.sense_written,
            capacity: completion.sense_capacity,
        });
    }
    match completion.scsi_status {
        0x00 => Ok(CompletionOutcome::DataIn {
            payload_len: completion.requested_len - completion.residual as usize,
        }),
        0x02 => Ok(CompletionOutcome::CheckCondition {
            sense_len: completion.sense_written,
        }),
        0x08 => Ok(CompletionOutcome::Busy),
        0x18 => Ok(CompletionOutcome::ReservationConflict),
        status => Err(CompletionError::UnexpectedScsiStatus(status)),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Truncated { needed: usize, actual: usize },
    UnexpectedPage { expected: u8, actual: u8 },
    DeclaredLengthExceedsPayload { declared: usize, actual: usize },
    InvalidPeripheralQualifier(u8),
    TruncatedParameter { offset: usize, declared: usize },
    InvalidParameterWidth { code: u16, width: usize },
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdDescriptor {
    pub protocol_identifier: u8,
    pub code_set: u8,
    pub association: u8,
    pub designator_type: u8,
    pub value: Vec<u8>,
}

pub fn parse_device_identification_vpd(data: &[u8]) -> Result<Vec<DeviceIdDescriptor>, ParseError> {
    let payload = vpd_payload(data, VpdPage::DeviceIdentification)?;
    let mut descriptors = Vec::new();
    let mut offset = 0;
    while offset < payload.len() {
        if offset.saturating_add(4) > payload.len() {
            return Err(ParseError::TruncatedParameter {
                offset: offset + 4,
                declared: payload.len() - offset,
            });
        }
        let length = payload[offset + 3] as usize;
        let value_start = offset + 4;
        let next = value_start.saturating_add(length);
        if next > payload.len() {
            return Err(ParseError::TruncatedParameter {
                offset: offset + 4,
                declared: length,
            });
        }
        descriptors.push(DeviceIdDescriptor {
            protocol_identifier: payload[offset] >> 4,
            code_set: payload[offset] & 0x0f,
            association: (payload[offset + 1] >> 4) & 0x03,
            designator_type: payload[offset + 1] & 0x0f,
            value: payload[value_start..next].to_vec(),
        });
        offset = next;
    }
    Ok(descriptors)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediumRotation {
    Unknown,
    SolidState,
    RotationalRpm(u16),
}

pub fn parse_block_device_characteristics_vpd(data: &[u8]) -> Result<MediumRotation, ParseError> {
    let payload = vpd_payload(data, VpdPage::BlockDeviceCharacteristics)?;
    require_len(payload, 2)?;
    Ok(match u16::from_be_bytes([payload[0], payload[1]]) {
        0x0001 => MediumRotation::SolidState,
        rpm @ 0x0401..=0xfffe => MediumRotation::RotationalRpm(rpm),
        _ => MediumRotation::Unknown,
    })
}

fn vpd_payload(data: &[u8], expected_page: VpdPage) -> Result<&[u8], ParseError> {
    require_len(data, 4)?;
    if data[1] != expected_page as u8 {
        return Err(ParseError::UnexpectedPage {
            expected: expected_page as u8,
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
    Ok(&data[4..end])
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

pub fn parse_supported_log_pages(data: &[u8]) -> Result<Vec<u8>, ParseError> {
    require_len(data, 4)?;
    let actual_page = data[0] & 0x3f;
    if actual_page != LogPage::Supported as u8 {
        return Err(ParseError::UnexpectedPage {
            expected: LogPage::Supported as u8,
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
    Ok(data[4..end].iter().map(|page| page & 0x3f).collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ErrorCounters {
    pub total_rewrites_or_rereads: Option<u64>,
    pub total_errors_corrected: Option<u64>,
    pub correction_algorithm_invocations: Option<u64>,
    pub total_bytes_processed: Option<u64>,
    pub total_uncorrected_errors: Option<u64>,
}

pub fn parse_error_counter_log(data: &[u8], page: LogPage) -> Result<ErrorCounters, ParseError> {
    if !matches!(page, LogPage::ReadErrors | LogPage::WriteErrors) {
        return Err(ParseError::UnexpectedPage {
            expected: LogPage::ReadErrors as u8,
            actual: page as u8,
        });
    }
    let parameters = parse_log_parameters(data, page as u8)?;
    let mut counters = ErrorCounters::default();
    for parameter in parameters {
        let value = parameter_u64(&parameter)?;
        match parameter.code {
            0x0002 => counters.total_rewrites_or_rereads = Some(value),
            0x0003 => counters.total_errors_corrected = Some(value),
            0x0004 => counters.correction_algorithm_invocations = Some(value),
            0x0005 => counters.total_bytes_processed = Some(value),
            0x0006 => counters.total_uncorrected_errors = Some(value),
            _ => {}
        }
    }
    Ok(counters)
}

pub fn parse_non_medium_error_log(data: &[u8]) -> Result<Option<u64>, ParseError> {
    let parameters = parse_log_parameters(data, LogPage::NonMediumErrors as u8)?;
    parameters
        .iter()
        .find(|parameter| parameter.code == 0)
        .map(parameter_u64)
        .transpose()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InformationalException {
    pub asc: u8,
    pub ascq: u8,
    pub temperature_c: Option<u8>,
}

impl InformationalException {
    pub const fn failure_predicted(self) -> bool {
        self.asc == 0x5d
    }
}

pub fn parse_informational_exception_log(
    data: &[u8],
) -> Result<Option<InformationalException>, ParseError> {
    let parameters = parse_log_parameters(data, LogPage::InformationalExceptions as u8)?;
    let Some(parameter) = parameters.iter().find(|parameter| parameter.code == 0) else {
        return Ok(None);
    };
    if parameter.value.len() < 3 {
        return Err(ParseError::TruncatedParameter {
            offset: 4,
            declared: parameter.value.len(),
        });
    }
    Ok(Some(InformationalException {
        asc: parameter.value[0],
        ascq: parameter.value[1],
        temperature_c: (parameter.value[2] != 0xff).then_some(parameter.value[2]),
    }))
}

fn parameter_u64(parameter: &LogParameter<'_>) -> Result<u64, ParseError> {
    match parameter.value {
        [value] => Ok(u64::from(*value)),
        [a, b] => Ok(u64::from(u16::from_be_bytes([*a, *b]))),
        [a, b, c, d] => Ok(u64::from(u32::from_be_bytes([*a, *b, *c, *d]))),
        [a, b, c, d, e, f, g, h] => Ok(u64::from_be_bytes([*a, *b, *c, *d, *e, *f, *g, *h])),
        value => Err(ParseError::InvalidParameterWidth {
            code: parameter.code,
            width: value.len(),
        }),
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenseAction {
    Unsupported,
    RetryOnceRefreshCapabilities,
    RetryOnce,
    TemporarilyUnavailable,
    MediaFailure,
    HardwareFailure,
    Report,
}

pub const fn sense_action(
    sense: SenseData,
    optional_command: bool,
    retry_attempted: bool,
) -> SenseAction {
    match (sense.sense_key, sense.asc, sense.ascq) {
        (0x05, 0x20 | 0x24, 0x00) if optional_command => SenseAction::Unsupported,
        (0x06, _, _) if !retry_attempted => SenseAction::RetryOnceRefreshCapabilities,
        (0x0b, _, _) if !retry_attempted => SenseAction::RetryOnce,
        (0x02, _, _) => SenseAction::TemporarilyUnavailable,
        (0x03, _, _) => SenseAction::MediaFailure,
        (0x04, _, _) => SenseAction::HardwareFailure,
        _ => SenseAction::Report,
    }
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
    fn untyped_data_out_vendor_and_other_raw_commands_are_rejected() {
        assert_eq!(
            reject_untyped_command(0x2a, ProposedDirection::ToDevice),
            CommandPolicyRejection::DataOut
        );
        assert_eq!(
            reject_untyped_command(0xc1, ProposedDirection::FromDevice),
            CommandPolicyRejection::VendorCommand
        );
        assert_eq!(
            reject_untyped_command(0x12, ProposedDirection::FromDevice),
            CommandPolicyRejection::RawCdb
        );
        assert_eq!(
            reject_untyped_command(0x00, ProposedDirection::None),
            CommandPolicyRejection::RawCdb
        );
    }

    #[test]
    fn capability_commands_require_advertised_pages() {
        let vpd = [0x83, 0xb1];
        assert!(discovered_vpd_command(VpdPage::DeviceIdentification, &vpd, 255).is_some());
        assert!(discovered_vpd_command(VpdPage::AtaInformation, &vpd, 255).is_none());
        let logs = [0x03, 0x0d];
        assert!(discovered_log_command(LogPage::Temperature, &logs, 512).is_some());
        assert!(discovered_log_command(LogPage::InformationalExceptions, &logs, 512).is_none());
    }

    #[test]
    fn routing_distinguishes_native_sat_hidden_ambiguous_and_non_disk() {
        let direct_disk = StandardInquiry {
            peripheral_device_type: 0,
            removable: false,
            version: 6,
        };
        let evidence = |count, pages: Vec<u8>| RoutingEvidence {
            scsi_generic_count: count,
            inquiry: Some(direct_disk.clone()),
            supported_vpd_pages: pages,
            transport_available: true,
            controller_logical_volume: false,
        };
        assert_eq!(
            route_backend(&evidence(1, vec![0x83])),
            BackendRoute::NativeScsi
        );
        assert_eq!(
            route_backend(&evidence(1, vec![0x83, 0x89])),
            BackendRoute::Sat
        );
        assert_eq!(
            route_backend(&evidence(2, vec![0x83])),
            BackendRoute::AmbiguousScsiMapping
        );

        let mut hidden = evidence(0, vec![]);
        hidden.controller_logical_volume = true;
        hidden.transport_available = false;
        assert_eq!(route_backend(&hidden), BackendRoute::ControllerHidden);

        let mut tape = evidence(1, vec![]);
        tape.inquiry.as_mut().unwrap().peripheral_device_type = 1;
        assert_eq!(route_backend(&tape), BackendRoute::UnsupportedPeripheral(1));
    }

    #[test]
    fn discovered_mapping_preserves_missing_partial_and_controller_hidden_states() {
        use mapping::{MappingAvailability, ScsiGenericMapping};

        let inquiry = Some(StandardInquiry {
            peripheral_device_type: 0,
            removable: false,
            version: 6,
        });
        let mapping = |availability, entries: &[&str], rejected_entries| ScsiGenericMapping {
            entries: entries.iter().map(|entry| (*entry).to_owned()).collect(),
            rejected_entries,
            availability,
        };
        assert_eq!(
            route_discovered_mapping(
                &mapping(MappingAvailability::Complete, &["sg0"], 0),
                inquiry.clone(),
                vec![0x83],
                false,
            ),
            BackendRoute::NativeScsi
        );
        assert_eq!(
            route_discovered_mapping(
                &mapping(MappingAvailability::Complete, &["sg0"], 1),
                inquiry.clone(),
                vec![],
                false,
            ),
            BackendRoute::AmbiguousScsiMapping
        );
        assert_eq!(
            route_discovered_mapping(
                &mapping(MappingAvailability::NoScsiGenericInterface, &[], 0),
                inquiry.clone(),
                vec![],
                true,
            ),
            BackendRoute::ControllerHidden
        );
        assert_eq!(
            route_discovered_mapping(
                &mapping(MappingAvailability::DeviceGone, &[], 0),
                inquiry,
                vec![],
                false,
            ),
            BackendRoute::InsufficientEvidence
        );
    }

    #[test]
    fn completion_validation_bounds_payload_sense_and_transport_status() {
        let completion = TransportCompletion {
            ioctl_succeeded: true,
            scsi_status: 0,
            host_status: 0,
            driver_status: 0,
            residual: 24,
            requested_len: 64,
            sense_written: 0,
            sense_capacity: 32,
        };
        assert_eq!(
            validate_completion(completion),
            Ok(CompletionOutcome::DataIn { payload_len: 40 })
        );
        assert_eq!(
            validate_completion(TransportCompletion {
                residual: -1,
                ..completion
            }),
            Err(CompletionError::InvalidResidual(-1))
        );
        assert_eq!(
            validate_completion(TransportCompletion {
                scsi_status: 2,
                residual: 0,
                sense_written: 18,
                ..completion
            }),
            Ok(CompletionOutcome::CheckCondition { sense_len: 18 })
        );
        assert!(matches!(
            validate_completion(TransportCompletion {
                host_status: 1,
                ..completion
            }),
            Err(CompletionError::HostStatus(1))
        ));
        assert!(matches!(
            validate_completion(TransportCompletion {
                sense_written: 33,
                ..completion
            }),
            Err(CompletionError::InvalidSenseLength { .. })
        ));
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
    fn parses_vpd_identifiers_by_scope_without_text_assumptions() {
        let fixture = [
            0, 0x83, 0, 12, 0x61, 0x03, 0, 4, 0xde, 0xad, 0xbe, 0xef, 0x12, 0x18, 0, 0,
        ];
        let descriptors = parse_device_identification_vpd(&fixture).unwrap();
        assert_eq!(descriptors.len(), 2);
        assert_eq!(descriptors[0].protocol_identifier, 6);
        assert_eq!(descriptors[0].code_set, 1);
        assert_eq!(descriptors[0].association, 0);
        assert_eq!(descriptors[0].designator_type, 3);
        assert_eq!(descriptors[0].value, [0xde, 0xad, 0xbe, 0xef]);
        assert!(descriptors[1].value.is_empty());
    }

    #[test]
    fn parses_rotation_and_supported_log_pages() {
        assert_eq!(
            parse_block_device_characteristics_vpd(&[0, 0xb1, 0, 2, 0, 1]),
            Ok(MediumRotation::SolidState)
        );
        assert_eq!(
            parse_block_device_characteristics_vpd(&[0, 0xb1, 0, 2, 0x1c, 0x20]),
            Ok(MediumRotation::RotationalRpm(7200))
        );
        assert_eq!(
            parse_supported_log_pages(&[0, 0, 0, 4, 0x02, 0x03, 0x0d, 0x2f]),
            Ok(vec![0x02, 0x03, 0x0d, 0x2f])
        );
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
    fn parses_error_counters_and_rejects_nonstandard_integer_width() {
        let fixture = [
            0x03, 0, 0, 16, 0, 3, 0, 4, 0, 0, 0, 9, 0, 6, 0, 4, 0, 0, 0, 2,
        ];
        assert_eq!(
            parse_error_counter_log(&fixture, LogPage::ReadErrors),
            Ok(ErrorCounters {
                total_errors_corrected: Some(9),
                total_uncorrected_errors: Some(2),
                ..ErrorCounters::default()
            })
        );
        let invalid_width = [0x06, 0, 0, 7, 0, 0, 0, 3, 1, 2, 3];
        assert!(matches!(
            parse_non_medium_error_log(&invalid_width),
            Err(ParseError::InvalidParameterWidth { width: 3, .. })
        ));
    }

    #[test]
    fn informational_exception_is_explicit_and_temperature_can_be_unavailable() {
        let fixture = [0x2f, 0, 0, 7, 0, 0, 0, 3, 0x5d, 0x01, 0xff];
        let exception = parse_informational_exception_log(&fixture)
            .unwrap()
            .unwrap();
        assert!(exception.failure_predicted());
        assert_eq!(exception.temperature_c, None);
        assert_eq!(
            parse_informational_exception_log(&[0x2f, 0, 0, 0]),
            Ok(None)
        );
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

    #[test]
    fn sense_policy_is_bounded_and_does_not_hide_media_failure() {
        let sense = |sense_key, asc| SenseData {
            format: SenseFormat::Descriptor,
            sense_key,
            asc,
            ascq: 0,
        };
        assert_eq!(
            sense_action(sense(5, 0x24), true, false),
            SenseAction::Unsupported
        );
        assert_eq!(
            sense_action(sense(6, 0x29), false, false),
            SenseAction::RetryOnceRefreshCapabilities
        );
        assert_eq!(
            sense_action(sense(6, 0x29), false, true),
            SenseAction::Report
        );
        assert_eq!(
            sense_action(sense(3, 0x11), false, false),
            SenseAction::MediaFailure
        );
        assert_eq!(
            sense_action(sense(4, 0x44), false, false),
            SenseAction::HardwareFailure
        );
    }

    #[test]
    fn every_truncated_prefix_fails_without_panicking() {
        let vpd = [0, 0x83, 0, 8, 0x61, 0x03, 0, 4, 1, 2, 3, 4];
        for end in 0..vpd.len() {
            assert!(parse_device_identification_vpd(&vpd[..end]).is_err());
        }
        let log = [0x03, 0, 0, 8, 0, 6, 0, 4, 0, 0, 0, 1];
        for end in 0..log.len() {
            assert!(parse_error_counter_log(&log[..end], LogPage::ReadErrors).is_err());
        }
    }
}
