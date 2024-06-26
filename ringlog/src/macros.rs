#[macro_export]
/// Logs a fatal error and terminates the program.
macro_rules! fatal {
    () => (
        error!();
        std::process::exit(1);
        );
    ($fmt:expr) => (
        error!($fmt);
        std::process::exit(1);
        );
    ($fmt:expr, $($arg:tt)*) => (
        error!($fmt, $($arg)*);
        std::process::exit(1);
        );
}

#[cfg(feature = "metrics")]
macro_rules! metrics {
    { $( $tt:tt )* } => { $( $tt )* }
}

#[cfg(not(feature = "metrics"))]
macro_rules! metrics {
    { $( $tt:tt)* } => {}
}
