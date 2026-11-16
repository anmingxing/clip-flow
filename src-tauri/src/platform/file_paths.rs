use std::path::Path;
use url::Url;

pub fn normalize_file_paths(files: Vec<String>) -> Vec<String> {
    files
        .into_iter()
        .filter_map(|file| {
            if file.starts_with("file:") {
                Url::parse(&file)
                    .ok()?
                    .to_file_path()
                    .ok()?
                    .into_os_string()
                    .into_string()
                    .ok()
            } else if Path::new(&file).is_absolute() {
                Some(file)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(any(target_os = "linux", test))]
pub fn file_urls(paths: &[String]) -> Option<Vec<String>> {
    paths
        .iter()
        .map(|path| Url::from_file_path(path).ok().map(String::from))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_list_round_trip_preserves_spaces_unicode_and_reserved_characters() {
        let root = std::env::temp_dir();
        let paths = vec![
            root.join("中文 a#b%.txt").to_str().unwrap().to_owned(),
            root.join("folder").to_str().unwrap().to_owned(),
        ];
        let uris = file_urls(&paths).unwrap();
        assert!(uris[0].contains("%23"));
        assert_eq!(normalize_file_paths(uris), paths);
    }

    #[test]
    fn ignores_remote_urls_comments_and_relative_paths() {
        assert!(normalize_file_paths(vec![
            "https://example.com/a".into(),
            "#comment".into(),
            "relative.txt".into()
        ])
        .is_empty());
        assert!(file_urls(&["relative.txt".into()]).is_none());
    }
}
