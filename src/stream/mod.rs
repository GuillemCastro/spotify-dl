pub mod channel_sink;
pub mod stream;

// Re-export the Stream type for easier access
pub use stream::Stream;

pub enum StreamEvent {
    Write {
        bytes: usize,
        total: usize,
        content: Vec<i32>,
    },
    Finished,
    Retry{
        attempt: usize,
        max_attempts: usize,
    },
    Error(StreamError),
}

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Failed to load track: {0}")]
    LoadError(String),

    #[error("Unknown error occurred")]
    Unknown,
}

pub type StreamEventChannel = tokio::sync::mpsc::UnboundedReceiver<StreamEvent>;