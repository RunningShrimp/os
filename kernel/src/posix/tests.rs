//! POSIX interface tests
//!
//! Tests for various POSIX functionality

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_set_operations() {
        let mut sigset = SigSet::empty();

        // Test adding signals
        sigset.add(SIGINT);
        assert!(sigset.has(SIGINT));
        assert!(!sigset.has(SIGTERM));

        // Test removing signals
        sigset.remove(SIGINT);
        assert!(!sigset.has(SIGINT));

        // Test all signals
        sigset.add(SIGINT);
        sigset.add(SIGTERM);
        sigset.add(SIGUSR1);
        assert!(sigset.has(SIGINT));
        assert!(sigset.has(SIGTERM));
        assert!(sigset.has(SIGUSR1));

        // Test clear
        sigset.clear();
        assert!(!sigset.has(SIGINT));
        assert!(!sigset.has(SIGTERM));
        assert!(!sigset.has(SIGUSR1));
    }

    #[test]
    fn test_timespec_operations() {
        // Test creating timespec
        let ts1 = Timespec::new(100, 500000000);
        assert_eq!(ts1.tv_sec, 100);
        assert_eq!(ts1.tv_nsec, 500000000);

        // Test zero timespec
        let ts0 = Timespec::zero();
        assert_eq!(ts0.tv_sec, 0);
        assert_eq!(ts0.tv_nsec, 0);

        // Test nanoseconds conversion
        let total_nanos = ts1.to_nanos();
        assert_eq!(total_nanos, 100500000000i64);

        // Test from nanoseconds
        let ts2 = Timespec::from_nanos(1500000000);
        assert_eq!(ts2.tv_sec, 1);
        assert_eq!(ts2.tv_nsec, 500000000);
    }

    #[test]
    fn test_fd_set_operations() {
        let mut fdset = FdSet::default();

        // Test setting and checking file descriptors
        fd_zero(&mut fdset);

        fd_set(&mut fdset, 5);
        fd_set(&mut fdset, 10);
        fd_set(&mut fdset, 100);

        assert!(fd_isset(&fdset, 5));
        assert!(fd_isset(&fdset, 10));
        assert!(fd_isset(&fdset, 100));
        assert!(!fd_isset(&fdset, 7));

        // Test clearing file descriptors
        fd_clr(&mut fdset, 10);
        assert!(fd_isset(&fdset, 5));
        assert!(!fd_isset(&fdset, 10));
        assert!(fd_isset(&fdset, 100));
    }

    #[test]
    fn test_file_mode_checks() {
        // Test file type checking functions
        assert!(s_isreg(S_IFREG));
        assert!(s_isdir(S_IFDIR));
        assert!(s_ischr(S_IFCHR));
        assert!(s_isblk(S_IFBLK));
        assert!(s_isfifo(S_IFIFO));
        assert!(s_islnk(S_IFLNK));
        assert!(s_issock(S_IFSOCK));

        // Test combined mode bits
        let file_mode = S_IFREG | S_IRUSR | S_IWUSR | S_IRGRP;
        assert!(s_isreg(file_mode));
        assert!(!s_isdir(file_mode));
    }

    #[test]
    fn test_wait_status_macros() {
        // Test wait status macros
        let exit_status = w_exitcode(42, SIGTERM);
        assert!(wifexited(exit_status));
        assert!(!wifsignaled(exit_status));
        assert_eq!(wexitstatus(exit_status), 42);

        let signal_status = SIGTERM;
        assert!(!wifexited(signal_status));
        assert!(wifsignaled(signal_status));
        assert_eq!(wtermsig(signal_status), SIGTERM);
    }

    #[test]
    fn test_mqueue_attributes() {
        let mut attr = MqAttr::default();

        assert_eq!(attr.mq_maxmsg, 10);
        assert_eq!(attr.mq_msgsize, 8192);
        assert_eq!(attr.mq_curmsgs, 0);
        assert_eq!(attr.mq_flags, 0);

        // Modify attributes
        attr.mq_maxmsg = 20;
        attr.mq_msgsize = 4096;
        attr.mq_flags = O_NONBLOCK;

        assert_eq!(attr.mq_maxmsg, 20);
        assert_eq!(attr.mq_msgsize, 4096);
        assert_eq!(attr.mq_flags, O_NONBLOCK);
    }

    #[test]
    fn test_timer_specifications() {
        let mut itspec = Itimerspec::default();

        assert_eq!(itspec.it_interval.tv_sec, 0);
        assert_eq!(itspec.it_interval.tv_nsec, 0);
        assert_eq!(itspec.it_value.tv_sec, 0);
        assert_eq!(itspec.it_value.tv_nsec, 0);

        // Set interval and initial value
        itspec.it_interval = Timespec::new(1, 0); // 1 second interval
        itspec.it_value = Timespec::new(5, 0);    // 5 second initial delay

        assert_eq!(itspec.it_interval.tv_sec, 1);
        assert_eq!(itspec.it_interval.tv_nsec, 0);
        assert_eq!(itspec.it_value.tv_sec, 5);
        assert_eq!(itspec.it_value.tv_nsec, 0);
    }
}