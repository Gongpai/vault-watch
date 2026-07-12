use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;

use super::{DataDirection, ReadOnlyCommand};

pub const MAX_DATA_LEN: usize = 64 * 1024;
pub const MAX_SENSE_LEN: usize = 252;
pub const MIN_TIMEOUT: Duration = Duration::from_millis(100);
pub const MAX_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestError {
    DataLengthTooLarge,
    SenseLengthOutOfRange,
    TimeoutOutOfRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SgIoRequest {
    pub command: ReadOnlyCommand,
    pub data_len: usize,
    pub sense_len: usize,
    pub timeout: Duration,
}

impl SgIoRequest {
    pub fn new(
        command: ReadOnlyCommand,
        sense_len: usize,
        timeout: Duration,
    ) -> Result<Self, RequestError> {
        let data_len = command.allocation_len();
        if data_len > MAX_DATA_LEN {
            return Err(RequestError::DataLengthTooLarge);
        }
        if sense_len == 0 || sense_len > MAX_SENSE_LEN {
            return Err(RequestError::SenseLengthOutOfRange);
        }
        if !(MIN_TIMEOUT..=MAX_TIMEOUT).contains(&timeout) {
            return Err(RequestError::TimeoutOutOfRange);
        }
        debug_assert!(matches!(
            command.direction(),
            DataDirection::None | DataDirection::FromDevice
        ));
        Ok(Self {
            command,
            data_len,
            sense_len,
            timeout,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SgIoResponse {
    pub data: Vec<u8>,
    pub sense: Vec<u8>,
    pub scsi_status: u8,
    pub host_status: u16,
    pub driver_status: u16,
    pub residual: i32,
}

pub trait TypedScsiExecutor: Send + Sync + 'static {
    type Error: Debug + Send + 'static;

    fn execute(&self, request: SgIoRequest) -> Result<SgIoResponse, Self::Error>;
}

#[derive(Debug)]
pub enum WorkerError<E> {
    Executor(E),
    Join,
    Closed,
}

pub struct BoundedScsiWorker<E> {
    executor: Arc<E>,
    permits: Arc<Semaphore>,
}

impl<E: TypedScsiExecutor> BoundedScsiWorker<E> {
    pub fn new(executor: E, max_concurrency: usize) -> Option<Self> {
        (max_concurrency > 0).then(|| Self {
            executor: Arc::new(executor),
            permits: Arc::new(Semaphore::new(max_concurrency)),
        })
    }

    pub async fn execute(
        &self,
        request: SgIoRequest,
    ) -> Result<SgIoResponse, WorkerError<E::Error>> {
        let permit = Arc::clone(&self.permits)
            .acquire_owned()
            .await
            .map_err(|_| WorkerError::Closed)?;
        let executor = Arc::clone(&self.executor);
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            executor.execute(request).map_err(WorkerError::Executor)
        })
        .await
        .map_err(|_| WorkerError::Join)?
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::scsi::{LogPage, VpdPage};

    struct MockExecutor {
        calls: AtomicUsize,
    }

    impl TypedScsiExecutor for MockExecutor {
        type Error = ();

        fn execute(&self, request: SgIoRequest) -> Result<SgIoResponse, Self::Error> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(SgIoResponse {
                data: vec![0; request.data_len],
                sense: Vec::new(),
                scsi_status: 0,
                host_status: 0,
                driver_status: 0,
                residual: 0,
            })
        }
    }

    #[test]
    fn request_limits_are_enforced_before_worker_submission() {
        let command = ReadOnlyCommand::InquiryVpd {
            page: VpdPage::Supported,
            allocation_len: 64,
        };
        assert!(SgIoRequest::new(command, 32, Duration::from_secs(2)).is_ok());
        assert_eq!(
            SgIoRequest::new(command, 0, Duration::from_secs(2)),
            Err(RequestError::SenseLengthOutOfRange)
        );
        assert_eq!(
            SgIoRequest::new(command, 32, Duration::from_millis(99)),
            Err(RequestError::TimeoutOutOfRange)
        );
        assert_eq!(
            SgIoRequest::new(command, 32, Duration::from_secs(31)),
            Err(RequestError::TimeoutOutOfRange)
        );
    }

    #[tokio::test]
    async fn bounded_worker_accepts_only_typed_requests_off_async_executor() {
        let worker = BoundedScsiWorker::new(
            MockExecutor {
                calls: AtomicUsize::new(0),
            },
            1,
        )
        .unwrap();
        let request = SgIoRequest::new(
            ReadOnlyCommand::LogSense {
                page: LogPage::Temperature,
                allocation_len: 512,
            },
            32,
            Duration::from_secs(2),
        )
        .unwrap();
        let response = worker.execute(request).await.unwrap();
        assert_eq!(response.data.len(), 512);
        assert!(BoundedScsiWorker::new(MockExecutor { calls: 0.into() }, 0).is_none());
    }
}
