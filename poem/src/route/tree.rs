use smallvec::SmallVec;

fn longest_common_prefix(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).take_while(|(a, b)| **a == **b).count()
}

#[derive(Debug, Eq, PartialEq)]
enum Segment<'a> {
    Static(&'a [u8]),
    Param(&'a [u8]),
    CatchAll(&'a [u8]),
}

fn find_slash(path: &[u8]) -> Option<usize> {
    path.iter()
        .copied()
        .enumerate()
        .find(|(_, c)| *c == b'/')
        .map(|(pos, _)| pos)
}

fn find_wildcard(path: &[u8]) -> Option<usize> {
    path.iter()
        .copied()
        .enumerate()
        .find(|(_, c)| *c == b':' || *c == b'*')
        .map(|(pos, _)| pos)
}

fn take_segment<'a>(path: &mut &'a [u8]) -> Segment<'a> {
    assert!(!path.is_empty());

    match path[0] {
        b':' => match find_slash(path) {
            Some(pos) => {
                let res = Segment::Param(&path[1..pos]);
                *path = &path[pos..];
                res
            }
            None => {
                let res = Segment::Param(&path[1..]);
                *path = &[];
                res
            }
        },
        b'*' => {
            let res = Segment::CatchAll(&path[1..]);
            *path = &[];
            res
        }
        _ => match find_wildcard(path) {
            Some(pos) => {
                let res = Segment::Static(&path[..pos]);
                *path = &path[pos..];
                res
            }
            None => {
                let res = Segment::Static(&path[..]);
                *path = &[];
                res
            }
        },
    }
}

#[derive(Debug, Eq, PartialEq)]
enum NodeType {
    Root,
    Static,
    Param,
    CatchAll,
}

#[derive(Debug, Eq, PartialEq)]
struct Node<T> {
    node_type: NodeType,
    name: Vec<u8>,
    children: Vec<Node<T>>,
    indices: Vec<u8>,
    param_child: Option<Box<Node<T>>>,
    catch_all_child: Option<Box<Node<T>>>,
    data: Option<T>,
}

impl<T> Node<T> {
    fn new(node_type: NodeType, name: &[u8]) -> Self {
        Self {
            node_type,
            name: name.to_vec(),
            children: vec![],
            indices: vec![],
            param_child: None,
            catch_all_child: None,
            data: None,
        }
    }

    fn find_static_child(&self, prefix: u8) -> Option<usize> {
        for i in 0..self.indices.len() {
            if self.indices[i] == prefix {
                return Some(i);
            }
        }
        None
    }

    fn insert_child(&mut self, mut path: &[u8], data: T) {
        if path.is_empty() {
            self.data = Some(data);
        } else {
            match take_segment(&mut path) {
                Segment::Static(name) => self.insert_static_child(name, data, path),
                Segment::Param(name) => self.insert_param_child(name, data, path),
                Segment::CatchAll(name) => {
                    assert!(path.is_empty());
                    self.insert_catch_all_child(name, data);
                }
            }
        }
    }

    fn insert_static_child(&mut self, name: &[u8], data: T, path: &[u8]) {
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
                        param_child: child.param_child.take(),
                        catch_all_child: child.catch_all_child.take(),
                        data: child.data.take(),
                    };

