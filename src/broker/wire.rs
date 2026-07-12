use super::{
    AtaBrokerOperation, BrokerDeviceRef, BrokerGeneration, BrokerRequest, MAX_DEVICE_ID_LEN,
    valid_device_id,
};

const MAGIC: [u8; 4] = *b"VWB1";
pub const BROKER_WIRE_VERSION: u8 = 1;
const HEADER_LEN: usize = 34;
const MAX_FRAME_LEN: usize = HEADER_LEN + MAX_DEVICE_ID_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerWireError {
    FrameTooShort,
    FrameTooLarge,
    InvalidMagic,
    UnsupportedVersion,
    NonZeroReserved,
    UnknownOperation,
    InvalidNodeLength,
    TrailingOrMissingBytes,
    InvalidNodeEncoding,
    InvalidRequest,
    UnauthorizedPeer,
    ReplayedOrOutOfOrder,
}

pub fn encode_request_frame(request: &BrokerRequest) -> Result<Vec<u8>, BrokerWireError> {
    if request.request_id == 0 || !valid_device_id(&request.device.node_id) {
        return Err(BrokerWireError::InvalidRequest);
    }
    let node = request.device.node_id.as_bytes();
    let node_len = u16::try_from(node.len()).map_err(|_| BrokerWireError::InvalidNodeLength)?;
    let mut frame = Vec::with_capacity(HEADER_LEN + node.len());
    frame.extend_from_slice(&MAGIC);
    frame.push(BROKER_WIRE_VERSION);
    frame.push(operation_code(request.operation));
    frame.extend_from_slice(&[0, 0]);
    frame.extend_from_slice(&request.request_id.to_le_bytes());
    frame.extend_from_slice(&request.device.generation.diskseq.to_le_bytes());
    frame.extend_from_slice(&request.device.generation.dev_major.to_le_bytes());
    frame.extend_from_slice(&request.device.generation.dev_minor.to_le_bytes());
    frame.extend_from_slice(&node_len.to_le_bytes());
    frame.extend_from_slice(node);
    Ok(frame)
}

