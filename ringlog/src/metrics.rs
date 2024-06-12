use metriken::{metric, Counter, Gauge};

#[metric(name = "log_create", description = "logging targets initialized")]
pub static LOG_CREATE: Counter = Counter::new();

#[metric(
    name = "log_create_ex",
    description = "number of exceptions while initializing logging targets"
)]
pub static LOG_CREATE_EX: Counter = Counter::new();

#[metric(name = "log_destroy", description = "logging targets destroyed")]
pub static LOG_DESTROY: Counter = Counter::new();

#[metric(name = "log_curr", description = "current number of logging targets")]
pub static LOG_CURR: Gauge = Gauge::new();

#[metric(
    name = "log_open",
    description = "number of logging destinations which have been opened"
)]
pub static LOG_OPEN: Counter = Counter::new();

#[metric(
    name = "log_open_ex",
    description = "number of exceptions while opening logging destinations"
)]
pub static LOG_OPEN_EX: Counter = Counter::new();

#[metric(
    name = "log_write",
    description = "number of writes to all logging destinations"
)]
pub static LOG_WRITE: Counter = Counter::new();

#[metric(
    name = "log_write_byte",
    description = "number of bytes written to all logging destinations"
)]
pub static LOG_WRITE_BYTE: Counter = Counter::new();

#[metric(
    name = "log_write_ex",
    description = "number of exceptions while writing to logging destinations"
)]
pub static LOG_WRITE_EX: Counter = Counter::new();

#[metric(
    name = "log_skip",
    description = "number of log messages skipped due to sampling policy"
)]
pub static LOG_SKIP: Counter = Counter::new();

#[metric(
    name = "log_drop",
    description = "number of log messages dropped due to full queues"
)]
pub static LOG_DROP: Counter = Counter::new();

#[metric(
    name = "log_drop_byte",
    description = "number of bytes dropped due to full queues"
)]
pub static LOG_DROP_BYTE: Counter = Counter::new();

#[metric(
    name = "log_flush",
    description = "number of times logging destinations have been flushed"
)]
pub static LOG_FLUSH: Counter = Counter::new();

#[metric(
    name = "log_flush_ex",
    description = "number of times logging destinations have been flushed"
)]
pub static LOG_FLUSH_EX: Counter = Counter::new();