                    if !name[n..].is_empty() {
                        let b = Node {
                            node_type: NodeType::Static,
                            name: name[n..].to_vec(),
                            children: vec![],
                            indices: vec![],
                            param_child: None,
                            catch_all_child: None,
                            data: None,
                        };

                        child.name = child.name[..n].to_vec();
                        child.indices = vec![a.name[0], b.name[0]];
                        child.children = vec![a, b];

                        let b = child.children.last_mut().unwrap();
                        b.insert_child(path, data);
                    } else {
                        child.name = child.name[..n].to_vec();
                        child.indices = vec![a.name[0]];
                        child.children = vec![a];

                        child.insert_child(path, data);
                    }
                } else if n < name.len() {
                    // add child
                    child.insert_static_child(&name[n..], data, path);
                } else {
                    child.insert_child(path, data);
                }
            }
            None => {
                self.children.push(Node::new(NodeType::Static, name));
                self.indices.push(name[0]);
                self.children.last_mut().unwrap().insert_child(path, data);
            }
        }
    }

    fn insert_param_child(&mut self, name: &[u8], data: T, path: &[u8]) {
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
                    param_child: None,
                    catch_all_child: None,
                    data: None,
                }));
                self.param_child.as_mut().unwrap()
            }
        };
        child.insert_child(path, data);
    }

    fn insert_catch_all_child(&mut self, name: &[u8], data: T) {
        self.catch_all_child = Some(Box::new(Node {
            node_type: NodeType::CatchAll,
            name: name.to_vec(),
            children: vec![],
            indices: vec![],
            param_child: None,
            catch_all_child: None,
            data: Some(data),
        }));
    }

    fn matches<'a: 'b, 'b>(
        &'a self,
        mut path: &'b [u8],
        params: &mut SmallVec<[(&'b [u8], &'b [u8]); 8]>,
    ) -> Option<&'a T> {
        if path.is_empty() {
            return self.data.as_ref();
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

        params.truncate(num_params);

        if let Some(param_child) = &self.param_child {
            let param = match find_slash(path) {
                Some(pos) => {
                    let param = &path[..pos];
                    path = &path[pos..];
                    param
                }
                None => std::mem::take(&mut path),
            };
            params.push((&param_child.name, param));
            if let Some(data) = param_child.matches(path, params) {
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
pub(crate) struct Tree<T> {
    root: Node<T>,
}

impl<T> Default for Tree<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Tree<T> {
    pub(crate) fn new() -> Self {
        Self {
            root: Node::new(NodeType::Root, &[]),
        }
    }

    pub(crate) fn add(&mut self, path: &str, data: T) {
        self.root.insert_child(path.as_bytes(), data);
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
    fn test_take_segment() {
        let mut path: &[u8] = b"/a/:b/c/:ui/ef/ghi/*jkl";
        assert_eq!(take_segment(&mut path), Segment::Static(b"/a/"));
        assert_eq!(take_segment(&mut path), Segment::Param(b"b"));
        assert_eq!(take_segment(&mut path), Segment::Static(b"/c/"));
        assert_eq!(take_segment(&mut path), Segment::Param(b"ui"));
        assert_eq!(take_segment(&mut path), Segment::Static(b"/ef/ghi/"));
        assert_eq!(take_segment(&mut path), Segment::CatchAll(b"jkl"));
    }

    #[test]
    fn test_insert_static_child_1() {
        let mut tree = Tree::new();
        tree.add("/abc", 1);
        tree.add("/abcdef", 2);
        tree.add("/abcdefgh", 3);

        assert_eq!(
            tree,
            Tree {
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
                                param_child: None,
                                catch_all_child: None,
                                data: Some(3),
                            }],
                            indices: vec![b'g'],
                            param_child: None,
                            catch_all_child: None,
                            data: Some(2),
                        }],
                        indices: vec![b'd'],
                        param_child: None,
                        catch_all_child: None,
                        data: Some(1)
                    }],
                    indices: vec![b'/'],
                    param_child: None,
                    catch_all_child: None,
                    data: None,
                }
            }
        );
    }

    #[test]
    fn test_insert_static_child_2() {
        let mut tree = Tree::new();
        tree.add("/abcd", 1);
        tree.add("/ab1234", 2);
        tree.add("/ab1256", 3);
        tree.add("/ab125678", 4);

        assert_eq!(
            tree,
            Tree {
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
                                param_child: None,
                                catch_all_child: None,
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
                                        param_child: None,
                                        catch_all_child: None,
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
                                            param_child: None,
                                            catch_all_child: None,
                                            data: Some(4)
                                        }],
                                        indices: vec![b'7'],
                                        param_child: None,
                                        catch_all_child: None,
                                        data: Some(3)
                                    }
                                ],
                                indices: vec![b'3', b'5'],
                                param_child: None,
                                catch_all_child: None,
                                data: None,
                            }
                        ],
                        indices: vec![b'c', b'1'],
                        param_child: None,
                        catch_all_child: None,
                        data: None
                    }],
                    indices: vec![b'/'],
                    param_child: None,
                    catch_all_child: None,
                    data: None
                }
            }
        );
    }

    #[test]
    fn test_insert_static_child_3() {
        let mut tree = Tree::new();
        tree.add("/abc", 1);
        tree.add("/ab", 2);
        assert_eq!(
            tree,
            Tree {
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
                            param_child: None,
                            catch_all_child: None,
                            data: Some(1)
                        }],
                        indices: vec![b'c'],
                        param_child: None,
                        catch_all_child: None,
                        data: Some(2)
                    }],
                    indices: vec![b'/'],
                    param_child: None,
                    catch_all_child: None,
                    data: None
                }
            }
        )
    }

    #[test]
    fn test_insert_param_child() {
        let mut tree = Tree::new();
        tree.add("/abc/:p1", 1);
        tree.add("/abc/:p1/p2", 2);
        tree.add("/abc/:p1/:p3", 3);
        assert_eq!(
            tree,
            Tree {
                root: Node {
                    node_type: NodeType::Root,
                    name: vec![],
                    children: vec![Node {
                        node_type: NodeType::Static,
                        name: b"/abc/".to_vec(),
                        children: vec![],
                        indices: vec![],
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
                                    param_child: None,
                                    catch_all_child: None,
                                    data: Some(2),
                                }],
                                indices: vec![b'p'],
                                param_child: Some(Box::new(Node {
                                    node_type: NodeType::Param,
                                    name: b"p3".to_vec(),
                                    children: vec![],
                                    indices: vec![],
                                    param_child: None,
                                    catch_all_child: None,
                                    data: Some(3)
                                })),
                                catch_all_child: None,
                                data: None,
                            }],
                            indices: vec![b'/'],
                            param_child: None,
                            catch_all_child: None,
                            data: Some(1)
                        })),
                        catch_all_child: None,
                        data: None
                    }],
                    indices: vec![b'/'],
                    param_child: None,
                    catch_all_child: None,
                    data: None
                }
            }
        )
    }

    #[test]
    fn test_catch_all_child_1() {
        let mut tree = Tree::new();
        tree.add("/abc/*p1", 1);
        tree.add("/ab/de", 2);
        assert_eq!(
            tree,
            Tree {
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
                                param_child: None,
                                catch_all_child: Some(Box::new(Node {
                                    node_type: NodeType::CatchAll,
                                    name: b"p1".to_vec(),
                                    children: vec![],
                                    indices: vec![],
                                    param_child: None,
                                    catch_all_child: None,
                                    data: Some(1)
                                })),
                                data: None
                            },
                            Node {
                                node_type: NodeType::Static,
                                name: b"/de".to_vec(),
                                children: vec![],
                                indices: vec![],
                                param_child: None,
                                catch_all_child: None,
                                data: Some(2)
                            }
                        ],
                        indices: vec![b'c', b'/'],
                        param_child: None,
                        catch_all_child: None,
                        data: None
                    }],
                    indices: vec![b'/'],
                    param_child: None,
                    catch_all_child: None,
                    data: None
                }
            }
        );
    }

    #[test]
    fn test_catch_all_child_2() {
        let mut tree = Tree::new();
        tree.add("*p1", 1);
        assert_eq!(
            tree,
            Tree {
                root: Node {
                    node_type: NodeType::Root,
                    name: vec![],
                    children: vec![],
                    indices: vec![],
                    param_child: None,
                    catch_all_child: Some(Box::new(Node {
                        node_type: NodeType::CatchAll,
                        name: b"p1".to_vec(),
                        children: vec![],
                        indices: vec![],
                        param_child: None,
                        catch_all_child: None,
                        data: Some(1)
                    })),
                    data: None
                }
            }
        );
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
        let mut tree = Tree::new();
        let paths = vec![
            ("/ab/def", 1),
            ("/abc/def", 2),
            ("/abc/:p1", 3),
            ("/abc/:p1/def", 4),
            ("/abc/:p1/:p2", 5),
            ("/abc/def/*p1", 6),
            ("/a/b/c/d", 7),
            ("/a/:p1/:p2/c", 8),
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
        ];

        for (path, res) in matches {
            assert_eq!(tree.matches(path), res);
        }
    }
}