pub fn decode_request_frame(frame: &[u8]) -> Result<BrokerRequest, BrokerWireError> {
    if frame.len() < HEADER_LEN {
        return Err(BrokerWireError::FrameTooShort);
    }
    if frame.len() > MAX_FRAME_LEN {
        return Err(BrokerWireError::FrameTooLarge);
    }
    if frame[..4] != MAGIC {
        return Err(BrokerWireError::InvalidMagic);
    }
    if frame[4] != BROKER_WIRE_VERSION {
        return Err(BrokerWireError::UnsupportedVersion);
    }
    if frame[6..8] != [0, 0] {
        return Err(BrokerWireError::NonZeroReserved);
    }
    let operation = decode_operation(frame[5]).ok_or(BrokerWireError::UnknownOperation)?;
    let request_id = u64::from_le_bytes(frame[8..16].try_into().expect("bounded header"));
    let diskseq = u64::from_le_bytes(frame[16..24].try_into().expect("bounded header"));
    let dev_major = u32::from_le_bytes(frame[24..28].try_into().expect("bounded header"));
    let dev_minor = u32::from_le_bytes(frame[28..32].try_into().expect("bounded header"));
    let node_len = usize::from(u16::from_le_bytes(
        frame[32..34].try_into().expect("bounded header"),
    ));
    if node_len == 0 || node_len > MAX_DEVICE_ID_LEN {
        return Err(BrokerWireError::InvalidNodeLength);
    }
    if frame.len() != HEADER_LEN + node_len {
        return Err(BrokerWireError::TrailingOrMissingBytes);
    }
    let node_id = std::str::from_utf8(&frame[HEADER_LEN..])
        .map_err(|_| BrokerWireError::InvalidNodeEncoding)?;
    if request_id == 0 || !valid_device_id(node_id) {
        return Err(BrokerWireError::InvalidRequest);
    }
    Ok(BrokerRequest {
        request_id,
        device: BrokerDeviceRef {
            node_id: node_id.to_owned(),
            generation: BrokerGeneration {
                diskseq,
                dev_major,
                dev_minor,
            },
        },
        operation,
    })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrokerPeerCredentials {
    pub uid: u32,
    pub gid: u32,
    pub pid: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrokerPeerPolicy {
    pub allowed_uid: u32,
    pub allowed_gid: u32,
}

impl BrokerPeerPolicy {
    pub const fn accepts(self, credentials: BrokerPeerCredentials) -> bool {
        credentials.pid != 0
            && credentials.uid == self.allowed_uid
            && credentials.gid == self.allowed_gid
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerSession {
    peer: BrokerPeerCredentials,
    last_request_id: u64,
}

impl BrokerSession {
    pub const fn new(
        policy: BrokerPeerPolicy,
        peer: BrokerPeerCredentials,
    ) -> Result<Self, BrokerWireError> {
        if !policy.accepts(peer) {
            return Err(BrokerWireError::UnauthorizedPeer);
        }
        Ok(Self {
            peer,
            last_request_id: 0,
        })
    }

    pub const fn peer(&self) -> BrokerPeerCredentials {
        self.peer
    }

    pub fn decode_next(&mut self, frame: &[u8]) -> Result<BrokerRequest, BrokerWireError> {
        let request = decode_request_frame(frame)?;
        if request.request_id <= self.last_request_id {
            return Err(BrokerWireError::ReplayedOrOutOfOrder);
        }
        self.last_request_id = request.request_id;
        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(operation: AtaBrokerOperation, request_id: u64) -> BrokerRequest {
        BrokerRequest {
            request_id,
            device: BrokerDeviceRef {
                node_id: "block:sda".to_owned(),
                generation: BrokerGeneration {
                    diskseq: 42,
                    dev_major: 8,
                    dev_minor: 0,
                },
            },
            operation,
        }
    }

    #[test]
    fn every_typed_operation_round_trips_without_raw_payload_fields() {
        for operation in [
            AtaBrokerOperation::IdentifyDevice,
            AtaBrokerOperation::SmartReadData,
            AtaBrokerOperation::SmartReadThresholds,
            AtaBrokerOperation::SmartReturnStatus,
            AtaBrokerOperation::ReadGplDirectory,
        ] {
            let request = request(operation, 1);
            let frame = encode_request_frame(&request).unwrap();
            assert!(frame.len() <= MAX_FRAME_LEN);
            assert_eq!(decode_request_frame(&frame), Ok(request));
        }
    }

    #[test]
    fn truncation_trailing_reserved_and_unknown_operations_are_rejected() {
        let frame = encode_request_frame(&request(AtaBrokerOperation::IdentifyDevice, 1)).unwrap();
        for length in 0..frame.len() {
            assert!(decode_request_frame(&frame[..length]).is_err());
        }
        let mut trailing = frame.clone();
        trailing.push(0);
        assert_eq!(
            decode_request_frame(&trailing),
            Err(BrokerWireError::TrailingOrMissingBytes)
        );
        let mut reserved = frame.clone();
        reserved[6] = 1;
        assert_eq!(
            decode_request_frame(&reserved),
            Err(BrokerWireError::NonZeroReserved)
        );
        let mut operation = frame;
        operation[5] = 0xff;
        assert_eq!(
            decode_request_frame(&operation),
            Err(BrokerWireError::UnknownOperation)
        );
    }

    #[test]
    fn peer_policy_and_monotonic_request_ids_block_unauthorized_replay() {
        let policy = BrokerPeerPolicy {
            allowed_uid: 1000,
            allowed_gid: 1000,
        };
        let peer = BrokerPeerCredentials {
            uid: 1000,
            gid: 1000,
            pid: 123,
        };
        assert_eq!(
            BrokerSession::new(policy, BrokerPeerCredentials { uid: 0, ..peer }),
            Err(BrokerWireError::UnauthorizedPeer)
        );
        let mut session = BrokerSession::new(policy, peer).unwrap();
        let first = encode_request_frame(&request(AtaBrokerOperation::IdentifyDevice, 7)).unwrap();
        assert!(session.decode_next(&first).is_ok());
        assert_eq!(
            session.decode_next(&first),
            Err(BrokerWireError::ReplayedOrOutOfOrder)
        );
        let older = encode_request_frame(&request(AtaBrokerOperation::SmartReadData, 6)).unwrap();
        assert_eq!(
            session.decode_next(&older),
            Err(BrokerWireError::ReplayedOrOutOfOrder)
        );
    }
}
