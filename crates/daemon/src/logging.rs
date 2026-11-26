//! Inisialisasi logging `tracing` rolling file dengan lapisan redaksi rahasia (T-134, NFR-05).

use std::io::{self, Write};
use std::path::Path;

use latch_core::redact::redact_secrets;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::EnvFilter;

/// Pembungkus `Write` yang otomatis menyamarkan token/API key sebelum menulis log.
pub struct RedactingWriter<W: Write> {
    inner: W,
}

impl<W: Write> Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        let redacted = redact_secrets(&text);
        self.inner.write_all(redacted.as_bytes())?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Implementasi `MakeWriter` agar seluruh event `tracing` tersaring sebelum ke file rolling.
#[derive(Clone)]
pub struct RedactingMakeWriter<M> {
    inner: M,
}

impl<M> RedactingMakeWriter<M> {
    /// Membungkus `MakeWriter` bawaan dengan filter redaksi rahasia.
    pub fn new(inner: M) -> Self {
        Self { inner }
    }
}

impl<'a, M> MakeWriter<'a> for RedactingMakeWriter<M>
where
    M: MakeWriter<'a>,
{
    type Writer = RedactingWriter<M::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter {
            inner: self.inner.make_writer(),
        }
    }
}

/// Mengaktifkan subscriber `tracing` ke file rolling harian di `logs_dir` dengan filter `LATCH_LOG`.
pub fn init_logging(logs_dir: &Path) -> io::Result<WorkerGuard> {
    std::fs::create_dir_all(logs_dir)?;
    let appender = tracing_appender::rolling::daily(logs_dir, "latchd.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);
    let make_writer = RedactingMakeWriter::new(non_blocking);

    let filter = EnvFilter::try_from_env("LATCH_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(make_writer)
        .try_init();

    Ok(guard)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::RedactingWriter;

    #[test]
    fn redacts_canary_secret_written_to_log() {
        let mut sink = Vec::new();
        {
            let mut w = RedactingWriter { inner: &mut sink };
            w.write_all(b"INFO proxy forward Authorization: Bearer sk-test-CANARY99887766\n")
                .unwrap();
        }
        let logged = String::from_utf8(sink).unwrap();
        assert!(!logged.contains("CANARY99887766"));
        assert!(logged.contains("[REDACTED]"));
    }
}
