use serde::{Deserialize, Serialize};

/// Task priority levels (P0 = highest, P9 = lowest).
///
/// Maps directly to the StorageMind architecture spec. Lower numeric value
/// means higher scheduling urgency. P0/P1 tasks are never paused; P2+ tasks
/// respect the scheduler's pause state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum Priority {
    /// P0: UI + IPC (never yield)
    UiIpc = 0,
    /// P1: File system watcher events
    Watcher = 1,
    /// P2: Scanner (Stage 1)
    Scanner = 2,
    /// P3: Metadata parsing (Stage 2)
    Metadata = 3,
    /// P4: Thumbnail generation
    Thumbnail = 4,
    /// P5: Hash computation (for deduplication)
    Hash = 5,
    /// P6: OCR
    Ocr = 6,
    /// P7: MobileCLIP embedding
    MobileClip = 7,
    /// P8: Similarity computation
    Similarity = 8,
    /// P9: Video analysis (most expensive)
    VideoAnalysis = 9,
}

impl Priority {
    /// Returns the raw priority number (0 = highest, 9 = lowest).
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Constructs a `Priority` from a raw `u8`, defaulting to
    /// [`Priority::VideoAnalysis`] for any value ≥ 9.
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::UiIpc,
            1 => Self::Watcher,
            2 => Self::Scanner,
            3 => Self::Metadata,
            4 => Self::Thumbnail,
            5 => Self::Hash,
            6 => Self::Ocr,
            7 => Self::MobileClip,
            8 => Self::Similarity,
            _ => Self::VideoAnalysis,
        }
    }

    /// Human-readable task name used for tracing spans and tokio task labels.
    pub fn tokio_task_name(&self) -> &'static str {
        match self {
            Self::UiIpc => "ui-ipc",
            Self::Watcher => "watcher",
            Self::Scanner => "scanner",
            Self::Metadata => "metadata",
            Self::Thumbnail => "thumbnail",
            Self::Hash => "hash",
            Self::Ocr => "ocr",
            Self::MobileClip => "mobileclip",
            Self::Similarity => "similarity",
            Self::VideoAnalysis => "video-analysis",
        }
    }

    /// Returns `true` for P0/P1 tasks that are never subject to the pause gate.
    pub fn is_high_priority(self) -> bool {
        self.as_u8() < Priority::Scanner.as_u8()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_ordering() {
        assert!(Priority::UiIpc < Priority::Scanner);
        assert!(Priority::Scanner < Priority::VideoAnalysis);
    }

    #[test]
    fn round_trip_u8() {
        for v in 0u8..=9 {
            assert_eq!(Priority::from_u8(v).as_u8(), v);
        }
        // Out-of-range saturates to VideoAnalysis (9)
        assert_eq!(Priority::from_u8(255), Priority::VideoAnalysis);
    }

    #[test]
    fn high_priority_flag() {
        assert!(Priority::UiIpc.is_high_priority());
        assert!(Priority::Watcher.is_high_priority());
        assert!(!Priority::Scanner.is_high_priority());
        assert!(!Priority::VideoAnalysis.is_high_priority());
    }
}
