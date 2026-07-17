use std::path::Path;
use lofty::probe::Probe;
use lofty::file::AudioFile;

use crate::error::MetadataResult;

pub struct AudioMetadata {
    pub duration_secs: Option<f64>,
    pub bitrate: Option<u32>,
    pub codec: Option<String>,
}

/// Extract duration and bitrate from audio/video files (supported by Lofty).
pub fn extract_audio_metadata<P: AsRef<Path>>(path: P) -> MetadataResult<AudioMetadata> {
    let tagged_file = match Probe::open(path).and_then(|p| p.read()) {
        Ok(f) => f,
        Err(_) => {
            return Ok(AudioMetadata {
                duration_secs: None,
                bitrate: None,
                codec: None,
            });
        }
    };

    let properties = tagged_file.properties();
    
    let duration = properties.duration().as_secs_f64();
    let bitrate = properties.audio_bitrate();

    Ok(AudioMetadata {
        duration_secs: if duration > 0.0 { Some(duration) } else { None },
        bitrate,
        codec: None, // Lofty properties doesn't expose a simple string codec name globally yet
    })
}
