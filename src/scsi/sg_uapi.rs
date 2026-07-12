//! Linux `sg_io_hdr_t` ABI metadata derived from `<scsi/sg.h>`.
//!
//! The actual ioctl remains broker-gated. Keeping this structure private means
//! callers cannot supply pointers, directions, or raw CDBs through this module.

use std::ffi::c_void;
#[cfg(target_os = "linux")]
use std::io;
use std::mem::{align_of, size_of};
#[cfg(target_os = "linux")]
use std::os::fd::RawFd;
#[cfg(target_os = "linux")]
use std::ptr;

#[repr(C)]
struct SgIoHdr {
    interface_id: libc::c_int,
    dxfer_direction: libc::c_int,
    cmd_len: libc::c_uchar,
    mx_sb_len: libc::c_uchar,
    iovec_count: libc::c_ushort,
    dxfer_len: libc::c_uint,
    dxferp: *mut c_void,
    cmdp: *mut libc::c_uchar,
    sbp: *mut libc::c_uchar,
    timeout: libc::c_uint,
    flags: libc::c_uint,
    pack_id: libc::c_int,
    usr_ptr: *mut c_void,
    status: libc::c_uchar,
    masked_status: libc::c_uchar,
    msg_status: libc::c_uchar,
    sb_len_wr: libc::c_uchar,
    host_status: libc::c_ushort,
    driver_status: libc::c_ushort,
    resid: libc::c_int,
    duration: libc::c_uint,
    info: libc::c_uint,
}

#[cfg(target_os = "linux")]
const SG_IO: libc::c_ulong = 0x2285;
#[cfg(target_os = "linux")]
const SG_DXFER_NONE: libc::c_int = -1;
#[cfg(target_os = "linux")]
const SG_DXFER_FROM_DEV: libc::c_int = -3;

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SgIoCompletion {
    pub data: Vec<u8>,
    pub sense: Vec<u8>,
    pub scsi_status: u8,
    pub host_status: u16,
    pub driver_status: u16,
    pub residual: i32,
    pub duration_ms: u32,
    pub info: u32,
}

#[cfg(target_os = "linux")]
pub(crate) fn execute_read_only(
    fd: RawFd,
    cdb: &[u8],
    data_len: usize,
    sense_len: usize,
    timeout_ms: u32,
) -> io::Result<SgIoCompletion> {
    let cmd_len = u8::try_from(cdb.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "SG_IO CDB is too large"))?;
    let mx_sb_len = u8::try_from(sense_len).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "SG_IO sense buffer is too large",
        )
    })?;
    let dxfer_len = u32::try_from(data_len).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "SG_IO data buffer is too large",
        )
    })?;
    if fd < 0 || cdb.is_empty() || sense_len == 0 || timeout_ms == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "SG_IO requires a live fd and bounded non-empty command/sense/timeout",
        ));
    }
    let mut data = vec![0u8; data_len];
    let mut sense = vec![0u8; sense_len];
    let mut header = SgIoHdr {
        interface_id: i32::from(b'S'),
        dxfer_direction: if data_len == 0 {
            SG_DXFER_NONE
        } else {
            SG_DXFER_FROM_DEV
        },
        cmd_len,
        mx_sb_len,
        iovec_count: 0,
        dxfer_len,
        dxferp: if data_len == 0 {
            ptr::null_mut()
        } else {
            data.as_mut_ptr().cast()
        },
        cmdp: cdb.as_ptr().cast_mut(),
        sbp: sense.as_mut_ptr(),
        timeout: timeout_ms,
        flags: 0,
        pack_id: 0,
        usr_ptr: ptr::null_mut(),
        status: 0,
        masked_status: 0,
        msg_status: 0,
        sb_len_wr: 0,
        host_status: 0,
        driver_status: 0,
        resid: 0,
        duration: 0,
        info: 0,
    };
    // SAFETY: header uses the host-checked C ABI layout; CDB, data, and sense
    // allocations remain live and immovable for this synchronous ioctl. Every
    // input length is converted to its UAPI width before pointers are exposed.
    let result = unsafe { libc::ioctl(fd, SG_IO, &mut header) };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    finish_completion(header, data, sense)
}

#[cfg(target_os = "linux")]
fn finish_completion(
    header: SgIoHdr,
    mut data: Vec<u8>,
    mut sense: Vec<u8>,
) -> io::Result<SgIoCompletion> {
    let residual = usize::try_from(header.resid).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "SG_IO returned a negative residual",
        )
    })?;
    if residual > data.len() || usize::from(header.sb_len_wr) > sense.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "SG_IO returned out-of-range completion lengths",
        ));
    }
    data.truncate(data.len() - residual);
    sense.truncate(usize::from(header.sb_len_wr));
    Ok(SgIoCompletion {
        data,
        sense,
        scsi_status: header.status,
        host_status: header.host_status,
        driver_status: header.driver_status,
        residual: header.resid,
        duration_ms: header.duration,
        info: header.info,
    })
}

#[cfg(target_pointer_width = "64")]
const _: [(); 88] = [(); size_of::<SgIoHdr>()];
#[cfg(target_pointer_width = "64")]
const _: [(); 8] = [(); align_of::<SgIoHdr>()];
#[cfg(target_pointer_width = "32")]
const _: [(); 64] = [(); size_of::<SgIoHdr>()];
#[cfg(target_pointer_width = "32")]
const _: [(); 4] = [(); align_of::<SgIoHdr>()];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SgIoAbiLayout {
    pub size: usize,
    pub alignment: usize,
}

pub const fn sg_io_abi_layout() -> SgIoAbiLayout {
    SgIoAbiLayout {
        size: size_of::<SgIoHdr>(),
        alignment: align_of::<SgIoHdr>(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_layout_matches_linux_sg_header_contract() {
        let layout = sg_io_abi_layout();
        #[cfg(target_pointer_width = "64")]
        assert_eq!(
            layout,
            SgIoAbiLayout {
                size: 88,
                alignment: 8
            }
        );
        #[cfg(target_pointer_width = "32")]
        assert_eq!(
            layout,
            SgIoAbiLayout {
                size: 64,
                alignment: 4
            }
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn direction_constants_match_linux_uapi_and_completion_lengths_are_bounded() {
        assert_eq!(SG_DXFER_NONE, -1);
        assert_eq!(SG_DXFER_FROM_DEV, -3);
        let mut header: SgIoHdr = unsafe { std::mem::zeroed() };
        header.resid = 3;
        header.sb_len_wr = 2;
        let completion = finish_completion(header, vec![1, 2, 3, 4], vec![5, 6, 7]).unwrap();
        assert_eq!(completion.data, vec![1]);
        assert_eq!(completion.sense, vec![5, 6]);

        let mut negative: SgIoHdr = unsafe { std::mem::zeroed() };
        negative.resid = -1;
        assert_eq!(
            finish_completion(negative, vec![0], vec![0])
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }
}
