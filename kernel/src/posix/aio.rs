//! POSIX AIO Types (aio.h)

use super::types::{off_t, size_t, aio_reqprio_t};

/// AIO operation codes
pub const LIO_READ: i32 = 0;
pub const LIO_WRITE: i32 = 1;
pub const LIO_NOP: i32 = 2;
pub const LIO_WAIT: i32 = 1;
pub const LIO_NOWAIT: i32 = 0;

/// AIO return values
pub const AIO_CANCELED: i32 = -1;
pub const AIO_NOTCANCELED: i32 = 0;
pub const AIO_ALLDONE: i32 = 1;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SigVal {
    pub sival_int: i32,
    pub sival_ptr: usize,
}

impl Default for SigVal {
    fn default() -> Self {
        Self {
            sival_int: 0,
            sival_ptr: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct aio_sigevent_t {
    pub sigev_notify: i32,
    pub sigev_signo: i32,
    pub sigev_value: SigVal,
    pub sigev_notify_function: usize,
    pub sigev_notify_attributes: usize,
}

impl Default for aio_sigevent_t {
    fn default() -> Self {
        Self {
            sigev_notify: 0,
            sigev_signo: 0,
            sigev_value: SigVal::default(),
            sigev_notify_function: 0,
            sigev_notify_attributes: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct aiocb {
    pub aio_fildes: i32,
    pub aio_offset: off_t,
    pub aio_buf: *mut u8,
    pub aio_nbytes: size_t,
    pub __return_value: isize,
    pub __error_code: i32,
    pub aio_reqprio: aio_reqprio_t,
    pub aio_sigevent: aio_sigevent_t,
    pub aio_lio_opcode: i32,
    pub aio_fsync_mode: i32,
    pub aio_listio: *mut *mut aiocb,
    pub aio_nent: i32,
}

impl Default for aiocb {
    fn default() -> Self {
        Self {
            aio_fildes: -1,
            aio_offset: 0,
            aio_buf: core::ptr::null_mut(),
            aio_nbytes: 0,
            __return_value: 0,
            __error_code: 0,
            aio_reqprio: 0,
            aio_sigevent: aio_sigevent_t::default(),
            aio_lio_opcode: LIO_NOP,
            aio_fsync_mode: 0,
            aio_listio: core::ptr::null_mut(),
            aio_nent: 0,
        }
    }
}

pub type AioOffsetT = off_t;
