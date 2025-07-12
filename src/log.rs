use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use once_cell::sync::OnceCell;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;
use tracing_subscriber::filter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::utils::get_dot_path;

static LOG_GUARD: OnceCell<WorkerGuard> = OnceCell::new();

const MAX_LOG_SIZE: u64 = 5 * 1024 * 1024; // 5 MB

#[derive(Clone)]
struct RotatingFileWriter {
    inner: Arc<Mutex<File>>,
    path: PathBuf,
}

impl RotatingFileWriter {
    fn new(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        std::fs::create_dir_all(path.parent().unwrap())?;
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(file)),
            path,
        })
    }
}

impl Write for RotatingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut file = self.inner.lock().unwrap();

        let metadata = file.metadata()?;
        if metadata.len() > MAX_LOG_SIZE {
            // Truncate the file
            *file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&self.path)?;
        }

        file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut file = self.inner.lock().unwrap();
        file.flush()
    }
}

pub fn configure_logger() -> Result<()> {
    let path = get_dot_path()?.join("spotify-dl.log");

    let writer = RotatingFileWriter::new(path)?;
    let (non_blocking, guard) = tracing_appender::non_blocking(writer);
    LOG_GUARD.set(guard).ok();

    let targets = filter::Targets::new()
        .with_target("spotify_dl", tracing::Level::DEBUG)
        .with_default(LevelFilter::OFF);

    let console_layer = fmt::layer().with_target(false).with_filter(
        EnvFilter::builder()
            .with_default_directive(LevelFilter::OFF.into())
            .from_env_lossy(),
    );

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_filter(EnvFilter::new("info"));

    Registry::default()
        .with(console_layer)
        .with(file_layer)
        .with(targets)
        .init();

    Ok(())
}
