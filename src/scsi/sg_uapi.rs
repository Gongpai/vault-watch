//! Linux `sg_io_hdr_t` ABI metadata derived from `<scsi/sg.h>`.
//!
//! The actual ioctl remains broker-gated. Keeping this structure private means
//! callers cannot supply pointers, directions, or raw CDBs through this module.

use std::ffi::c_void;
use std::mem::{align_of, size_of};

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
}
