use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use exif::{Reader as ExifReader, Tag, In, Value};
use serde_json::json;

use crate::error::MetadataResult;

pub struct ImageMetadata {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub exif_json: Option<String>,
}

/// Extract basic EXIF data and dimensions from an image file.
pub fn extract_image_metadata<P: AsRef<Path>>(path: P) -> MetadataResult<ImageMetadata> {
    let file = File::open(path)?;
    let mut bufreader = BufReader::new(&file);
    let exifreader = ExifReader::new();
    
    let exif = match exifreader.read_from_container(&mut bufreader) {
        Ok(exif) => exif,
        Err(_) => {
            // Not all images have EXIF. Return empty.
            return Ok(ImageMetadata {
                width: None,
                height: None,
                exif_json: None,
            });
        }
    };

    let mut width = None;
    let mut height = None;
    let mut exif_map = serde_json::Map::new();

    for field in exif.fields() {
        let tag = field.tag;
        let value = format!("{}", field.display_value().with_unit(&exif));
        
        match tag {
            Tag::PixelXDimension | Tag::ImageWidth => {
                if let Value::Long(ref v) = field.value {
                    if let Some(&w) = v.first() { width = Some(w); }
                } else if let Value::Short(ref v) = field.value {
                    if let Some(&w) = v.first() { width = Some(w as u32); }
                }
            }
            Tag::PixelYDimension | Tag::ImageLength => {
                if let Value::Long(ref v) = field.value {
                    if let Some(&h) = v.first() { height = Some(h); }
                } else if let Value::Short(ref v) = field.value {
                    if let Some(&h) = v.first() { height = Some(h as u32); }
                }
            }
            Tag::Make => { exif_map.insert("camera_make".to_string(), json!(value)); }
            Tag::Model => { exif_map.insert("camera_model".to_string(), json!(value)); }
            Tag::DateTimeOriginal => { exif_map.insert("date_original".to_string(), json!(value)); }
            Tag::GPSLatitude => { exif_map.insert("gps_lat".to_string(), json!(value)); }
            Tag::GPSLongitude => { exif_map.insert("gps_lon".to_string(), json!(value)); }
            _ => {}
        }
    }
    
    let exif_json = if exif_map.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&exif_map).unwrap_or_default())
    };

    Ok(ImageMetadata {
        width,
        height,
        exif_json,
    })
}
