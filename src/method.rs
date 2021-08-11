#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Method {
    Options = 0,
    Get = 1,
    Post = 2,
    Put = 3,
    Delete = 4,
    Head = 5,
    Trace = 6,
    Connect = 7,
    Patch = 8,
}

pub(crate) const COUNT_METHODS: usize = 9;
