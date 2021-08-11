mod nfa;

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ops::Index;

use nfa::{CharacterClass, NFA};

#[derive(Clone, Eq, Debug)]
struct Metadata {
    statics: u32,
    dynamics: u32,
    wildcards: u32,
    param_names: Vec<String>,
}

impl Metadata {
    pub(crate) fn new() -> Self {
        Self {
            statics: 0,
            dynamics: 0,
            wildcards: 0,
            param_names: Vec::new(),
        }
    }
}

impl Ord for Metadata {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.statics > other.statics {
            Ordering::Greater
        } else if self.statics < other.statics {
            Ordering::Less
        } else if self.dynamics > other.dynamics {
            Ordering::Greater
        } else if self.dynamics < other.dynamics {
            Ordering::Less
        } else if self.wildcards > other.wildcards {
            Ordering::Greater
        } else if self.wildcards < other.wildcards {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}

impl PartialOrd for Metadata {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Metadata {
    fn eq(&self, other: &Self) -> bool {
        self.statics == other.statics
            && self.dynamics == other.dynamics
            && self.wildcards == other.wildcards
    }
}

/// Router parameters.
#[derive(PartialEq, Clone, Debug, Default)]
pub(crate) struct Params(pub(crate) Vec<(String, String)>);

impl Index<&str> for Params {
    type Output = str;

    fn index(&self, name: &str) -> &Self::Output {
        self.0
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, value)| value.as_str())
            .unwrap()
    }
}

/// The result of a successful match returned by `Router::recognize`.
#[derive(Debug)]
pub(crate) struct Match<T> {
    /// Return the endpoint handler.
    pub(crate) handler: T,
    /// Return the params.
    pub(crate) params: Params,
}

/// Recognizes URL patterns with support for dynamic and wildcard segments.
#[derive(Clone, Debug)]
pub(crate) struct Router<T> {
    nfa: NFA<Metadata>,
    handlers: BTreeMap<usize, T>,
}

fn segments(route: &str) -> Vec<(Option<char>, &str)> {
    let predicate = |c| c == '.' || c == '/';

    let mut segments = vec![];
    let mut segment_start = 0;

    while segment_start < route.len() {
        let segment_end = route[segment_start + 1..]
            .find(predicate)
            .map(|i| i + segment_start + 1)
            .unwrap_or_else(|| route.len());
        let potential_sep = route.chars().nth(segment_start);
        let sep_and_segment = match potential_sep {
            Some(sep) if predicate(sep) => (Some(sep), &route[segment_start + 1..segment_end]),
            _ => (None, &route[segment_start..segment_end]),
        };

        segments.push(sep_and_segment);
        segment_start = segment_end;
    }

    segments
}

impl<T> Router<T> {
    /// Create a new instance of `Router`.
    pub(crate) fn new() -> Self {
        Self {
            nfa: NFA::new(),
            handlers: BTreeMap::new(),
        }
    }

    /// Add a route to the router.
    pub(crate) fn add(&mut self, mut route: &str, dest: T) {
        if !route.is_empty() && route.as_bytes()[0] == b'/' {
            route = &route[1..];
        }

        let nfa = &mut self.nfa;
        let mut state = 0;
        let mut metadata = Metadata::new();

        for (separator, segment) in segments(route) {
            if let Some(separator) = separator {
                state = nfa.put(state, CharacterClass::valid_char(separator));
            }

            if !segment.is_empty() && segment.as_bytes()[0] == b':' {
                state = process_dynamic_segment(nfa, state);
                metadata.dynamics += 1;
                metadata.param_names.push(segment[1..].to_string());
            } else if !segment.is_empty() && segment.as_bytes()[0] == b'*' {
                state = process_star_state(nfa, state);
                metadata.wildcards += 1;
                metadata.param_names.push(segment[1..].to_string());
            } else {
                state = process_static_segment(segment, nfa, state);
                metadata.statics += 1;
            }
        }

        nfa.acceptance(state);
        nfa.metadata(state, metadata);
        self.handlers.insert(state, dest);
    }

