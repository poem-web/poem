use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Feature {
    location: Location,
    name: String,
}

#[derive(Debug, Deserialize)]
struct Location {
    latitude: i32,
    longitude: i32,
}

#[allow(dead_code)]
pub fn load() -> Vec<crate::Feature> {
    let decoded: Vec<Feature> = serde_json::from_slice(include_bytes!("route_guide_db.json"))
        .expect("failed to deserialize features");

    decoded
        .into_iter()
        .map(|feature| crate::Feature {
            name: feature.name,
            location: Some(crate::Point {
                longitude: feature.location.longitude,
                latitude: feature.location.latitude,
            }),
        })
        .collect()
}
