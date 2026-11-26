#![forbid(unsafe_code)]
//! Manajemen kolam PTY paralel, ring buffer terikat, rolling log, batching frame 8-16 ms,
//! layar virtual `vt100`, serta kontrol SIGSTOP/SIGCONT/kill (T-120..T-124).

pub mod ring_buffer;
pub mod session;

pub use ring_buffer::{RingBuffer, RollingLogWriter};
pub use session::{PtyError, PtyFrame, PtyPool, PtySpawnRequest, PtySubscriber};
