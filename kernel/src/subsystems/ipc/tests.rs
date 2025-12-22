//! Pipe Tests
//!
//! Tests for pipe IPC functionality

#[cfg(feature = "kernel_tests")]
pub mod pipe_tests {
    use crate::{test_assert_eq, test_assert};
    use crate::tests::{skip_test, TestResult};
    use crate::ipc::pipe;
    use crate::fs::file;

    /// Test basic pipe operations
    pub fn test_pipe_basic() -> TestResult {
        let result = pipe::pipe_alloc();
        if let Some((read_fd, write_fd)) = result {
            let data = b"hello";
            let written = pipe::pipe_write(write_fd, data);
            test_assert_eq!(written, 5);

            let mut buf = [0u8; 16];
            let read = pipe::pipe_read(read_fd, &mut buf);
            test_assert_eq!(read, 5);
            test_assert_eq!(&buf[..5], b"hello");

            file::file_close(read_fd);
            file::file_close(write_fd);
            Ok(())
        } else {
            skip_test("pipe_alloc not available")
        }
    }

    /// Test pipe with non-blocking mode
    pub fn test_pipe_nonblock() -> TestResult {
        use crate::posix::O_NONBLOCK;
        
        if let Some((rfd_idx, wfd_idx)) = pipe::pipe_alloc() {
            {
                let mut table = file::FILE_TABLE.lock();
                if let Some(f) = table.get_mut(rfd_idx) {
                    f.status_flags |= O_NONBLOCK;
                }
            }
            let mut buf = [0u8; 4];
            let ret = file::file_read(rfd_idx, &mut buf);
            test_assert_eq!(ret, crate::reliability::errno::errno_neg(crate::reliability::errno::EAGAIN));
            file::file_close(rfd_idx);
            file::file_close(wfd_idx);
            Ok(())
        } else {
            skip_test("pipe_alloc not available")
        }
    }

    /// Test pipe close behavior
    pub fn test_pipe_close_read() -> TestResult {
        if let Some((rfd_idx, wfd_idx)) = pipe::pipe_alloc() {
            file::file_close(rfd_idx);
            let buf = [0xBBu8; 16];
            let n = file::file_write(wfd_idx, &buf);
            test_assert_eq!(n, crate::reliability::errno::errno_neg(crate::reliability::errno::EPIPE));
            file::file_close(wfd_idx);
            Ok(())
        } else {
            skip_test("pipe_alloc not available")
        }
    }
}
