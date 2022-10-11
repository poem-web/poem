use std::{
    fmt::{self, Debug, Formatter},
    sync::Arc,
};

use regex::bytes::Regex;
use smallvec::SmallVec;

use crate::error::RouteError;

fn longest_common_prefix(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).take_while(|(a, b)| **a == **b).count()
}

#[derive(Debug, Eq, PartialEq)]
enum RawSegment<'a> {
    Static(&'a [u8]),
    Param(&'a [u8]),
    CatchAll(Option<&'a [u8]>),
    Regex(Option<&'a [u8]>, &'a [u8]),
}

enum Segment<'a> {
    Static(&'a [u8]),
    Param(&'a [u8]),
    CatchAll(Option<&'a [u8]>),
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

fn parse_path_segments(path: &[u8]) -> Result<Vec<RawSegment<'_>>, ()> {
    fn parse_static<'a>(path: &'a [u8], i: &mut usize) -> &'a [u8] {
        let s = *i;
        while *i < path.len() {
            match path[*i] {
                b':' | b'*' | b'<' => break,
                _ => *i += 1,
            }
        }
        &path[s..*i]
    }

    fn parse_name<'a>(path: &'a [u8], i: &mut usize) -> Result<&'a [u8], ()> {
        let s = *i;
        while *i < path.len() {
            match path[*i] {
                b'/' | b'<' | b'*' => break,
                _ => *i += 1,
            }
        }

        if !path[s..*i].is_empty() {
            Ok(&path[s..*i])
        } else {
            Err(())
        }
    }

    fn parse_re<'a>(path: &'a [u8], i: &mut usize) -> Result<&'a [u8], ()> {
        let s = *i;
        while *i < path.len() {
            match path[*i] {
                b'>' => {
                    let re = &path[s..*i];
                    *i += 1;
                    if re.is_empty() {
                        return Err(());
                    }
                    return Ok(re);
                }
                _ => *i += 1,
            }
        }
        Err(())
    }

    let mut i = 0;
    let mut segments = Vec::new();

    while i < path.len() {
        match path[i] {
            b':' => {
                i += 1;
                let name = parse_name(path, &mut i)?;
                if i < path.len() && path[i] == b'<' {
                    i += 1;
                    let re = parse_re(path, &mut i)?;
                    segments.push(RawSegment::Regex(Some(name), re));
                } else {
                    segments.push(RawSegment::Param(name));
                }
            }
            b'*' => {
                i += 1;
                let name = &path[i..];
                if name.is_empty() {
                    segments.push(RawSegment::CatchAll(None));
                } else {
                    segments.push(RawSegment::CatchAll(Some(name)));
                }
                break;
            }
            b'<' => {
                i += 1;
                let re = parse_re(path, &mut i)?;
                segments.push(RawSegment::Regex(None, re));
            }
            _ => {
                let s = parse_static(path, &mut i);
                segments.push(RawSegment::Static(s));
            }
        }
    }

    Ok(segments)
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
pub(crate) struct NodeData<T> {
    pub(crate) data: T,
    pub(crate) pattern: Arc<str>,
}

