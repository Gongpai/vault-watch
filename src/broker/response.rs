use crate::ata::{AtaLogDirectory, AtaLogPageSupport, AtaMedium, SmartAttribute, SmartStatus};

use super::{AtaBrokerOperation, BrokerAtaResponse};

const MAGIC: [u8; 4] = *b"VWR1";
pub const BROKER_RESPONSE_WIRE_VERSION: u8 = 1;
const HEADER_LEN: usize = 20;
const MAX_SMART_ATTRIBUTES: usize = 64;
const MAX_GPL_PAGES: usize = 255;
const MAX_PAYLOAD_LEN: usize = 2 + MAX_SMART_ATTRIBUTES * 15;
const MAX_FRAME_LEN: usize = HEADER_LEN + MAX_PAYLOAD_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerResponseError {
    InvalidRequest,
    AuthorizationDenied,
    DeviceUnavailable,
    ExecutionFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerResponseFrame {
    pub request_id: u64,
    pub operation: AtaBrokerOperation,
    pub result: Result<BrokerAtaResponse, BrokerResponseError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerResponseWireError {
    FrameTooShort,
    FrameTooLarge,
    InvalidMagic,
    UnsupportedVersion,
    NonZeroReserved,
    UnknownOperation,
    UnknownStatus,
    InvalidRequestId,
    PayloadTooLarge,
    TrailingOrMissingBytes,
    UnexpectedErrorPayload,
    OperationPayloadMismatch,
    MalformedPayload,
}

pub fn encode_response_frame(
    response: &BrokerResponseFrame,
) -> Result<Vec<u8>, BrokerResponseWireError> {
    if response.request_id == 0 {
        return Err(BrokerResponseWireError::InvalidRequestId);
    }
    let payload = match &response.result {
        Ok(value) => encode_payload(response.operation, value)?,
        Err(_) => Vec::new(),
    };
    if payload.len() > MAX_PAYLOAD_LEN {
        return Err(BrokerResponseWireError::PayloadTooLarge);
    }
    let payload_len =
        u32::try_from(payload.len()).map_err(|_| BrokerResponseWireError::PayloadTooLarge)?;
    let mut frame = Vec::with_capacity(HEADER_LEN + payload.len());
    frame.extend_from_slice(&MAGIC);
    frame.push(BROKER_RESPONSE_WIRE_VERSION);
    frame.push(operation_code(response.operation));
    frame.push(status_code(response.result.as_ref().err().copied()));
    frame.push(0);
    frame.extend_from_slice(&response.request_id.to_le_bytes());
    frame.extend_from_slice(&payload_len.to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_response_frame(frame: &[u8]) -> Result<BrokerResponseFrame, BrokerResponseWireError> {
    if frame.len() < HEADER_LEN {
        return Err(BrokerResponseWireError::FrameTooShort);
    }
    if frame.len() > MAX_FRAME_LEN {
        return Err(BrokerResponseWireError::FrameTooLarge);
    }
    if frame[..4] != MAGIC {
        return Err(BrokerResponseWireError::InvalidMagic);
    }
    if frame[4] != BROKER_RESPONSE_WIRE_VERSION {
        return Err(BrokerResponseWireError::UnsupportedVersion);
    }
    if frame[7] != 0 {
        return Err(BrokerResponseWireError::NonZeroReserved);
    }
    let operation = decode_operation(frame[5]).ok_or(BrokerResponseWireError::UnknownOperation)?;
    let status = decode_status(frame[6]).ok_or(BrokerResponseWireError::UnknownStatus)?;
    let request_id = u64::from_le_bytes(frame[8..16].try_into().expect("bounded header"));
    if request_id == 0 {
        return Err(BrokerResponseWireError::InvalidRequestId);
    }
    let payload_len = usize::try_from(u32::from_le_bytes(
        frame[16..20].try_into().expect("bounded header"),
    ))
    .map_err(|_| BrokerResponseWireError::PayloadTooLarge)?;
    if payload_len > MAX_PAYLOAD_LEN {
        return Err(BrokerResponseWireError::PayloadTooLarge);
    }
    if frame.len() != HEADER_LEN + payload_len {
        return Err(BrokerResponseWireError::TrailingOrMissingBytes);
    }
    let payload = &frame[HEADER_LEN..];
    let result = match status {
        Some(error) => {
            if !payload.is_empty() {
                return Err(BrokerResponseWireError::UnexpectedErrorPayload);
            }
            Err(error)
        }
        None => Ok(decode_payload(operation, payload)?),
    };
    Ok(BrokerResponseFrame {
        request_id,
        operation,
        result,
    })
}

pub(super) const fn response_header_len() -> usize {
    HEADER_LEN
}

pub(super) fn response_frame_len(header: &[u8]) -> Result<usize, BrokerResponseWireError> {
    if header.len() != HEADER_LEN {
        return Err(BrokerResponseWireError::FrameTooShort);
    }
    let payload_len = usize::try_from(u32::from_le_bytes(
        header[16..20].try_into().expect("bounded header"),
    ))
    .map_err(|_| BrokerResponseWireError::PayloadTooLarge)?;
    if payload_len > MAX_PAYLOAD_LEN {
        return Err(BrokerResponseWireError::PayloadTooLarge);
    }
    Ok(HEADER_LEN + payload_len)
}

fn encode_payload(
    operation: AtaBrokerOperation,
    response: &BrokerAtaResponse,
) -> Result<Vec<u8>, BrokerResponseWireError> {
    let matches = matches!(
        (operation, response),
        (
            AtaBrokerOperation::IdentifyDevice,
            BrokerAtaResponse::Identify(_)
        ) | (
            AtaBrokerOperation::SmartReadData,
            BrokerAtaResponse::SmartData(_)
        ) | (
            AtaBrokerOperation::SmartReadThresholds,
            BrokerAtaResponse::SmartThresholds(_)
        ) | (
            AtaBrokerOperation::SmartReturnStatus,
            BrokerAtaResponse::SmartStatus(_)
        ) | (
            AtaBrokerOperation::ReadGplDirectory,
            BrokerAtaResponse::GplDirectory(_)
        )
    );
    if !matches {
        return Err(BrokerResponseWireError::OperationPayloadMismatch);
    }

    let mut payload = Vec::new();
    match response {
        BrokerAtaResponse::Identify(identify) => {
            payload.push(identify.capacity_bytes.is_some() as u8);
            payload.extend_from_slice(&identify.capacity_bytes.unwrap_or_default().to_le_bytes());
            match identify.medium {
                AtaMedium::Unknown => payload.extend_from_slice(&[0, 0, 0]),
                AtaMedium::SolidState => payload.extend_from_slice(&[1, 0, 0]),
                AtaMedium::RotationalRpm(rpm) => {
                    payload.push(2);
                    payload.extend_from_slice(&rpm.to_le_bytes());
                }
            }
            payload.push(
                identify.smart_supported as u8
                    | ((identify.general_purpose_logging_supported as u8) << 1),
            );
        }
        BrokerAtaResponse::SmartData(attributes) => {
            if attributes.len() > MAX_SMART_ATTRIBUTES {
                return Err(BrokerResponseWireError::PayloadTooLarge);
            }
            push_count(&mut payload, attributes.len())?;
            for attribute in attributes {
                payload.push(attribute.id);
                payload.extend_from_slice(&attribute.flags.to_le_bytes());
                push_optional_u8(&mut payload, attribute.current);
                push_optional_u8(&mut payload, attribute.worst);
                payload.extend_from_slice(&attribute.raw);
                push_optional_u8(&mut payload, attribute.threshold);
            }
        }
        BrokerAtaResponse::SmartThresholds(thresholds) => {
            if thresholds.len() > MAX_SMART_ATTRIBUTES {
                return Err(BrokerResponseWireError::PayloadTooLarge);
            }
            push_count(&mut payload, thresholds.len())?;
            for (id, threshold) in thresholds {
                payload.extend_from_slice(&[*id, *threshold]);
            }
        }
        BrokerAtaResponse::SmartStatus(status) => payload.push(match status {
            SmartStatus::Passed => 0,
            SmartStatus::PredictingFailure => 1,
            SmartStatus::Unknown => 2,
        }),
        BrokerAtaResponse::GplDirectory(directory) => {
            if directory.supported_pages.len() > MAX_GPL_PAGES {
                return Err(BrokerResponseWireError::PayloadTooLarge);
            }
            payload.extend_from_slice(&directory.version.to_le_bytes());
            push_count(&mut payload, directory.supported_pages.len())?;
            for page in &directory.supported_pages {
                payload.push(page.address);
                payload.extend_from_slice(&page.sectors.to_le_bytes());
            }
        }
    }
    Ok(payload)
}

fn decode_payload(
    operation: AtaBrokerOperation,
    payload: &[u8],
) -> Result<BrokerAtaResponse, BrokerResponseWireError> {
    match operation {
        AtaBrokerOperation::IdentifyDevice => decode_identify(payload),
        AtaBrokerOperation::SmartReadData => decode_smart_data(payload),
        AtaBrokerOperation::SmartReadThresholds => decode_thresholds(payload),
        AtaBrokerOperation::SmartReturnStatus => decode_smart_status(payload),
        AtaBrokerOperation::ReadGplDirectory => decode_gpl_directory(payload),
    }
}

fn decode_identify(payload: &[u8]) -> Result<BrokerAtaResponse, BrokerResponseWireError> {
    if payload.len() != 21 || payload[0] > 1 {
        return Err(BrokerResponseWireError::MalformedPayload);
    }
    let capacity = u128::from_le_bytes(
        payload[1..17]
            .try_into()
            .map_err(|_| BrokerResponseWireError::MalformedPayload)?,
    );
    let rpm = u16::from_le_bytes([payload[18], payload[19]]);
    let medium = match (payload[17], rpm) {
        (0, 0) => AtaMedium::Unknown,
        (1, 0) => AtaMedium::SolidState,
        (2, rpm) if rpm != 0 => AtaMedium::RotationalRpm(rpm),
        _ => return Err(BrokerResponseWireError::MalformedPayload),
    };
    if payload[20] & !0b11 != 0 {
        return Err(BrokerResponseWireError::MalformedPayload);
    }
    Ok(BrokerAtaResponse::Identify(super::AtaIdentifySummary {
        capacity_bytes: (payload[0] == 1).then_some(capacity),
        medium,
        smart_supported: payload[20] & 1 != 0,
        general_purpose_logging_supported: payload[20] & 2 != 0,
    }))
}

fn decode_smart_data(payload: &[u8]) -> Result<BrokerAtaResponse, BrokerResponseWireError> {
    let (count, mut offset) = read_count(payload)?;
    if count > MAX_SMART_ATTRIBUTES || payload.len() != offset + count * 15 {
        return Err(BrokerResponseWireError::MalformedPayload);
    }
    let mut attributes = Vec::with_capacity(count);
    for _ in 0..count {
        let id = payload[offset];
        let flags = u16::from_le_bytes([payload[offset + 1], payload[offset + 2]]);
        let current = read_optional_u8(payload[offset + 3], payload[offset + 4])?;
        let worst = read_optional_u8(payload[offset + 5], payload[offset + 6])?;
        let raw = payload[offset + 7..offset + 13]
            .try_into()
            .map_err(|_| BrokerResponseWireError::MalformedPayload)?;
        let threshold = read_optional_u8(payload[offset + 13], payload[offset + 14])?;
        attributes.push(SmartAttribute {
            id,
            flags,
            current,
            worst,
            raw,
            threshold,
        });
        offset += 15;
    }
    Ok(BrokerAtaResponse::SmartData(attributes))
}

fn decode_thresholds(payload: &[u8]) -> Result<BrokerAtaResponse, BrokerResponseWireError> {
    let (count, offset) = read_count(payload)?;
    if count > MAX_SMART_ATTRIBUTES || payload.len() != offset + count * 2 {
        return Err(BrokerResponseWireError::MalformedPayload);
    }
    Ok(BrokerAtaResponse::SmartThresholds(
        payload[offset..]
            .chunks_exact(2)
            .map(|entry| (entry[0], entry[1]))
            .collect(),
    ))
}

fn decode_smart_status(payload: &[u8]) -> Result<BrokerAtaResponse, BrokerResponseWireError> {
    let status = match payload {
        [0] => SmartStatus::Passed,
        [1] => SmartStatus::PredictingFailure,
        [2] => SmartStatus::Unknown,
        _ => return Err(BrokerResponseWireError::MalformedPayload),
    };
    Ok(BrokerAtaResponse::SmartStatus(status))
}

fn decode_gpl_directory(payload: &[u8]) -> Result<BrokerAtaResponse, BrokerResponseWireError> {
    if payload.len() < 4 {
        return Err(BrokerResponseWireError::MalformedPayload);
    }
    let version = u16::from_le_bytes([payload[0], payload[1]]);
    let count = usize::from(u16::from_le_bytes([payload[2], payload[3]]));
    if count > MAX_GPL_PAGES || payload.len() != 4 + count * 3 {
        return Err(BrokerResponseWireError::MalformedPayload);
    }
    let supported_pages = payload[4..]
        .chunks_exact(3)
        .map(|entry| AtaLogPageSupport {
            address: entry[0],
            sectors: u16::from_le_bytes([entry[1], entry[2]]),
        })
        .collect();
    Ok(BrokerAtaResponse::GplDirectory(AtaLogDirectory {
        version,
        supported_pages,
    }))
}

fn push_count(payload: &mut Vec<u8>, count: usize) -> Result<(), BrokerResponseWireError> {
    let count = u16::try_from(count).map_err(|_| BrokerResponseWireError::PayloadTooLarge)?;
    payload.extend_from_slice(&count.to_le_bytes());
    Ok(())
}

fn read_count(payload: &[u8]) -> Result<(usize, usize), BrokerResponseWireError> {
    if payload.len() < 2 {
        return Err(BrokerResponseWireError::MalformedPayload);
    }
    Ok((usize::from(u16::from_le_bytes([payload[0], payload[1]])), 2))
}

fn push_optional_u8(payload: &mut Vec<u8>, value: Option<u8>) {
    payload.extend_from_slice(&[value.is_some() as u8, value.unwrap_or_default()]);
}

fn read_optional_u8(tag: u8, value: u8) -> Result<Option<u8>, BrokerResponseWireError> {
    match (tag, value) {
        (0, 0) => Ok(None),
        (1, value) => Ok(Some(value)),
        _ => Err(BrokerResponseWireError::MalformedPayload),
    }
}

const fn operation_code(operation: AtaBrokerOperation) -> u8 {
    match operation {
        AtaBrokerOperation::IdentifyDevice => 1,
        AtaBrokerOperation::SmartReadData => 2,
        AtaBrokerOperation::SmartReadThresholds => 3,
        AtaBrokerOperation::SmartReturnStatus => 4,
        AtaBrokerOperation::ReadGplDirectory => 5,
    }
}

const fn decode_operation(code: u8) -> Option<AtaBrokerOperation> {
    match code {
        1 => Some(AtaBrokerOperation::IdentifyDevice),
        2 => Some(AtaBrokerOperation::SmartReadData),
        3 => Some(AtaBrokerOperation::SmartReadThresholds),
        4 => Some(AtaBrokerOperation::SmartReturnStatus),
        5 => Some(AtaBrokerOperation::ReadGplDirectory),
        _ => None,
    }
}

const fn status_code(error: Option<BrokerResponseError>) -> u8 {
    match error {
        None => 0,
        Some(BrokerResponseError::InvalidRequest) => 1,
        Some(BrokerResponseError::AuthorizationDenied) => 2,
        Some(BrokerResponseError::DeviceUnavailable) => 3,
        Some(BrokerResponseError::ExecutionFailed) => 4,
    }
}

const fn decode_status(code: u8) -> Option<Option<BrokerResponseError>> {
    match code {
        0 => Some(None),
        1 => Some(Some(BrokerResponseError::InvalidRequest)),
        2 => Some(Some(BrokerResponseError::AuthorizationDenied)),
        3 => Some(Some(BrokerResponseError::DeviceUnavailable)),
        4 => Some(Some(BrokerResponseError::ExecutionFailed)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::AtaIdentifySummary;

    fn round_trip(operation: AtaBrokerOperation, response: BrokerAtaResponse) {
        let expected = BrokerResponseFrame {
            request_id: 7,
            operation,
            result: Ok(response),
        };
        let encoded = encode_response_frame(&expected).unwrap();
        assert!(encoded.len() <= MAX_FRAME_LEN);
        assert_eq!(decode_response_frame(&encoded), Ok(expected));
    }

    #[test]
    fn every_typed_response_round_trips() {
        round_trip(
            AtaBrokerOperation::IdentifyDevice,
            BrokerAtaResponse::Identify(AtaIdentifySummary {
                capacity_bytes: Some(4_000_000_000_000),
                medium: AtaMedium::RotationalRpm(7_200),
                smart_supported: true,
                general_purpose_logging_supported: true,
            }),
        );
        round_trip(
            AtaBrokerOperation::SmartReadData,
            BrokerAtaResponse::SmartData(vec![SmartAttribute {
                id: 5,
                flags: 3,
                current: Some(100),
                worst: None,
                raw: [1, 2, 3, 4, 5, 6],
                threshold: Some(10),
            }]),
        );
        round_trip(
            AtaBrokerOperation::SmartReadThresholds,
            BrokerAtaResponse::SmartThresholds(vec![(5, 10), (9, 0)]),
        );
        round_trip(
            AtaBrokerOperation::SmartReturnStatus,
            BrokerAtaResponse::SmartStatus(SmartStatus::PredictingFailure),
        );
        round_trip(
            AtaBrokerOperation::ReadGplDirectory,
            BrokerAtaResponse::GplDirectory(AtaLogDirectory {
                version: 1,
                supported_pages: vec![AtaLogPageSupport {
                    address: 4,
                    sectors: 2,
                }],
            }),
        );
    }

    #[test]
    fn sanitized_errors_have_no_payload() {
        for error in [
            BrokerResponseError::InvalidRequest,
            BrokerResponseError::AuthorizationDenied,
            BrokerResponseError::DeviceUnavailable,
            BrokerResponseError::ExecutionFailed,
        ] {
            let expected = BrokerResponseFrame {
                request_id: 9,
                operation: AtaBrokerOperation::SmartReturnStatus,
                result: Err(error),
            };
            let encoded = encode_response_frame(&expected).unwrap();
            assert_eq!(encoded.len(), HEADER_LEN);
            assert_eq!(decode_response_frame(&encoded), Ok(expected));
        }
    }

    #[test]
    fn malformed_lengths_tags_and_operation_mismatch_are_rejected() {
        let mismatch = BrokerResponseFrame {
            request_id: 1,
            operation: AtaBrokerOperation::SmartReturnStatus,
            result: Ok(BrokerAtaResponse::SmartData(Vec::new())),
        };
        assert_eq!(
            encode_response_frame(&mismatch),
            Err(BrokerResponseWireError::OperationPayloadMismatch)
        );

        let valid = encode_response_frame(&BrokerResponseFrame {
            request_id: 1,
            operation: AtaBrokerOperation::SmartReturnStatus,
            result: Ok(BrokerAtaResponse::SmartStatus(SmartStatus::Passed)),
        })
        .unwrap();
        for length in 0..valid.len() {
            assert!(decode_response_frame(&valid[..length]).is_err());
        }
        let mut trailing = valid.clone();
        trailing.push(0);
        assert_eq!(
            decode_response_frame(&trailing),
            Err(BrokerResponseWireError::TrailingOrMissingBytes)
        );

        let mut error_with_payload = valid;
        error_with_payload[6] = 1;
        assert_eq!(
            decode_response_frame(&error_with_payload),
            Err(BrokerResponseWireError::UnexpectedErrorPayload)
        );
    }

    #[test]
    fn response_bounds_reject_oversized_collections() {
        let attributes = vec![
            SmartAttribute {
                id: 1,
                flags: 0,
                current: None,
                worst: None,
                raw: [0; 6],
                threshold: None,
            };
            MAX_SMART_ATTRIBUTES + 1
        ];
        assert_eq!(
            encode_response_frame(&BrokerResponseFrame {
                request_id: 1,
                operation: AtaBrokerOperation::SmartReadData,
                result: Ok(BrokerAtaResponse::SmartData(attributes)),
            }),
            Err(BrokerResponseWireError::PayloadTooLarge)
        );
    }
}