    /// Match a route on the router.
    pub(crate) fn recognize(&self, mut path: &str) -> Result<Match<&T>, String> {
        if !path.is_empty() && path.as_bytes()[0] == b'/' {
            path = &path[1..];
        }

        let nfa = &self.nfa;
        let result = nfa.process(path, |index| nfa.get(index).metadata.as_ref().unwrap());

        match result {
            Ok(nfa_match) => {
                let mut params = Params::default();
                let state = &nfa.get(nfa_match.state);
                let metadata = state.metadata.as_ref().unwrap();
                let param_names = metadata.param_names.clone();

                for (i, capture) in nfa_match.captures.iter().enumerate() {
                    if !param_names[i].is_empty() {
                        params
                            .0
                            .push((param_names[i].to_string(), capture.to_string()));
                    }
                }

                let handler = self.handlers.get(&nfa_match.state).unwrap();
                Ok(Match { handler, params })
            }
            Err(str) => Err(str),
        }
    }
}

impl<T> Default for Router<T> {
    fn default() -> Self {
        Self::new()
    }
}

fn process_static_segment<T>(segment: &str, nfa: &mut NFA<T>, mut state: usize) -> usize {
    for char in segment.chars() {
        state = nfa.put(state, CharacterClass::valid_char(char));
    }

    state
}

fn process_dynamic_segment<T>(nfa: &mut NFA<T>, mut state: usize) -> usize {
    state = nfa.put(state, CharacterClass::invalid_char('/'));
    nfa.put_state(state, state);
    nfa.start_capture(state);
    nfa.end_capture(state);

    state
}

fn process_star_state<T>(nfa: &mut NFA<T>, mut state: usize) -> usize {
    state = nfa.put(state, CharacterClass::any());
    nfa.put_state(state, state);
    nfa.start_capture(state);
    nfa.end_capture(state);

    state
}

#[cfg(test)]
mod tests {
    use super::{Params, Router};

    #[test]
    fn basic_router() {
        let mut router = Router::new();

        router.add("/thomas", "Thomas".to_string());
        router.add("/tom", "Tom".to_string());
        router.add("/wycats", "Yehuda".to_string());

        let m = router.recognize("/thomas").unwrap();

        assert_eq!(*m.handler, "Thomas".to_string());
        assert_eq!(m.params, Params::default());
    }

    #[test]
    fn root_router() {
        let mut router = Router::new();
        router.add("/", 10);
        assert_eq!(*router.recognize("/").unwrap().handler, 10)
    }

    #[test]
    fn empty_path() {
        let mut router = Router::new();
        router.add("/", 12);
        assert_eq!(*router.recognize("").unwrap().handler, 12)
    }

    #[test]
    fn empty_route() {
        let mut router = Router::new();
        router.add("", 12);
        assert_eq!(*router.recognize("/").unwrap().handler, 12)
    }

    #[test]
    fn ambiguous_router() {
        let mut router = Router::new();

        router.add("/posts/new", "new".to_string());
        router.add("/posts/:id", "id".to_string());

        let id = router.recognize("/posts/1").unwrap();

        assert_eq!(*id.handler, "id".to_string());
        assert_eq!(id.params, params("id", "1"));

        let new = router.recognize("/posts/new").unwrap();
        assert_eq!(*new.handler, "new".to_string());
        assert_eq!(new.params, Params::default());
    }

    #[test]
    fn ambiguous_router_b() {
        let mut router = Router::new();

        router.add("/posts/:id", "id".to_string());
        router.add("/posts/new", "new".to_string());

        let id = router.recognize("/posts/1").unwrap();

        assert_eq!(*id.handler, "id".to_string());
        assert_eq!(id.params, params("id", "1"));

        let new = router.recognize("/posts/new").unwrap();
        assert_eq!(*new.handler, "new".to_string());
        assert_eq!(new.params, Params::default());
    }

    #[test]
    fn multiple_params() {
        let mut router = Router::new();

        router.add("/posts/:post_id/comments/:id", "comment".to_string());
        router.add("/posts/:post_id/comments", "comments".to_string());

        let com = router.recognize("/posts/12/comments/100").unwrap();
        let coms = router.recognize("/posts/12/comments").unwrap();

        assert_eq!(*com.handler, "comment".to_string());
        assert_eq!(com.params, two_params("post_id", "12", "id", "100"));

        assert_eq!(*coms.handler, "comments".to_string());
        assert_eq!(coms.params, params("post_id", "12"));
        assert_eq!(coms.params["post_id"], "12".to_string());
    }

