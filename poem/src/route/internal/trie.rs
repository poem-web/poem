use std::{
    collections::HashMap,
    iter::{Peekable, Rev},
    str::Split,
};

use crate::error::RouteError;

#[derive(Debug, Eq, PartialEq)]
struct Node<T> {
    plus_child: Option<Box<Node<T>>>,
    star_child: Option<T>,
    named_children: HashMap<String, Node<T>>,
    data: Option<T>,
}

impl<T> Default for Node<T> {
    fn default() -> Self {
        Self {
            plus_child: None,
            star_child: None,
            named_children: HashMap::new(),
            data: None,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Trie<T> {
    root: Node<T>,
}

impl<T> Default for Trie<T> {
    fn default() -> Self {
        Self {
            root: Node::default(),
        }
    }
}

impl<T> Trie<T> {
    pub(crate) fn add(&mut self, pattern: &str, data: T) -> Result<(), RouteError> {
        let segments = pattern.split('.').rev().peekable();
        if Self::internal_add(segments, &mut self.root, data) {
            Ok(())
        } else {
            Err(RouteError::Duplicate(pattern.to_string()))
        }
    }

    fn internal_add(
        mut segments: Peekable<Rev<Split<char>>>,
        parent_node: &mut Node<T>,
        data: T,
    ) -> bool {
        let segment = segments.next().unwrap();
        let is_last = segments.peek().is_none();

        let node = match segment {
            "+" => parent_node.plus_child.get_or_insert_with(Box::default),
            "*" => return parent_node.star_child.replace(data).is_none(),
            _ => parent_node
                .named_children
                .entry(segment.to_string())
                .or_default(),
        };

        if is_last {
            if node.data.is_some() {
                return false;
            }
            node.data = Some(data);
            true
        } else {
            Self::internal_add(segments, node, data)
        }
    }

    pub(crate) fn matches(&self, domain: &str) -> Option<&T> {
        if domain.is_empty() {
            return self.root.star_child.as_ref();
        }
        let segments = domain.split('.').rev().collect::<Vec<_>>();
        Self::internal_matches(&segments, &self.root)
    }

    fn internal_matches<'a>(segments: &[&str], parent_node: &'a Node<T>) -> Option<&'a T> {
        let (segment, tail) = match segments.split_first() {
            Some((segment, tail)) => (*segment, tail),
            None => return parent_node.data.as_ref(),
        };

        if let Some(node) = parent_node.named_children.get(segment) {
            if let Some(data) = Self::internal_matches(tail, node) {
                return Some(data);
            }
        }

        if let Some(plus_child) = &parent_node.plus_child {
            if let Some(data) = Self::internal_matches(tail, plus_child) {
                return Some(data);
            }
        }

        if let Some(data) = &parent_node.star_child {
            return Some(data);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let mut tree = Trie::default();
        tree.add("www.example.com", 1).unwrap();
        tree.add("example.com", 2).unwrap();
        tree.add("+.example.com", 3).unwrap();
        tree.add("+.a.example.com", 4).unwrap();
        tree.add("b.a.+.com", 5).unwrap();
        tree.add("+.+.com", 6).unwrap();
        tree.add("*.com", 7).unwrap();
        tree.add("*", 8).unwrap();

        assert_eq!(
            tree,
            Trie {
                root: Node {
                    plus_child: None,
                    star_child: Some(8),
                    named_children: vec![(
                        "com".to_string(),
                        Node {
                            plus_child: Some(Box::new(Node {
                                plus_child: Some(Box::new(Node {
                                    plus_child: None,
                                    star_child: None,
                                    named_children: Default::default(),
                                    data: Some(6),
                                })),
                                star_child: None,
                                named_children: vec![(
                                    "a".to_string(),
                                    Node {
                                        plus_child: None,
                                        star_child: None,
                                        named_children: vec![(
                                            "b".to_string(),
                                            Node {
                                                plus_child: None,
                                                star_child: None,
                                                named_children: Default::default(),
                                                data: Some(5)
                                            }
                                        )]
                                        .into_iter()
                                        .collect(),
                                        data: None
                                    }
                                )]
                                .into_iter()
                                .collect(),
                                data: None
                            })),
                            star_child: Some(7),
                            named_children: vec![(
                                "example".to_string(),
                                Node {
                                    plus_child: Some(Box::new(Node {
                                        plus_child: None,
                                        star_child: None,
                                        named_children: Default::default(),
                                        data: Some(3)
                                    })),
                                    star_child: None,
                                    named_children: vec![
                                        (
                                            "www".to_string(),
                                            Node {
                                                plus_child: None,
                                                star_child: None,
                                                named_children: Default::default(),
                                                data: Some(1)
                                            }
                                        ),
                                        (
                                            "a".to_string(),
                                            Node {
                                                plus_child: Some(Box::new(Node {
                                                    plus_child: None,
                                                    star_child: None,
                                                    named_children: Default::default(),
                                                    data: Some(4)
                                                })),
                                                star_child: None,
                                                named_children: Default::default(),
                                                data: None
                                            }
                                        )
                                    ]
                                    .into_iter()
                                    .collect(),
                                    data: Some(2)
                                }
                            )]
                            .into_iter()
                            .collect(),
                            data: None
                        }
                    )]
                    .into_iter()
                    .collect(),
                    data: None
                }
            }
        )
    }

    #[test]
    fn test_matches() {
        let mut tree = Trie::default();

        let domains = vec![
            ("www.example.com", 1),
            ("example.com", 2),
            ("+.example.com", 3),
            ("+.a.example.com", 4),
            ("b.a.+.com", 5),
            ("+.+.com", 6),
            ("*.com", 7),
            ("*", 8),
        ];

        for (domain, id) in domains {
            tree.add(domain, id).unwrap();
        }

        let matches = vec![
            ("www.example.com", Some(&1)),
            ("example.com", Some(&2)),
            ("c.example.com", Some(&3)),
            ("c.a.example.com", Some(&4)),
            ("c.b.example.com", Some(&7)),
            ("b.a.sd.com", Some(&5)),
            ("k.c.com", Some(&6)),
            ("asd.com", Some(&7)),
            ("localhost", Some(&8)),
            ("", Some(&8)),
        ];

        for (domain, id) in matches {
            assert_eq!(tree.matches(domain), id);
        }
    }
}
