fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else if !path.starts_with('/') {
        format!("/{path}")
    } else {
        path.to_string()
    }
}

/// Converts path parameters from poem's `:name` syntax to OpenAPI's `{name}` syntax.
#[doc(hidden)]
pub fn convert_to_oai_path(path: &str) -> String {
    let mut result = String::new();
    let has_trailing_slash = path.ends_with('/') && path.len() > 1;

    for segment in path.split('/') {
        if segment.is_empty() {
            continue;
        }

        result.push('/');
        if let Some(var) = segment.strip_prefix(':') {
            result.push('{');
            result.push_str(var);
            result.push('}');
        } else {
            result.push_str(segment);
        }
    }

    if result.is_empty() {
        "/".to_string()
    } else {
        if has_trailing_slash {
            result.push('/');
        }
        result
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

    #[test]
    fn test_convert_to_oai_path() {
        // Basic paths without parameters
        assert_eq!(convert_to_oai_path("/hello"), "/hello");
        assert_eq!(convert_to_oai_path("/hello/world"), "/hello/world");

        // Paths with parameters
        assert_eq!(convert_to_oai_path("/hello/:name"), "/hello/{name}");
        assert_eq!(
            convert_to_oai_path("/hello/:name/:surname"),
            "/hello/{name}/{surname}"
        );
        assert_eq!(
            convert_to_oai_path("/users/:id/posts/:post_id"),
            "/users/{id}/posts/{post_id}"
        );

        // Mixed paths
        assert_eq!(
            convert_to_oai_path("/api/v1/:tenant/users"),
            "/api/v1/{tenant}/users"
        );

        // Edge cases
        assert_eq!(convert_to_oai_path("/"), "/");
        assert_eq!(convert_to_oai_path(""), "/");
        assert_eq!(convert_to_oai_path("/:id"), "/{id}");

        // Trailing slashes should be preserved
        assert_eq!(convert_to_oai_path("/hello/"), "/hello/");
        assert_eq!(convert_to_oai_path("/hello/:name/"), "/hello/{name}/");
    }
}
