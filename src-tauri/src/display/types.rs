use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub id: String,
    pub name: String,
    pub is_active: bool,
    pub is_primary: bool,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathLabel {
    pub gdi_device_name: String,
    pub target_device_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayProfile {
    pub version: u32,
    pub paths_b64: String,
    pub modes_b64: String,
    pub path_labels: Vec<PathLabel>,
}

pub fn encode_structs<T: Copy>(items: &[T]) -> String {
    let bytes = unsafe {
        std::slice::from_raw_parts(
            items.as_ptr() as *const u8,
            std::mem::size_of_val(items),
        )
    };
    STANDARD.encode(bytes)
}

pub fn decode_structs<T: Copy>(b64: &str) -> Result<Vec<T>, String> {
    let bytes = STANDARD
        .decode(b64)
        .map_err(|e| format!("Invalid profile encoding: {e}"))?;
    let size = std::mem::size_of::<T>();
    if bytes.len() % size != 0 {
        return Err("Profile data size mismatch".into());
    }
    let count = bytes.len() / size;
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        let item = unsafe {
            std::ptr::read_unaligned(bytes.as_ptr().add(i * size) as *const T)
        };
        items.push(item);
    }
    Ok(items)
}