impl<T> NodeData<T> {
    #[inline]
    fn new<P>(data: T, pattern: P) -> Self
    where
        P: Into<Arc<str>>,
    {
        Self {
            data,
            pattern: pattern.into(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Node<T> {
    node_type: NodeType,
    name: Vec<u8>,
    children: Vec<Node<T>>,
    indices: Vec<u8>,
    re: Option<PathRegex>,
    param_children: Vec<Box<Node<T>>>,
    catch_all_child: Option<Box<Node<T>>>,
    regex_children: Vec<Box<Node<T>>>,
    data: Option<NodeData<T>>,
}

impl<T> Node<T> {
    fn find_static_child(&self, prefix: u8) -> Option<usize> {
        (0..self.indices.len()).find(|&i| self.indices[i] == prefix)
    }

    fn insert_child(&mut self, mut segments: Vec<Segment<'_>>, data: NodeData<T>) -> bool {
        match segments.pop() {
            Some(segment) => match segment {
                Segment::Static(name) => self.insert_static_child(segments, name, data),
                Segment::Param(name) => self.insert_param_child(segments, name, data),
                Segment::CatchAll(name) => self.insert_catch_all_child(name, data),
                Segment::Regex(name, re) => self.insert_regex_child(segments, name, re, data),
            },
            None => {
                if self.data.is_some() {
                    return false;
                }
                self.data = Some(data);
                true
            }
        }
    }

    fn insert_static_child(
        &mut self,
        segments: Vec<Segment<'_>>,
        name: &[u8],
        data: NodeData<T>,
    ) -> bool {
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
                        param_children: ::std::mem::take(&mut child.param_children),
                        catch_all_child: child.catch_all_child.take(),
                        regex_children: vec![],
                        data: child.data.take(),
                    };

                    if !name[n..].is_empty() {
                        let b = Node {
                            node_type: NodeType::Static,
                            name: name[n..].to_vec(),
                            children: vec![],
                            indices: vec![],
                            re: None,
                            param_children: vec![],
                            catch_all_child: None,
                            regex_children: vec![],
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
                    param_children: vec![],
                    catch_all_child: None,
                    regex_children: vec![],
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

    fn insert_param_child(
        &mut self,
        segments: Vec<Segment<'_>>,
        name: &[u8],
        data: NodeData<T>,
    ) -> bool {
        let child = match self
            .param_children
            .iter_mut()
            .find(|child| child.name == name)
        {
            Some(child) => child,
            None => {
                self.param_children.push(Box::new(Node {
                    node_type: NodeType::Param,
                    name: name.to_vec(),
                    children: vec![],
                    indices: vec![],
                    re: None,
                    param_children: vec![],
                    catch_all_child: None,
                    regex_children: vec![],
                    data: None,
                }));
                self.param_children.last_mut().unwrap()
            }
        };

        child.insert_child(segments, data)
    }

    fn insert_catch_all_child(&mut self, name: Option<&[u8]>, data: NodeData<T>) -> bool {
        self.catch_all_child
            .replace(Box::new(Node {
                node_type: NodeType::CatchAll,
                name: name.unwrap_or_default().to_vec(),
                children: vec![],
                indices: vec![],
                re: None,
                param_children: vec![],
                catch_all_child: None,
                regex_children: vec![],
                data: Some(data),
            }))
            .is_none()
    }

    fn insert_regex_child(
        &mut self,
        segments: Vec<Segment<'_>>,
        name: Option<&[u8]>,
        re: PathRegex,
        data: NodeData<T>,
    ) -> bool {
        let name = name.unwrap_or_default();
        let child = match self
            .regex_children
            .iter_mut()
            .find(|child| child.name == name && child.re.as_ref() == Some(&re))
        {
            Some(child) => child,
            None => {
                self.regex_children.push(Box::new(Node {
                    node_type: NodeType::Regex,
                    name: name.to_vec(),
                    children: vec![],
                    indices: vec![],
                    re: Some(re),
                    param_children: vec![],
                    catch_all_child: None,
                    regex_children: vec![],
                    data: None,
                }));
                self.regex_children.last_mut().unwrap()
            }
        };

        child.insert_child(segments, data)
    }

    fn matches<'a: 'b, 'b>(
        &'a self,
        path: &'b [u8],
        params: &mut SmallVec<[(&'b [u8], &'b [u8]); 8]>,
    ) -> Option<&'a NodeData<T>> {
        if path.is_empty() {
            return if let Some(catch_all_child) = &self.catch_all_child {
                if !catch_all_child.name.is_empty() {
                    params.push((&catch_all_child.name, path));
                }
                catch_all_child.data.as_ref()
            } else {
                self.data.as_ref()
            };
        }

        let num_params = params.len();

        if let Some(pos) = self.find_static_child(path[0]) {
            let child = &self.children[pos];
            if let Some(tail_path) = path.strip_prefix(child.name.as_slice()) {
                if let Some(data) = child.matches(tail_path, params) {
                    return Some(data);
                }
            }
        }

        for regex_children in &self.regex_children {
            params.truncate(num_params);

            if let Some(captures) = regex_children.re.as_ref().unwrap().re.captures(path) {
                let value = &path[..captures[0].len()];
                if !regex_children.name.is_empty() {
                    params.push((&regex_children.name, value));
                }
                if let Some(data) = regex_children.matches(&path[value.len()..], params) {
                    return Some(data);
                }
            }
        }

        for param_children in &self.param_children {
            params.truncate(num_params);

            let value = match find_slash(path) {
                Some(pos) => &path[..pos],
                None => path,
            };
            params.push((&param_children.name, value));
            if let Some(data) = param_children.matches(&path[value.len()..], params) {
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
    pub(crate) data: &'a NodeData<T>,
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
                param_children: vec![],
                catch_all_child: None,
                regex_children: vec![],
                data: None,
            },
        }
    }
}

impl<T> RadixTree<T> {
    pub(crate) fn add(&mut self, path: &str, data: T) -> Result<(), RouteError> {
        let raw_segments = match parse_path_segments(path.as_bytes()) {
            Ok(raw_segments) => raw_segments,
            Err(_) => return Err(RouteError::InvalidPath(path.to_string())),
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
                        return Err(RouteError::InvalidRegex {
                            path: path.to_string(),
                            regex: String::from_utf8(re_bytes.to_vec()).unwrap(),
                        });
                    }
                }
            };
            segments.push(segment);
        }
        segments.reverse();

        if self.root.insert_child(segments, NodeData::new(data, path)) {
            Ok(())
        } else {
            Err(RouteError::Duplicate(path.to_string()))
        }
    }

    pub(crate) fn matches(&self, path: &str) -> Option<Matches<T>> {
        if path.is_empty() {
            return None;
        }

        let mut params = SmallVec::default();

        match self.root.matches(path.as_bytes(), &mut params) {
            Some(data) => {
                let mut params2 = Vec::with_capacity(params.len());
                for (name, value) in params {
                    if let (Ok(name), Ok(value)) = (
                        std::str::from_utf8(name),
                        percent_encoding::percent_decode(value).decode_utf8(),
                    ) {
                        params2.push((name.to_string(), value.into_owned()));
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
            parse_path_segments(b"/a/b"),
            Ok(vec![RawSegment::Static(b"/a/b")])
        );

        assert_eq!(parse_path_segments(b""), Ok(vec![]));

        assert_eq!(
            parse_path_segments(b"/a/:v/b"),
            Ok(vec![
                RawSegment::Static(b"/a/"),
                RawSegment::Param(b"v"),
                RawSegment::Static(b"/b"),
            ])
        );

        assert_eq!(
            parse_path_segments(b"/a/*"),
            Ok(vec![RawSegment::Static(b"/a/"), RawSegment::CatchAll(None)])
        );

        assert_eq!(
            parse_path_segments(b"/a/:v"),
            Ok(vec![RawSegment::Static(b"/a/"), RawSegment::Param(b"v")])
        );

        assert_eq!(
            parse_path_segments(b"/a/:v<\\d+>"),
            Ok(vec![
                RawSegment::Static(b"/a/"),
                RawSegment::Regex(Some(b"v"), b"\\d+")
            ])
        );

        assert_eq!(parse_path_segments(b"/a/:v<\\d+"), Err(()));

        assert_eq!(
            parse_path_segments(b"*p"),
            Ok(vec![RawSegment::CatchAll(Some(b"p"))])
        );

        assert_eq!(
            parse_path_segments(b"/a/:b/:re<\\d+>/:ui/ef/<\\d+>/*jkl"),
            Ok(vec![
                RawSegment::Static(b"/a/"),
                RawSegment::Param(b"b"),
                RawSegment::Static(b"/"),
                RawSegment::Regex(Some(b"re"), b"\\d+"),
                RawSegment::Static(b"/"),
                RawSegment::Param(b"ui"),
                RawSegment::Static(b"/ef/"),
                RawSegment::Regex(None, b"\\d+"),
                RawSegment::Static(b"/"),
                RawSegment::CatchAll(Some(b"jkl"))
            ])
        );

        assert_eq!(parse_path_segments(b"/a/:"), Err(()));
    }

    #[test]
    fn test_insert_static_child_1() {
        let mut tree = RadixTree::default();
        tree.add("/abc", 1).unwrap();
        tree.add("/abcdef", 2).unwrap();
        tree.add("/abcdefgh", 3).unwrap();

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
                                param_children: vec![],
                                catch_all_child: None,
                                regex_children: vec![],
                                data: Some(NodeData::new(3, "/abcdefgh")),
                            }],
                            indices: vec![b'g'],
                            re: None,
                            param_children: vec![],
                            catch_all_child: None,
                            regex_children: vec![],
                            data: Some(NodeData::new(2, "/abcdef")),
                        }],
                        indices: vec![b'd'],
                        re: None,
                        param_children: vec![],
                        catch_all_child: None,
                        regex_children: vec![],
                        data: Some(NodeData::new(1, "/abc"))
                    }],
                    indices: vec![b'/'],
                    re: None,
                    param_children: vec![],
                    catch_all_child: None,
                    regex_children: vec![],
                    data: None,
                }
            }
        );
    }

    #[test]
    fn test_insert_static_child_2() {
        let mut tree = RadixTree::default();
        tree.add("/abcd", 1).unwrap();
        tree.add("/ab1234", 2).unwrap();
        tree.add("/ab1256", 3).unwrap();
        tree.add("/ab125678", 4).unwrap();

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
                                param_children: vec![],
                                catch_all_child: None,
                                regex_children: vec![],
                                data: Some(NodeData::new(1, "/abcd")),
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
                                        param_children: vec![],
                                        catch_all_child: None,
                                        regex_children: vec![],
                                        data: Some(NodeData::new(2, "/ab1234"))
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
                                            param_children: vec![],
                                            catch_all_child: None,
                                            regex_children: vec![],
                                            data: Some(NodeData::new(4, "/ab125678"))
                                        }],
                                        indices: vec![b'7'],
                                        re: None,
                                        param_children: vec![],
                                        catch_all_child: None,
                                        regex_children: vec![],
                                        data: Some(NodeData::new(3, "/ab1256"))
                                    }
                                ],
                                indices: vec![b'3', b'5'],
                                re: None,
                                param_children: vec![],
                                catch_all_child: None,
                                regex_children: vec![],
                                data: None,
                            }
                        ],
                        indices: vec![b'c', b'1'],
                        re: None,
                        param_children: vec![],
                        catch_all_child: None,
                        regex_children: vec![],
                        data: None
                    }],
                    indices: vec![b'/'],
                    re: None,
                    param_children: vec![],
                    catch_all_child: None,
                    regex_children: vec![],
                    data: None
                }
            }
        );
    }

    #[test]
    fn test_insert_static_child_3() {
        let mut tree = RadixTree::default();
        tree.add("/abc", 1).unwrap();
        tree.add("/ab", 2).unwrap();
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
                            param_children: vec![],
                            catch_all_child: None,
                            regex_children: vec![],
                            data: Some(NodeData::new(1, "/abc"))
                        }],
                        indices: vec![b'c'],
                        re: None,
                        param_children: vec![],
                        catch_all_child: None,
                        regex_children: vec![],
                        data: Some(NodeData::new(2, "/ab"))
                    }],
                    indices: vec![b'/'],
                    re: None,
                    param_children: vec![],
                    catch_all_child: None,
                    regex_children: vec![],
                    data: None
                }
            }
        )
    }

    #[test]
    fn test_insert_param_child() {
        let mut tree = RadixTree::default();
        tree.add("/abc/:p1", 1).unwrap();
        tree.add("/abc/:p1/p2", 2).unwrap();
        tree.add("/abc/:p1/:p3", 3).unwrap();
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
                        param_children: vec![Box::new(Node {
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
                                    param_children: vec![],
                                    catch_all_child: None,
                                    regex_children: vec![],
                                    data: Some(NodeData::new(2, "/abc/:p1/p2"))
                                }],
                                indices: vec![b'p'],
                                re: None,
                                param_children: vec![Box::new(Node {
                                    node_type: NodeType::Param,
                                    name: b"p3".to_vec(),
                                    children: vec![],
                                    indices: vec![],
                                    re: None,
                                    param_children: vec![],
                                    catch_all_child: None,
                                    regex_children: vec![],
                                    data: Some(NodeData::new(3, "/abc/:p1/:p3"))
                                })],
                                catch_all_child: None,
                                regex_children: vec![],
                                data: None,
                            }],
                            indices: vec![b'/'],
                            re: None,
                            param_children: vec![],
                            catch_all_child: None,
                            regex_children: vec![],
                            data: Some(NodeData::new(1, "/abc/:p1"))
                        })],
                        catch_all_child: None,
                        regex_children: vec![],
                        data: None
                    }],
                    indices: vec![b'/'],
                    re: None,
                    param_children: vec![],
                    catch_all_child: None,
                    regex_children: vec![],
                    data: None
                }
            }
        )
    }

    #[test]
    fn test_catch_all_child_1() {
        let mut tree = RadixTree::default();
        tree.add("/abc/*p1", 1).unwrap();
        tree.add("/ab/de", 2).unwrap();
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
                                param_children: vec![],
                                catch_all_child: Some(Box::new(Node {
                                    node_type: NodeType::CatchAll,
                                    name: b"p1".to_vec(),
                                    children: vec![],
                                    indices: vec![],
                                    re: None,
                                    param_children: vec![],
                                    catch_all_child: None,
                                    regex_children: vec![],
                                    data: Some(NodeData::new(1, "/abc/*p1"))
                                })),
                                regex_children: vec![],
                                data: None
                            },
                            Node {
                                node_type: NodeType::Static,
                                name: b"/de".to_vec(),
                                children: vec![],
                                indices: vec![],
                                re: None,
                                param_children: vec![],
                                catch_all_child: None,
                                regex_children: vec![],
                                data: Some(NodeData::new(2, "/ab/de"))
                            }
                        ],
                        indices: vec![b'c', b'/'],
                        re: None,
                        param_children: vec![],
                        catch_all_child: None,
                        regex_children: vec![],
                        data: None
                    }],
                    indices: vec![b'/'],
                    re: None,
                    param_children: vec![],
                    catch_all_child: None,
                    regex_children: vec![],
                    data: None
                }
            }
        );
    }

    #[test]
    fn test_catch_all_child_2() {
        let mut tree = RadixTree::default();
        tree.add("*p1", 1).unwrap();
        assert_eq!(
            tree,
            RadixTree {
                root: Node {
                    node_type: NodeType::Root,
                    name: vec![],
                    children: vec![],
                    indices: vec![],
                    re: None,
                    param_children: vec![],
                    catch_all_child: Some(Box::new(Node {
                        node_type: NodeType::CatchAll,
                        name: b"p1".to_vec(),
                        children: vec![],
                        indices: vec![],
                        re: None,
                        param_children: vec![],
                        catch_all_child: None,
                        regex_children: vec![],
                        data: Some(NodeData::new(1, "*p1"))
                    })),
                    regex_children: vec![],
                    data: None
                }
            }
        );
    }

    #[test]
    fn test_insert_regex_child() {
        let mut tree = RadixTree::default();
        tree.add("/abc/<\\d+>/def", 1).unwrap();
        tree.add("/abc/def/:name<\\d+>", 2).unwrap();

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
                            param_children: vec![],
                            catch_all_child: None,
                            regex_children: vec![Box::new(Node {
                                node_type: NodeType::Regex,
                                name: b"name".to_vec(),
                                children: vec![],
                                indices: vec![],
                                re: Some(PathRegex::new(b"\\d+").unwrap()),
                                param_children: vec![],
                                catch_all_child: None,
                                regex_children: vec![],
                                data: Some(NodeData::new(2, "/abc/def/:name<\\d+>"))
                            })],
                            data: None
                        }],
                        indices: vec![b'd'],
                        re: None,
                        param_children: vec![],
                        catch_all_child: None,
                        regex_children: vec![Box::new(Node {
                            node_type: NodeType::Regex,
                            name: vec![],
                            children: vec![Node {
                                node_type: NodeType::Static,
                                name: b"/def".to_vec(),
                                children: vec![],
                                indices: vec![],
                                re: None,
                                param_children: vec![],
                                catch_all_child: None,
                                regex_children: vec![],
                                data: Some(NodeData::new(1, "/abc/<\\d+>/def"))
                            }],
                            indices: vec![b'/'],
                            re: Some(PathRegex::new(b"\\d+").unwrap()),
                            param_children: vec![],
                            catch_all_child: None,
                            regex_children: vec![],
                            data: None
                        })],
                        data: None
                    }],
                    indices: vec![b'/'],
                    re: None,
                    param_children: vec![],
                    catch_all_child: None,
                    regex_children: vec![],
                    data: None
                }
            }
        );
    }

    #[test]
    fn test_add_result() {
        let mut tree = RadixTree::default();
        assert!(tree.add("/a/b", 1).is_ok());
        assert!(tree.add("/a/b", 2).is_err());
        assert!(tree.add("/a/b/:p/d", 1).is_ok());
        assert!(tree.add("/a/b/c/d", 2).is_ok());
        assert!(tree.add("/a/b/:p2/d", 3).is_ok());
        assert!(tree.add("/a/*p", 1).is_ok());
        assert!(tree.add("/a/*p", 2).is_err());
        assert!(tree.add("/a/b/*p", 1).is_ok());
        assert!(tree.add("/a/b/*p2", 2).is_err());
        assert!(tree.add("/k/h/<\\d>+", 1).is_ok());
        assert!(tree.add("/k/h/:name<\\d>+", 2).is_ok());
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
            tree.add(path, id).unwrap();
        }

        let matches = vec![
            ("/ab/def", Some((vec![], NodeData::new(1, "/ab/def")))),
            ("/abc/def", Some((vec![], NodeData::new(2, "/abc/def")))),
            (
                "/abc/cde",
                Some((
                    create_url_params(vec![("p1", "cde")]),
                    NodeData::new(3, "/abc/:p1"),
                )),
            ),
            (
                "/abc/cde/def",
                Some((
                    create_url_params(vec![("p1", "cde")]),
                    NodeData::new(4, "/abc/:p1/def"),
                )),
            ),
            (
                "/abc/cde/hjk",
                Some((
                    create_url_params(vec![("p1", "cde"), ("p2", "hjk")]),
                    NodeData::new(5, "/abc/:p1/:p2"),
                )),
            ),
            (
                "/abc/def/iop/123",
                Some((
                    create_url_params(vec![("p1", "iop/123")]),
                    NodeData::new(6, "/abc/def/*p1"),
                )),
            ),
            (
                "/a/b/k/c",
                Some((
                    create_url_params(vec![("p1", "b"), ("p2", "k")]),
                    NodeData::new(8, "/a/:p1/:p2/c"),
                )),
            ),
            (
                "/kcd/uio",
                Some((
                    create_url_params(vec![("p1", "kcd/uio")]),
                    NodeData::new(9, "/*p1"),
                )),
            ),
            (
                "/",
                Some((
                    create_url_params(vec![("p1", "")]),
                    NodeData::new(9, "/*p1"),
                )),
            ),
            (
                "/abc/123/def",
                Some((vec![], NodeData::new(10, "/abc/<\\d+>/def"))),
            ),
            (
                "/kcd/567",
                Some((
                    create_url_params(vec![("p1", "567")]),
                    NodeData::new(11, "/kcd/:p1<\\d+>"),
                )),
            ),
        ];

        for (path, mut res) in matches {
            assert_eq!(
                tree.matches(path),
                res.as_mut().map(|(params, data)| Matches {
                    params: std::mem::take(params),
                    data
                })
            );
        }
    }

    #[test]
    fn test_match_priority() {
        let mut tree = RadixTree::default();
        tree.add("/a/bc", 1).unwrap();
        tree.add("/a/*path", 2).unwrap();

        let matches = tree.matches("/a/123");
        assert_eq!(matches.unwrap().data.data, 2);

        tree.add("/a/:id", 3).unwrap();
        let matches = tree.matches("/a/123");
        assert_eq!(matches.unwrap().data.data, 3);

        tree.add("/a/:id<\\d+>", 4).unwrap();
        let matches = tree.matches("/a/123");
        assert_eq!(matches.unwrap().data.data, 4);

        tree.add("/a/123", 5).unwrap();
        let matches = tree.matches("/a/123");
        assert_eq!(matches.unwrap().data.data, 5);
    }

    #[test]
    fn test_catch_all_priority_in_sub_path() {
        let mut tree = RadixTree::default();
        tree.add("/a/*path", 1).unwrap();

        let matches = tree.matches("/a/b/c/123");
        assert_eq!(matches.unwrap().data.data, 1);

        tree.add("/a/b/*path", 2).unwrap();
        let matches = tree.matches("/a/b/c/123");
        assert_eq!(matches.unwrap().data.data, 2);

        tree.add("/a/b/c/*path", 3).unwrap();
        let matches = tree.matches("/a/b/c/123");
        assert_eq!(matches.unwrap().data.data, 3);
    }

    #[test]
    fn test_issue_275() {
        let mut tree = RadixTree::default();
        tree.add("/:id1/a", 1).unwrap();
        tree.add("/:id2/b", 2).unwrap();

        let matches = tree.matches("/abc/a").unwrap();
        assert_eq!(matches.data.data, 1);
        assert_eq!(matches.params.len(), 1);
        assert_eq!(matches.params[0].0, "id1");
        assert_eq!(matches.params[0].1, "abc");

        let matches = tree.matches("/def/b").unwrap();
        assert_eq!(matches.data.data, 2);
        assert_eq!(matches.params.len(), 1);
        assert_eq!(matches.params[0].0, "id2");
        assert_eq!(matches.params[0].1, "def");
    }

    #[test]
    fn test_percent_decoded() {
        let mut tree = RadixTree::default();
        tree.add("/a/:id", 1).unwrap();

        let matches = tree.matches("/a/abc").unwrap();
        assert_eq!(matches.data.data, 1);
        assert_eq!(matches.params[0].0, "id");
        assert_eq!(matches.params[0].1, "abc");

        let matches = tree.matches("/a/%E4%BD%A0%E5%A5%BD").unwrap();
        assert_eq!(matches.data.data, 1);
        assert_eq!(matches.params[0].0, "id");
        assert_eq!(matches.params[0].1, "你好");
    }
}
