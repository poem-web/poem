use std::fmt::{self, Debug, Formatter};

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while1},
    combinator::{eof, map, opt},
    multi::many1,
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};
use regex::bytes::Regex;
use smallvec::SmallVec;

fn longest_common_prefix(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).take_while(|(a, b)| **a == **b).count()
}

#[derive(Debug, Eq, PartialEq)]
enum RawSegment<'a> {
    Static(&'a [u8]),
    Param(&'a [u8]),
    CatchAll(&'a [u8]),
    Regex(Option<&'a [u8]>, &'a [u8]),
}

enum Segment<'a> {
    Static(&'a [u8]),
    Param(&'a [u8]),
    CatchAll(&'a [u8]),
    Regex(Option<&'a [u8]>, PathRegex),
}

fn find_slash(path: &[u8]) -> Option<usize> {
    for (i, c) in path.iter().enumerate() {
        if *c == b'/' {
            return Some(i);
        }
    }
    None
}

fn parse_path_segments(path: &[u8]) -> Option<Vec<RawSegment<'_>>> {
    let static_path = map(is_not(":*<"), RawSegment::Static);
    let catch_all = map(
        preceded(tag(b"*"), take_while1(|_| true)),
        RawSegment::CatchAll,
    );
    let param = map(
        tuple((
            tag(b":"),
            is_not(":*</"),
            opt(delimited(tag(b"<"), is_not(">"), tag(b">"))),
        )),
        |(_, name, regex)| match regex {
            Some(regex) => RawSegment::Regex(Some(name), regex),
            None => RawSegment::Param(name),
        },
    );
    let regex = map(delimited(tag(b"<"), is_not(">"), tag(b">")), |re| {
        RawSegment::Regex(None, re)
    });

    let res: IResult<&[u8], Vec<RawSegment>> =
        terminated(many1(alt((static_path, catch_all, param, regex))), eof)(path);
    res.map(|(_, segments)| segments).ok()
}

#[derive(Debug, Eq, PartialEq)]
enum NodeType {
    Root,
    Static,
    Param,
    CatchAll,
    Regex,
}

struct PathRegex {
    re_str: String,
    re: Regex,
}

impl PathRegex {
    fn new(re_bytes: &[u8]) -> Option<Self> {
        let re_str = std::str::from_utf8(re_bytes).ok()?;
        Some(PathRegex {
            re_str: re_str.to_string(),
            re: Regex::new(re_str).ok()?,
        })
    }
}

impl Debug for PathRegex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PathRegex").field(&self.re_str).finish()
    }
}

impl PartialEq for PathRegex {
    fn eq(&self, other: &Self) -> bool {
        self.re_str.eq(&other.re_str)
    }
}

impl Eq for PathRegex {}

#[derive(Debug, Eq, PartialEq)]
struct Node<T> {
    node_type: NodeType,
    name: Vec<u8>,
    children: Vec<Node<T>>,
    indices: Vec<u8>,
    re: Option<PathRegex>,
    param_child: Option<Box<Node<T>>>,
    catch_all_child: Option<Box<Node<T>>>,
    regex_child: Option<Box<Node<T>>>,
    data: Option<T>,
}

impl<T> Node<T> {
    fn find_static_child(&self, prefix: u8) -> Option<usize> {
        for i in 0..self.indices.len() {
            if self.indices[i] == prefix {
                return Some(i);
            }
        }
        None
    }

    fn insert_child(&mut self, mut segments: Vec<Segment<'_>>, data: T) -> bool {
        match segments.pop() {
            Some(segment) => match segment {
                Segment::Static(name) => self.insert_static_child(segments, name, data),
                Segment::Param(name) => self.insert_param_child(segments, name, data),
                Segment::CatchAll(name) => self.insert_catch_all_child(name, data),
                Segment::Regex(name, re) => self.insert_regex_child(segments, name, re, data),
            },
            None => self.data.replace(data).is_none(),
        }
    }

    fn insert_static_child(&mut self, segments: Vec<Segment<'_>>, name: &[u8], data: T) -> bool {
        match self.find_static_child(name[0]) {
            Some(pos) => {
                let mut child = &mut self.children[pos];
                let n = longest_common_prefix(&child.name, name);

                if n < child.name.len() {
                    // split node
                    let a = Node {
                        node_type: NodeType::Static,
                        name: child.name[n..].to_vec(),
                        children: ::std::mem::take(&mut child.children),
                        indices: std::mem::take(&mut child.indices),
                        re: None,
                        param_child: child.param_child.take(),
                        catch_all_child: child.catch_all_child.take(),
                        regex_child: None,
                        data: child.data.take(),
                    };

                    if !name[n..].is_empty() {
                        let b = Node {
                            node_type: NodeType::Static,
                            name: name[n..].to_vec(),
                            children: vec![],
                            indices: vec![],
                            re: None,
                            param_child: None,
                            catch_all_child: None,
                            regex_child: None,
                            data: None,
                        };

                        child.name = child.name[..n].to_vec();
                        child.indices = vec![a.name[0], b.name[0]];
                        child.children = vec![a, b];

                        let b = child.children.last_mut().unwrap();
                        b.insert_child(segments, data)
                    } else {
                        child.name = child.name[..n].to_vec();
                        child.indices = vec![a.name[0]];
                        child.children = vec![a];

                        child.insert_child(segments, data)
                    }
                } else if n < name.len() {
                    // add child
                    child.insert_static_child(segments, &name[n..], data)
                } else {
                    child.insert_child(segments, data)
                }
            }
            None => {
                self.children.push(Node {
                    node_type: NodeType::Static,
                    name: name.to_vec(),
                    children: vec![],
                    indices: vec![],
                    re: None,
                    param_child: None,
                    catch_all_child: None,
                    regex_child: None,
                    data: None,
                });
                self.indices.push(name[0]);
                self.children
                    .last_mut()
                    .unwrap()
                    .insert_child(segments, data)
            }
        }
    }

    fn insert_param_child(&mut self, segments: Vec<Segment<'_>>, name: &[u8], data: T) -> bool {
        let child = match &mut self.param_child {
            Some(child) => {
                child.name = name.to_vec();
                child
            }
            None => {
                self.param_child = Some(Box::new(Node {
                    node_type: NodeType::Param,
                    name: name.to_vec(),
                    children: vec![],
                    indices: vec![],
                    re: None,
                    param_child: None,
                    catch_all_child: None,
                    regex_child: None,
                    data: None,
                }));
                self.param_child.as_mut().unwrap()
            }
        };
        child.insert_child(segments, data)
    }

    fn insert_catch_all_child(&mut self, name: &[u8], data: T) -> bool {
        self.catch_all_child
            .replace(Box::new(Node {
                node_type: NodeType::CatchAll,
                name: name.to_vec(),
                children: vec![],
                indices: vec![],
                re: None,
                param_child: None,
                catch_all_child: None,
                regex_child: None,
                data: Some(data),
            }))
            .is_none()
    }

    fn insert_regex_child(
        &mut self,
        segments: Vec<Segment<'_>>,
        name: Option<&[u8]>,
        re: PathRegex,
        data: T,
    ) -> bool {
        let child = match &mut self.regex_child {
            Some(child) => {
                child.name = name.unwrap_or_default().to_vec();
                child.re = Some(re);
                child
            }
            None => {
                self.regex_child = Some(Box::new(Node {
                    node_type: NodeType::Regex,
                    name: name.unwrap_or_default().to_vec(),
                    children: vec![],
                    indices: vec![],
                    re: Some(re),
                    param_child: None,
                    catch_all_child: None,
                    regex_child: None,
                    data: None,
                }));
                self.regex_child.as_mut().unwrap()
            }
        };
        child.insert_child(segments, data)
    }

    fn matches<'a: 'b, 'b>(
        &'a self,
        path: &'b [u8],
        params: &mut SmallVec<[(&'b [u8], &'b [u8]); 8]>,
    ) -> Option<&'a T> {
        if path.is_empty() {
            return if let Some(catch_all_child) = &self.catch_all_child {
                params.push((&catch_all_child.name, path));
                catch_all_child.data.as_ref()
            } else {
                self.data.as_ref()
            };
        }

        let num_params = params.len();

        if let Some(pos) = self.find_static_child(path[0]) {
            let child = &self.children[pos];
            if path == child.name {
                if let Some(data) = child.matches(&[], params) {
                    return Some(data);
                }
            }
            if let Some(tail_path) = path.strip_prefix(child.name.as_slice()) {
                if let Some(data) = child.matches(tail_path, params) {
                    return Some(data);
                }
            }
        }

        params.truncate(num_params);
        if let Some(regex_child) = &self.regex_child {
            if let Some(captures) = regex_child.re.as_ref().unwrap().re.captures(path) {
                let value = &path[..captures[0].len()];
                if !regex_child.name.is_empty() {
                    params.push((&regex_child.name, value));
                }
                if let Some(data) = regex_child.matches(&path[value.len()..], params) {
                    return Some(data);
                }
            }
        }

        params.truncate(num_params);
        if let Some(param_child) = &self.param_child {
            let value = match find_slash(path) {
                Some(pos) => &path[..pos],
                None => path,
            };
            params.push((&param_child.name, value));
            if let Some(data) = param_child.matches(&path[value.len()..], params) {
                return Some(data);
            }
        }

        params.truncate(num_params);
        if let Some(catch_all_child) = &self.catch_all_child {
            params.push((&catch_all_child.name, path));
            return catch_all_child.data.as_ref();
        }

        None
    }
}

