use crate::uri::{Authority, PathAndQuery, Scheme};

#[derive(Debug, Clone)]
pub struct Parts {
    pub scheme: Option<Scheme>,
    pub authority: Option<Authority>,
    pub path_and_query: Option<PathAndQuery>,
}
