struct Logger {
    lock: crate::util::SpinLock<()>, // To unscramble concurrent log messages.
}

static LOGGER: Logger = Logger {
    lock: crate::util::SpinLock::new(()),
};

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let _lock = self.lock.lock(line!());

            let line = record.line().unwrap_or(0);
            let target = record.target();

            let tm = crate::arch::time::system_start_time().elapsed();

            let millis = tm.as_millis();
            let secs = millis / 1000;
            let millis = millis % 1000;

            let cpu = crate::arch::current_cpu();

            crate::arch::arch_write_serial!(
                "{:3}:{:03} {:2}: {:6} {}:{} - {}\n\r",
                secs,
                millis,
                cpu,
                record.level(),
                target,
                line,
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

pub fn log_user(thread: &crate::uspace::process::Thread, msg: &str) {
    let _lock = LOGGER.lock.lock(line!());

    let thr = thread.debug_name();
    let tm = crate::arch::time::system_start_time().elapsed();
    let millis = tm.as_millis();
    let secs = millis / 1000;
    let millis = millis % 1000;

    let cpu = crate::arch::current_cpu();

    crate::arch::arch_write_serial!(
        "{:3}:{:03} {:2}: {:6} {}: {}\n\r",
        secs,
        millis,
        cpu,
        "USER",
        thr,
        msg
    );
}

// Initializes the logger from crate log.
// Must be called after the global allocator has been set up.
pub fn init_logging() {
    assert!(log::set_logger(&LOGGER).is_ok());

    #[cfg(debug_assertions)]
    log::set_max_level(log::LevelFilter::Debug);
    #[cfg(not(debug_assertions))]
    log::set_max_level(log::LevelFilter::Info);

    // NOTE: init.rs sets max log level to INFO before starting
    // the userspace, as otherwise the kernel spams the console too much.
    // This can be overwritten via SysCtl from the userspace.
}

pub fn lock() -> crate::util::LockGuard<'static, ()> {
    LOGGER.lock.lock(line!())
}