    #[test]
    fn wildcard() {
        let mut router = Router::new();

        router.add("*foo", "test".to_string());
        router.add("/bar/*foo", "test2".to_string());

        let m = router.recognize("/test").unwrap();
        assert_eq!(*m.handler, "test".to_string());
        assert_eq!(m.params, params("foo", "test"));

        let m = router.recognize("/foo/bar").unwrap();
        assert_eq!(*m.handler, "test".to_string());
        assert_eq!(m.params, params("foo", "foo/bar"));

        let m = router.recognize("/bar/foo").unwrap();
        assert_eq!(*m.handler, "test2".to_string());
        assert_eq!(m.params, params("foo", "foo"));
    }

    #[test]
    fn wildcard_colon() {
        let mut router = Router::new();

        router.add("/a/*b", "ab".to_string());
        router.add("/a/*b/c", "abc".to_string());
        router.add("/a/*b/c/:d", "abcd".to_string());

        let m = router.recognize("/a/foo").unwrap();
        assert_eq!(*m.handler, "ab".to_string());
        assert_eq!(m.params, params("b", "foo"));

        let m = router.recognize("/a/foo/bar").unwrap();
        assert_eq!(*m.handler, "ab".to_string());
        assert_eq!(m.params, params("b", "foo/bar"));

        let m = router.recognize("/a/foo/c").unwrap();
        assert_eq!(*m.handler, "abc".to_string());
        assert_eq!(m.params, params("b", "foo"));

        let m = router.recognize("/a/foo/bar/c").unwrap();
        assert_eq!(*m.handler, "abc".to_string());
        assert_eq!(m.params, params("b", "foo/bar"));

        let m = router.recognize("/a/foo/c/baz").unwrap();
        assert_eq!(*m.handler, "abcd".to_string());
        assert_eq!(m.params, two_params("b", "foo", "d", "baz"));

        let m = router.recognize("/a/foo/bar/c/baz").unwrap();
        assert_eq!(*m.handler, "abcd".to_string());
        assert_eq!(m.params, two_params("b", "foo/bar", "d", "baz"));

        let m = router.recognize("/a/foo/bar/c/baz/bay").unwrap();
        assert_eq!(*m.handler, "ab".to_string());
        assert_eq!(m.params, params("b", "foo/bar/c/baz/bay"));
    }

    #[test]
    fn unnamed_parameters() {
        let mut router = Router::new();

        router.add("/foo/:/bar", "test".to_string());
        router.add("/foo/:bar/*", "test2".to_string());
        let m = router.recognize("/foo/test/bar").unwrap();
        assert_eq!(*m.handler, "test");
        assert_eq!(m.params, Params::default());

        let m = router.recognize("/foo/test/blah").unwrap();
        assert_eq!(*m.handler, "test2");
        assert_eq!(m.params, params("bar", "test"));
    }

    fn params(key: &str, val: &str) -> Params {
        let mut params = Params::default();
        params.0.push((key.to_string(), val.to_string()));
        params
    }

    fn two_params(k1: &str, v1: &str, k2: &str, v2: &str) -> Params {
        let mut params = Params::default();
        params.0.push((k1.to_string(), v1.to_string()));
        params.0.push((k2.to_string(), v2.to_string()));
        params
    }

    #[test]
    fn dot() {
        let mut router = Router::new();
        router.add("/1/baz.:wibble", ());
        router.add("/2/:bar.baz", ());
        router.add("/3/:dynamic.:extension", ());
        router.add("/4/static.static", ());

        let m = router.recognize("/1/baz.jpg").unwrap();
        assert_eq!(m.params, params("wibble", "jpg"));

        let m = router.recognize("/2/test.baz").unwrap();
        assert_eq!(m.params, params("bar", "test"));

        let m = router.recognize("/3/any.thing").unwrap();
        assert_eq!(m.params, two_params("dynamic", "any", "extension", "thing"));

        let m = router.recognize("/3/this.performs.a.greedy.match").unwrap();
        assert_eq!(
            m.params,
            two_params("dynamic", "this.performs.a.greedy", "extension", "match")
        );

        let m = router.recognize("/4/static.static").unwrap();
        assert_eq!(m.params, Params::default());

        let m = router.recognize("/4/static/static");
        assert!(m.is_err());

        let m = router.recognize("/4.static.static");
        assert!(m.is_err());
    }
}
