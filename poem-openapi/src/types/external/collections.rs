// Expand these hot loops directly to preserve the original allocation and
// iterator optimizations without relying on cross-function inlining.
macro_rules! sequence_to_json {
    ($iter:expr, $len:expr) => {{
        let mut values = Vec::with_capacity($len);
        for item in $iter {
            if let Some(value) = item.to_json() {
                values.push(value);
            }
        }
        Some(serde_json::Value::Array(values))
    }};
}

macro_rules! map_to_json {
    ($iter:expr) => {{
        let mut map = serde_json::Map::new();
        for (name, value) in $iter {
            if let Some(value) = value.to_json() {
                map.insert(name.to_string(), value);
            }
        }
        Some(serde_json::Value::Object(map))
    }};
}

pub(super) use map_to_json;
pub(super) use sequence_to_json;
