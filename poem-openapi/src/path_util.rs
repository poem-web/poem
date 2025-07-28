fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else if !path.starts_with('/') {
        format!("/{path}")
    } else {
        path.to_string()
    }
}

#[doc(hidden)]
pub fn join_path(base: &str, path: &str) -> String {
    let base = normalize_path(base);
    let path = normalize_path(path);

    if path == "/" {
        return base;
    }

    if base.ends_with('/') {
        base + path.trim_start_matches('/')
    } else {
        base + &path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_path() {
        assert_eq!(join_path("", "/abc"), "/abc");
        assert_eq!(join_path("/abc", "/def"), "/abc/def");
        assert_eq!(join_path("/abc", "def"), "/abc/def");
        assert_eq!(join_path("abc/def", "ghi"), "/abc/def/ghi");
        assert_eq!(join_path("/", "/ghi"), "/ghi");
        assert_eq!(join_path("/", "/"), "/");
        assert_eq!(join_path("/abc", ""), "/abc");
        assert_eq!(join_path("", ""), "/");
        assert_eq!(join_path("/abc/", "/"), "/abc/");
        assert_eq!(join_path("/abc/", "/def"), "/abc/def");
        assert_eq!(join_path("/abc/", "/def/"), "/abc/def/");
    }
}
