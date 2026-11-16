use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::SecondsFormat;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ClipContentType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "richText")]
    RichText,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "fileURL")]
    FileURL,
    #[serde(rename = "color")]
    Color,
}

impl ClipContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClipContentType::Text => "text",
            ClipContentType::RichText => "richText",
            ClipContentType::Image => "image",
            ClipContentType::FileURL => "fileURL",
            ClipContentType::Color => "color",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipItem {
    pub id: String,
    #[serde(rename = "type")]
    pub content_type: ClipContentType,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub rich_text_data: Option<String>,
    #[serde(default)]
    pub image_file_name: Option<String>,
    #[serde(default)]
    pub image_width: Option<u32>,
    #[serde(default)]
    pub image_height: Option<u32>,
    #[serde(default)]
    pub file_paths: Option<Vec<String>>,
    #[serde(default)]
    pub source_app_bundle_id: Option<String>,
    #[serde(default)]
    pub source_app_name: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub is_pinned: bool,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryStats {
    pub total: usize,
    pub pinned: usize,
}

impl ClipItem {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        content_type: ClipContentType,
        text: Option<String>,
        rich_text_data: Option<Vec<u8>>,
        image_file_name: Option<String>,
        image_width: Option<u32>,
        image_height: Option<u32>,
        file_paths: Option<Vec<String>>,
        source_app_bundle_id: Option<String>,
        source_app_name: Option<String>,
        content_hash: Option<String>,
    ) -> Self {
        let computed_hash = content_hash.unwrap_or_else(|| {
            Self::calculate_hash(
                content_type,
                text.as_deref(),
                image_file_name.as_deref(),
                file_paths.as_deref(),
            )
        });

        Self {
            id: Uuid::new_v4().to_string(),
            content_type,
            text,
            rich_text_data: rich_text_data.map(|bytes| BASE64.encode(bytes)),
            image_file_name,
            image_width,
            image_height,
            file_paths,
            source_app_bundle_id,
            source_app_name,
            created_at: chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            is_pinned: false,
            content_hash: computed_hash,
        }
    }

    pub fn calculate_hash(
        content_type: ClipContentType,
        text: Option<&str>,
        image_file_name: Option<&str>,
        file_paths: Option<&[String]>,
    ) -> String {
        let mut raw = format!("{}:", content_type.as_str());
        if let Some(t) = text {
            raw.push_str(t);
        } else if let Some(img) = image_file_name {
            raw.push_str(img);
        } else if let Some(paths) = file_paths {
            raw.push_str(&paths.join("|"));
        }

        Self::sha256_hex(raw.as_bytes())
    }

    pub fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        digest.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn rich_text_bytes(&self) -> Option<Vec<u8>> {
        self.rich_text_data
            .as_deref()
            .and_then(|encoded| BASE64.decode(encoded).ok())
    }
}

pub fn is_color_format(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }

    static HEX_RE: OnceLock<Regex> = OnceLock::new();
    static RGB_RE: OnceLock<Regex> = OnceLock::new();

    let hex_re = HEX_RE.get_or_init(|| {
        Regex::new(r"^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$")
            .expect("valid hex color regex")
    });
    if hex_re.is_match(trimmed) {
        return true;
    }

    let rgb_re = RGB_RE.get_or_init(|| {
        Regex::new(r"^rgba?\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*(,\s*(0|1|0?\.\d+)\s*)?\)$")
            .expect("valid rgb color regex")
    });
    rgb_re.is_match(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_format_detection() {
        assert!(is_color_format("#fff"));
        assert!(is_color_format("#0A84FF"));
        assert!(is_color_format("#0A84FF80"));
        assert!(is_color_format("  #abc123  "));
        assert!(is_color_format("rgb(10, 132, 255)"));
        assert!(is_color_format("rgba(255, 159, 10, 0.75)"));
        assert!(is_color_format("rgba(0,0,0,1)"));

        assert!(!is_color_format(""));
        assert!(!is_color_format("#12"));
        assert!(!is_color_format("#12345"));
        assert!(!is_color_format("#gggggg"));
        assert!(!is_color_format("hello world"));
        assert!(!is_color_format("rgb(10, 20)"));
    }

    #[test]
    fn test_calculate_hash_deterministic() {
        let h1 = ClipItem::calculate_hash(ClipContentType::Text, Some("hello"), None, None);
        let h2 = ClipItem::calculate_hash(ClipContentType::Text, Some("hello"), None, None);
        let h3 = ClipItem::calculate_hash(ClipContentType::Color, Some("hello"), None, None);
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);

        let files = vec!["/tmp/a.txt".to_string(), "/tmp/b.txt".to_string()];
        let hf = ClipItem::calculate_hash(ClipContentType::FileURL, None, None, Some(&files));
        assert_eq!(hf.len(), 64);
    }
}