pub(crate) type PathParams = Vec<(String, String)>;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Matches<'a, T> {
    pub(crate) params: PathParams,
    pub(crate) data: &'a T,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct RadixTree<T> {
    root: Node<T>,
}

impl<T> Default for RadixTree<T> {
    fn default() -> Self {
        Self {
            root: Node {
                node_type: NodeType::Root,
                name: vec![],
                children: vec![],
                indices: vec![],
                re: None,
                param_child: None,
                catch_all_child: None,
                regex_child: None,
                data: None,
            },
        }
    }
}

impl<T> RadixTree<T> {
    pub(crate) fn add(&mut self, path: &str, data: T) -> bool {
        let raw_segments = match parse_path_segments(path.as_bytes()) {
            Some(raw_segments) => raw_segments,
            None => return false,
        };

        let mut segments = Vec::with_capacity(raw_segments.len());
        for raw_segment in raw_segments {
            let segment = match raw_segment {
                RawSegment::Static(value) => Segment::Static(value),
                RawSegment::Param(name) => Segment::Param(name),
                RawSegment::CatchAll(name) => Segment::CatchAll(name),
                RawSegment::Regex(name, re_bytes) => {
                    if let Some(re) = PathRegex::new(re_bytes) {
                        Segment::Regex(name, re)
                    } else {
                        return false;
                    }
                }
            };
            segments.push(segment);
        }
        segments.reverse();

        self.root.insert_child(segments, data)
    }

