//! Ring buffer memori berkapasitas tetap dan penulis log rolling ke disk (T-121).

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Buffer melingkar byte berkapasitas tetap agar penggunaan RAM PTY selalu terbatas.
#[derive(Debug)]
pub struct RingBuffer {
    buf: VecDeque<u8>,
    capacity: usize,
}

impl RingBuffer {
    /// Membuat ring buffer dengan kapasitas maksimum dalam byte.
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(1024);
        Self {
            buf: VecDeque::with_capacity(cap.min(64 * 1024)),
            capacity: cap,
        }
    }

    /// Menambahkan irisan byte baru; byte terlama dibuang bila melampaui kapasitas.
    pub fn push(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        if data.len() >= self.capacity {
            self.buf.clear();
            let tail = &data[data.len() - self.capacity..];
            self.buf.extend(tail.iter().copied());
            return;
        }
        let overflow = (self.buf.len() + data.len()).saturating_sub(self.capacity);
        if overflow > 0 {
            self.buf.drain(..overflow);
        }
        self.buf.extend(data.iter().copied());
    }

    /// Mengembalikan salinan isi buffer saat ini dalam urutan kronologis.
    pub fn snapshot(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.buf.len());
        let (a, b) = self.buf.as_slices();
        out.extend_from_slice(a);
        out.extend_from_slice(b);
        out
    }

    /// Panjang byte tersimpan saat ini.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Apakah buffer kosong.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

/// Penulis log PTY ke disk dengan rotasi ukuran maksimal dan izin `0600`.
pub struct RollingLogWriter {
    path: PathBuf,
    backup_path: PathBuf,
    file: File,
    written: u64,
    max_bytes: u64,
}

impl RollingLogWriter {
    /// Membuka berkas log rolling di `path` dengan batas `max_bytes` per berkas.
    pub fn open(path: &Path, max_bytes: u64) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = open_private_file(path, false)?;
        let written = file.metadata()?.len();
        let backup_path = path.with_extension("log.1");
        Ok(Self {
            path: path.to_path_buf(),
            backup_path,
            file,
            written,
            max_bytes: max_bytes.max(4096),
        })
    }

    /// Menulis chunk output ke log rolling dan merotasi berkas bila melampaui `max_bytes`.
    pub fn write_chunk(&mut self, data: &[u8]) -> io::Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        if self.written + (data.len() as u64) > self.max_bytes {
            self.rotate()?;
        }
        self.file.write_all(data)?;
        self.written += data.len() as u64;
        Ok(())
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.file.flush()?;
        if self.backup_path.exists() {
            let _ = std::fs::remove_file(&self.backup_path);
        }
        if self.path.exists() {
            std::fs::rename(&self.path, &self.backup_path)?;
        }
        self.file = open_private_file(&self.path, true)?;
        self.written = 0;
        Ok(())
    }
}

fn open_private_file(path: &Path, truncate: bool) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.create(true).write(true);
    if truncate {
        opts.truncate(true);
    } else {
        opts.append(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{RingBuffer, RollingLogWriter};

    #[test]
    fn ring_buffer_caps_memory_under_high_throughput() {
        let mut rb = RingBuffer::new(4096);
        let line = vec![b'A'; 128];
        for _ in 0..20_000 {
            rb.push(&line);
        }
        assert_eq!(rb.len(), 4096);
        assert_eq!(rb.snapshot().len(), 4096);
    }

    #[test]
    fn rolling_log_rotates_and_stays_bounded() {
        let dir = TempDir::new().unwrap();
        let log_path = dir.path().join("pty.log");
        let mut writer = RollingLogWriter::open(&log_path, 4096).unwrap();
        let chunk = vec![b'x'; 512];
        for _ in 0..40 {
            writer.write_chunk(&chunk).unwrap();
        }
        let active_len = std::fs::metadata(&log_path).unwrap().len();
        let backup_len = std::fs::metadata(dir.path().join("pty.log.1"))
            .unwrap()
            .len();
        assert!(active_len <= 4096);
        assert!(backup_len <= 4096);
    }
}