    pub(crate) fn matches(&self, path: &str) -> Option<Matches<T>> {
        let mut params = SmallVec::default();

        match self.root.matches(path.as_bytes(), &mut params) {
            Some(data) => {
                let mut params2 = Vec::with_capacity(params.len());
                for (name, value) in params {
                    if let (Ok(name), Ok(value)) =
                        (std::str::from_utf8(name), std::str::from_utf8(value))
                    {
                        params2.push((name.to_string(), value.to_string()));
                    }
                }
                Some(Matches {
                    params: params2,
                    data,
                })
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_longest_common_prefix() {
        assert_eq!(longest_common_prefix(b"abc", b"a"), 1);
        assert_eq!(longest_common_prefix(b"abc", b"ab"), 2);
        assert_eq!(longest_common_prefix(b"abc", b"dbc"), 0);
    }

    #[test]
    fn test_parse_path_segments() {
        assert_eq!(
            parse_path_segments(b"/a/:b/:re<\\d+>/:ui/ef/<\\d+>/*jkl"),
            Some(vec![
                RawSegment::Static(b"/a/"),
                RawSegment::Param(b"b"),
                RawSegment::Static(b"/"),
                RawSegment::Regex(Some(b"re"), b"\\d+"),
                RawSegment::Static(b"/"),
                RawSegment::Param(b"ui"),
                RawSegment::Static(b"/ef/"),
                RawSegment::Regex(None, b"\\d+"),
                RawSegment::Static(b"/"),
                RawSegment::CatchAll(b"jkl")
            ])
        );

        assert_eq!(parse_path_segments(b"/a/:"), None);
    }

    #[test]
    fn test_insert_static_child_1() {
        let mut tree = RadixTree::default();
        tree.add("/abc", 1);
        tree.add("/abcdef", 2);
        tree.add("/abcdefgh", 3);

        assert_eq!(
            tree,
            RadixTree {
                root: Node {
                    node_type: NodeType::Root,
                    name: vec![],
                    children: vec![Node {
                        node_type: NodeType::Static,
                        name: b"/abc".to_vec(),
                        children: vec![Node {
                            node_type: NodeType::Static,
                            name: b"def".to_vec(),
                            children: vec![Node {
                                node_type: NodeType::Static,
                                name: b"gh".to_vec(),
                                children: vec![],
                                indices: vec![],
                                re: None,
                                param_child: None,
                                catch_all_child: None,
                                regex_child: None,
                                data: Some(3),
                            }],
                            indices: vec![b'g'],
                            re: None,
                            param_child: None,
                            catch_all_child: None,
                            regex_child: None,
                            data: Some(2),
                        }],
                        indices: vec![b'd'],
                        re: None,
                        param_child: None,
                        catch_all_child: None,
                        regex_child: None,
                        data: Some(1)
                    }],
                    indices: vec![b'/'],
                    re: None,
                    param_child: None,
                    catch_all_child: None,
                    regex_child: None,
                    data: None,
                }
            }
        );
    }

    #[test]
    fn test_insert_static_child_2() {
        let mut tree = RadixTree::default();
        tree.add("/abcd", 1);
        tree.add("/ab1234", 2);
        tree.add("/ab1256", 3);
        tree.add("/ab125678", 4);

        assert_eq!(
            tree,
            RadixTree {
                root: Node {
                    node_type: NodeType::Root,
                    name: vec![],
                    children: vec![Node {
                        node_type: NodeType::Static,
                        name: b"/ab".to_vec(),
                        children: vec![
                            Node {
                                node_type: NodeType::Static,
                                name: b"cd".to_vec(),
                                children: vec![],
                                indices: vec![],
                                re: None,
                                param_child: None,
                                catch_all_child: None,
                                regex_child: None,
                                data: Some(1),
                            },
                            Node {
                                node_type: NodeType::Static,
                                name: b"12".to_vec(),
                                children: vec![
                                    Node {
                                        node_type: NodeType::Static,
                                        name: b"34".to_vec(),
                                        children: vec![],
                                        indices: vec![],
                                        re: None,
                                        param_child: None,
                                        catch_all_child: None,
                                        regex_child: None,
                                        data: Some(2)
                                    },
                                    Node {
                                        node_type: NodeType::Static,
                                        name: b"56".to_vec(),
                                        children: vec![Node {
                                            node_type: NodeType::Static,
                                            name: b"78".to_vec(),
                                            children: vec![],
                                            indices: vec![],
                                            re: None,
                                            param_child: None,
                                            catch_all_child: None,
                                            regex_child: None,
                                            data: Some(4)
                                        }],
                                        indices: vec![b'7'],
                                        re: None,
                                        param_child: None,
                                        catch_all_child: None,
                                        regex_child: None,
                                        data: Some(3)
                                    }
                                ],
                                indices: vec![b'3', b'5'],
                                re: None,
                                param_child: None,
                                catch_all_child: None,
                                regex_child: None,
                                data: None,
                            }
                        ],
                        indices: vec![b'c', b'1'],
                        re: None,
                        param_child: None,
                        catch_all_child: None,
                        regex_child: None,
                        data: None
                    }],
                    indices: vec![b'/'],
                    re: None,
                    param_child: None,
                    catch_all_child: None,
                    regex_child: None,
                    data: None
                }
            }
        );
    }

    #[test]
    fn test_insert_static_child_3() {
        let mut tree = RadixTree::default();
        tree.add("/abc", 1);
        tree.add("/ab", 2);
        assert_eq!(
            tree,
            RadixTree {
                root: Node {
                    node_type: NodeType::Root,
                    name: vec![],
                    children: vec![Node {
                        node_type: NodeType::Static,
                        name: b"/ab".to_vec(),
                        children: vec![Node {
                            node_type: NodeType::Static,
                            name: b"c".to_vec(),
                            children: vec![],
                            indices: vec![],
                            re: None,
                            param_child: None,
                            catch_all_child: None,
                            regex_child: None,
                            data: Some(1)
                        }],
                        indices: vec![b'c'],
                        re: None,
                        param_child: None,
                        catch_all_child: None,
                        regex_child: None,
                        data: Some(2)
                    }],
                    indices: vec![b'/'],
                    re: None,
                    param_child: None,
                    catch_all_child: None,
                    regex_child: None,
                    data: None
                }
            }
        )
    }

    #[test]
    fn test_insert_param_child() {
        let mut tree = RadixTree::default();
        tree.add("/abc/:p1", 1);
        tree.add("/abc/:p1/p2", 2);
        tree.add("/abc/:p1/:p3", 3);
        assert_eq!(
            tree,
            RadixTree {
                root: Node {
                    node_type: NodeType::Root,
                    name: vec![],
                    children: vec![Node {
                        node_type: NodeType::Static,
                        name: b"/abc/".to_vec(),
                        children: vec![],
                        indices: vec![],
                        re: None,
                        param_child: Some(Box::new(Node {
                            node_type: NodeType::Param,
                            name: b"p1".to_vec(),
                            children: vec![Node {
                                node_type: NodeType::Static,
                                name: b"/".to_vec(),
                                children: vec![Node {
                                    node_type: NodeType::Static,
                                    name: b"p2".to_vec(),
                                    children: vec![],
                                    indices: vec![],
                                    re: None,
                                    param_child: None,
                                    catch_all_child: None,
                                    regex_child: None,
                                    data: Some(2),
                                }],
                                indices: vec![b'p'],
                                re: None,
                                param_child: Some(Box::new(Node {
                                    node_type: NodeType::Param,
                                    name: b"p3".to_vec(),
                                    children: vec![],
                                    indices: vec![],
                                    re: None,
                                    param_child: None,
                                    catch_all_child: None,
                                    regex_child: None,
                                    data: Some(3)
                                })),
                                catch_all_child: None,
                                regex_child: None,
                                data: None,
                            }],
                            indices: vec![b'/'],
                            re: None,
                            param_child: None,
                            catch_all_child: None,
                            regex_child: None,
                            data: Some(1)
                        })),
                        catch_all_child: None,
                        regex_child: None,
                        data: None
                    }],
                    indices: vec![b'/'],
                    re: None,
                    param_child: None,
                    catch_all_child: None,
                    regex_child: None,
                    data: None
                }
            }
        )
    }

    #[test]
    fn test_catch_all_child_1() {
        let mut tree = RadixTree::default();
        tree.add("/abc/*p1", 1);
        tree.add("/ab/de", 2);
        assert_eq!(
            tree,
            RadixTree {
                root: Node {
                    node_type: NodeType::Root,
                    name: vec![],
                    children: vec![Node {
                        node_type: NodeType::Static,
                        name: b"/ab".to_vec(),
                        children: vec![
                            Node {
                                node_type: NodeType::Static,
                                name: b"c/".to_vec(),
                                children: vec![],
                                indices: vec![],
                                re: None,
                                param_child: None,
                                catch_all_child: Some(Box::new(Node {
                                    node_type: NodeType::CatchAll,
                                    name: b"p1".to_vec(),
                                    children: vec![],
                                    indices: vec![],
                                    re: None,
                                    param_child: None,
                                    catch_all_child: None,
                                    regex_child: None,
                                    data: Some(1)
                                })),
                                regex_child: None,
                                data: None
                            },
                            Node {
                                node_type: NodeType::Static,
                                name: b"/de".to_vec(),
                                children: vec![],
                                indices: vec![],
                                re: None,
                                param_child: None,
                                catch_all_child: None,
                                regex_child: None,
                                data: Some(2)
                            }
                        ],
                        indices: vec![b'c', b'/'],
                        re: None,
                        param_child: None,
                        catch_all_child: None,
                        regex_child: None,
                        data: None
                    }],
                    indices: vec![b'/'],
                    re: None,
                    param_child: None,
                    catch_all_child: None,
                    regex_child: None,
                    data: None
                }
            }
        );
    }

    #[test]
    fn test_catch_all_child_2() {
        let mut tree = RadixTree::default();
        tree.add("*p1", 1);
        assert_eq!(
            tree,
            RadixTree {
                root: Node {
                    node_type: NodeType::Root,
                    name: vec![],
                    children: vec![],
                    indices: vec![],
                    re: None,
                    param_child: None,
                    catch_all_child: Some(Box::new(Node {
                        node_type: NodeType::CatchAll,
                        name: b"p1".to_vec(),
                        children: vec![],
                        indices: vec![],
                        re: None,
                        param_child: None,
                        catch_all_child: None,
                        regex_child: None,
                        data: Some(1)
                    })),
                    regex_child: None,
                    data: None
                }
            }
        );
    }

    #[test]
    fn test_insert_regex_child() {
        let mut tree = RadixTree::default();
        tree.add("/abc/<\\d+>/def", 1);
        tree.add("/abc/def/:name<\\d+>", 2);

        assert_eq!(
            tree,
            RadixTree {
                root: Node {
                    node_type: NodeType::Root,
                    name: vec![],
                    children: vec![Node {
                        node_type: NodeType::Static,
                        name: b"/abc/".to_vec(),
                        children: vec![Node {
                            node_type: NodeType::Static,
                            name: b"def/".to_vec(),
                            children: vec![],
                            indices: vec![],
                            re: None,
                            param_child: None,
                            catch_all_child: None,
                            regex_child: Some(Box::new(Node {
                                node_type: NodeType::Regex,
                                name: b"name".to_vec(),
                                children: vec![],
                                indices: vec![],
                                re: Some(PathRegex::new(b"\\d+").unwrap()),
                                param_child: None,
                                catch_all_child: None,
                                regex_child: None,
                                data: Some(2),
                            })),
                            data: None
                        }],
                        indices: vec![b'd'],
                        re: None,
                        param_child: None,
                        catch_all_child: None,
                        regex_child: Some(Box::new(Node {
                            node_type: NodeType::Regex,
                            name: vec![],
                            children: vec![Node {
                                node_type: NodeType::Static,
                                name: b"/def".to_vec(),
                                children: vec![],
                                indices: vec![],
                                re: None,
                                param_child: None,
                                catch_all_child: None,
                                regex_child: None,
                                data: Some(1),
                            }],
                            indices: vec![b'/'],
                            re: Some(PathRegex::new(b"\\d+").unwrap()),
                            param_child: None,
                            catch_all_child: None,
                            regex_child: None,
                            data: None
                        })),
                        data: None
                    }],
                    indices: vec![b'/'],
                    re: None,
                    param_child: None,
                    catch_all_child: None,
                    regex_child: None,
                    data: None
                }
            }
        );
    }

    #[test]
    fn test_add_result() {
        let mut tree = RadixTree::default();
        assert!(tree.add("/a/b", 1));
        assert!(!tree.add("/a/b", 2));
        assert!(tree.add("/a/b/:p/d", 1));
        assert!(tree.add("/a/b/c/d", 2));
        assert!(!tree.add("/a/b/:p2/d", 3));
        assert!(tree.add("/a/*p", 1));
        assert!(!tree.add("/a/*p", 2));
        assert!(tree.add("/a/b/*p", 1));
        assert!(!tree.add("/a/b/*p2", 2));
        assert!(tree.add("/k/h/<\\d>+", 1));
        assert!(!tree.add("/k/h/:name<\\d>+", 2));
    }

    fn create_url_params<I, K, V>(values: I) -> PathParams
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        values
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect()
    }

    #[test]
    fn test_matches() {
        let mut tree = RadixTree::default();
        let paths = vec![
            ("/ab/def", 1),
            ("/abc/def", 2),
            ("/abc/:p1", 3),
            ("/abc/:p1/def", 4),
            ("/abc/:p1/:p2", 5),
            ("/abc/def/*p1", 6),
            ("/a/b/c/d", 7),
            ("/a/:p1/:p2/c", 8),
            ("/*p1", 9),
            ("/abc/<\\d+>/def", 10),
            ("/kcd/:p1<\\d+>", 11),
        ];

        for (path, id) in paths {
            tree.add(path, id);
        }

        let matches = vec![
            (
                "/ab/def",
                Some(Matches {
                    params: vec![],
                    data: &1,
                }),
            ),
            (
                "/abc/def",
                Some(Matches {
                    params: vec![],
                    data: &2,
                }),
            ),
            (
                "/abc/cde",
                Some(Matches {
                    params: create_url_params(vec![("p1", "cde")]),
                    data: &3,
                }),
            ),
            (
                "/abc/cde/def",
                Some(Matches {
                    params: create_url_params(vec![("p1", "cde")]),
                    data: &4,
                }),
            ),
            (
                "/abc/cde/hjk",
                Some(Matches {
                    params: create_url_params(vec![("p1", "cde"), ("p2", "hjk")]),
                    data: &5,
                }),
            ),
            (
                "/abc/def/iop/123",
                Some(Matches {
                    params: create_url_params(vec![("p1", "iop/123")]),
                    data: &6,
                }),
            ),
            (
                "/a/b/k/c",
                Some(Matches {
                    params: create_url_params(vec![("p1", "b"), ("p2", "k")]),
                    data: &8,
                }),
            ),
            (
                "/kcd/uio",
                Some(Matches {
                    params: create_url_params(vec![("p1", "kcd/uio")]),
                    data: &9,
                }),
            ),
            (
                "/",
                Some(Matches {
                    params: create_url_params(vec![("p1", "")]),
                    data: &9,
                }),
            ),
            (
                "/abc/123/def",
                Some(Matches {
                    params: vec![],
                    data: &10,
                }),
            ),
            (
                "/kcd/567",
                Some(Matches {
                    params: create_url_params(vec![("p1", "567")]),
                    data: &11,
                }),
            ),
        ];

        for (path, res) in matches {
            assert_eq!(tree.matches(path), res);
        }
    }
}
