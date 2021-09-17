use std::collections::HashSet;

use self::CharacterClass::{Ascii, InvalidChars, ValidChars};

#[derive(PartialEq, Eq, Clone, Default, Debug)]
pub(crate) struct CharSet {
    low_mask: u64,
    high_mask: u64,
    non_ascii: HashSet<char>,
}

impl CharSet {
    pub(crate) fn new() -> Self {
        Self {
            low_mask: 0,
            high_mask: 0,
            non_ascii: HashSet::new(),
        }
    }

    pub(crate) fn insert(&mut self, char: char) {
        let val = char as u32 - 1;

        if val > 127 {
            self.non_ascii.insert(char);
        } else if val > 63 {
            let bit = 1 << (val - 64);
            self.high_mask |= bit;
        } else {
            let bit = 1 << val;
            self.low_mask |= bit;
        }
    }

    pub(crate) fn contains(&self, char: char) -> bool {
        let val = char as u32 - 1;

        if val > 127 {
            self.non_ascii.contains(&char)
        } else if val > 63 {
            let bit = 1 << (val - 64);
            self.high_mask & bit != 0
        } else {
            let bit = 1 << val;
            self.low_mask & bit != 0
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub(crate) enum CharacterClass {
    Ascii(u64, u64, bool),
    ValidChars(CharSet),
    InvalidChars(CharSet),
}

impl CharacterClass {
    pub(crate) fn any() -> Self {
        Ascii(u64::max_value(), u64::max_value(), true)
    }

    pub(crate) fn valid_char(char: char) -> Self {
        let val = char as u32 - 1;

        if val > 127 {
            ValidChars(Self::char_to_set(char))
        } else if val > 63 {
            Ascii(1 << (val - 64), 0, false)
        } else {
            Ascii(0, 1 << val, false)
        }
    }

    #[cfg(test)]
    pub(crate) fn valid(string: &str) -> Self {
        ValidChars(Self::str_to_set(string))
    }

    #[cfg(test)]
    pub(crate) fn invalid(string: &str) -> Self {
        InvalidChars(Self::str_to_set(string))
    }

    pub(crate) fn invalid_char(char: char) -> Self {
        let val = char as u32 - 1;

        if val > 127 {
            InvalidChars(Self::char_to_set(char))
        } else if val > 63 {
            Ascii(u64::max_value() ^ (1 << (val - 64)), u64::max_value(), true)
        } else {
            Ascii(u64::max_value(), u64::max_value() ^ (1 << val), true)
        }
    }

    pub(crate) fn matches(&self, char: char) -> bool {
        match *self {
            ValidChars(ref valid) => valid.contains(char),
            InvalidChars(ref invalid) => !invalid.contains(char),
            Ascii(high, low, unicode) => {
                let val = char as u32 - 1;
                if val > 127 {
                    unicode
                } else if val > 63 {
                    high & (1 << (val - 64)) != 0
                } else {
                    low & (1 << val) != 0
                }
            }
        }
    }

    fn char_to_set(char: char) -> CharSet {
        let mut set = CharSet::new();
        set.insert(char);
        set
    }

    #[cfg(test)]
    fn str_to_set(string: &str) -> CharSet {
        let mut set = CharSet::new();
        for char in string.chars() {
            set.insert(char);
        }
        set
    }
}

#[derive(Clone)]
struct Thread {
    state: usize,
    captures: Vec<(usize, usize)>,
    capture_begin: Option<usize>,
}

impl Thread {
    pub(crate) fn new() -> Self {
        Self {
            state: 0,
            captures: Vec::new(),
            capture_begin: None,
        }
    }

    #[inline]
    pub(crate) fn start_capture(&mut self, start: usize) {
        self.capture_begin = Some(start);
    }

    #[inline]
    pub(crate) fn end_capture(&mut self, end: usize) {
        self.captures.push((self.capture_begin.unwrap(), end));
        self.capture_begin = None;
    }

    pub(crate) fn extract<'a>(&self, source: &'a str) -> Vec<&'a str> {
        self.captures
            .iter()
            .map(|&(begin, end)| &source[begin..end])
            .collect()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct State<T> {
    pub(crate) index: usize,
    pub(crate) chars: CharacterClass,
    pub(crate) next_states: Vec<usize>,
    pub(crate) acceptance: bool,
    pub(crate) start_capture: bool,
    pub(crate) end_capture: bool,
    pub(crate) metadata: Option<T>,
}

impl<T> PartialEq for State<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> State<T> {
    pub(crate) fn new(index: usize, chars: CharacterClass) -> Self {
        Self {
            index,
            chars,
            next_states: Vec::new(),
            acceptance: false,
            start_capture: false,
            end_capture: false,
            metadata: None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Match<'a> {
    pub(crate) state: usize,
    pub(crate) captures: Vec<&'a str>,
}

impl<'a> Match<'a> {
    pub(crate) fn new(state: usize, captures: Vec<&'_ str>) -> Match<'_> {
        Match { state, captures }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Default, Debug)]
pub(crate) struct NFA<T> {
    states: Vec<State<T>>,
    start_capture: Vec<bool>,
    end_capture: Vec<bool>,
    acceptance: Vec<bool>,
}

impl<T> NFA<T> {
    pub(crate) fn new() -> Self {
        let root = State::new(0, CharacterClass::any());
        Self {
            states: vec![root],
            start_capture: vec![false],
            end_capture: vec![false],
            acceptance: vec![false],
        }
    }

    pub(crate) fn process<'a, I, F>(&self, string: &'a str, mut ord: F) -> Result<Match<'a>, String>
    where
        I: Ord,
        F: FnMut(usize) -> I,
    {
        let mut threads = vec![Thread::new()];

        for (i, char) in string.chars().enumerate() {
            let next_threads = self.process_char(threads, char, i);

            if next_threads.is_empty() {
                return Err(format!("Couldn't process {}", string));
            }

            threads = next_threads;
        }

        let returned = threads
            .into_iter()
            .filter(|thread| self.get(thread.state).acceptance);

        let thread = returned
            .fold(None, |prev, y| {
                let y_v = ord(y.state);
                match prev {
                    None => Some((y_v, y)),
                    Some((x_v, x)) => {
                        if x_v < y_v {
                            Some((y_v, y))
                        } else {
                            Some((x_v, x))
                        }
                    }
                }
            })
            .map(|p| p.1);

        match thread {
            None => Err("The string was exhausted before reaching an \
                         acceptance state"
                .to_string()),
            Some(mut thread) => {
                if thread.capture_begin.is_some() {
                    thread.end_capture(string.len());
                }
                let state = self.get(thread.state);
                Ok(Match::new(state.index, thread.extract(string)))
            }
        }
    }

    #[inline]
    fn process_char(&self, threads: Vec<Thread>, char: char, pos: usize) -> Vec<Thread> {
        let mut returned = Vec::with_capacity(threads.len());

        for mut thread in threads {
            let current_state = self.get(thread.state);

            let mut count = 0;
            let mut found_state = 0;

            for &index in &current_state.next_states {
                let state = &self.states[index];

                if state.chars.matches(char) {
                    count += 1;
                    found_state = index;
                }
            }

            if count == 1 {
                thread.state = found_state;
                capture(self, &mut thread, current_state.index, found_state, pos);
                returned.push(thread);
                continue;
            }

            for &index in &current_state.next_states {
                let state = &self.states[index];
                if state.chars.matches(char) {
                    let mut thread = fork_thread(&thread, state);
                    capture(self, &mut thread, current_state.index, index, pos);
                    returned.push(thread);
                }
            }
        }

        returned
    }

    #[inline]
    pub(crate) fn get(&self, state: usize) -> &State<T> {
        &self.states[state]
    }

    pub(crate) fn get_mut(&mut self, state: usize) -> &mut State<T> {
        &mut self.states[state]
    }

    pub(crate) fn put(&mut self, index: usize, chars: CharacterClass) -> usize {
        {
            let state = self.get(index);

            for &index in &state.next_states {
                let state = self.get(index);
                if state.chars == chars {
                    return index;
                }
            }
        }

        let state = self.new_state(chars);
        self.get_mut(index).next_states.push(state);
        state
    }

    pub(crate) fn put_state(&mut self, index: usize, child: usize) {
        if !self.states[index].next_states.contains(&child) {
            self.get_mut(index).next_states.push(child);
        }
    }

    pub(crate) fn acceptance(&mut self, index: usize) {
        self.get_mut(index).acceptance = true;
        self.acceptance[index] = true;
    }

    pub(crate) fn start_capture(&mut self, index: usize) {
        self.get_mut(index).start_capture = true;
        self.start_capture[index] = true;
    }

    pub(crate) fn end_capture(&mut self, index: usize) {
        self.get_mut(index).end_capture = true;
        self.end_capture[index] = true;
    }

    pub(crate) fn metadata(&mut self, index: usize, metadata: T) {
        self.get_mut(index).metadata = Some(metadata);
    }

    fn new_state(&mut self, chars: CharacterClass) -> usize {
        let index = self.states.len();
        let state = State::new(index, chars);
        self.states.push(state);

        self.acceptance.push(false);
        self.start_capture.push(false);
        self.end_capture.push(false);

        index
    }
}

#[inline]
fn fork_thread<T>(thread: &Thread, state: &State<T>) -> Thread {
    let mut new_trace = thread.clone();
    new_trace.state = state.index;
    new_trace
}

#[inline]
fn capture<T>(
    nfa: &NFA<T>,
    thread: &mut Thread,
    current_state: usize,
    next_state: usize,
    pos: usize,
) {
    if thread.capture_begin == None && nfa.start_capture[next_state] {
        thread.start_capture(pos);
    }

    if thread.capture_begin != None && nfa.end_capture[current_state] && next_state > current_state
    {
        thread.end_capture(pos);
    }
}

#[cfg(test)]
#[allow(clippy::many_single_char_names)]
mod tests {
    use super::{CharSet, CharacterClass, NFA};

    #[test]
    fn basic_test() {
        let mut nfa = NFA::<()>::new();
        let a = nfa.put(0, CharacterClass::valid("h"));
        let b = nfa.put(a, CharacterClass::valid("e"));
        let c = nfa.put(b, CharacterClass::valid("l"));
        let d = nfa.put(c, CharacterClass::valid("l"));
        let e = nfa.put(d, CharacterClass::valid("o"));
        nfa.acceptance(e);

        let m = nfa.process("hello", |a| a);

        assert!(
            m.unwrap().state == e,
            "You didn't get the right final state"
        );
    }

    #[test]
    fn multiple_solutions() {
        let mut nfa = NFA::<()>::new();
        let a1 = nfa.put(0, CharacterClass::valid("n"));
        let b1 = nfa.put(a1, CharacterClass::valid("e"));
        let c1 = nfa.put(b1, CharacterClass::valid("w"));
        nfa.acceptance(c1);

        let a2 = nfa.put(0, CharacterClass::invalid(""));
        let b2 = nfa.put(a2, CharacterClass::invalid(""));
        let c2 = nfa.put(b2, CharacterClass::invalid(""));
        nfa.acceptance(c2);

        let m = nfa.process("new", |a| a);

        assert!(m.unwrap().state == c2, "The two states were not found");
    }

    #[test]
    fn multiple_paths() {
        let mut nfa = NFA::<()>::new();
        let a = nfa.put(0, CharacterClass::valid("t")); // t
        let b1 = nfa.put(a, CharacterClass::valid("h")); // th
        let c1 = nfa.put(b1, CharacterClass::valid("o")); // tho
        let d1 = nfa.put(c1, CharacterClass::valid("m")); // thom
        let e1 = nfa.put(d1, CharacterClass::valid("a")); // thoma
        let f1 = nfa.put(e1, CharacterClass::valid("s")); // thomas

        let b2 = nfa.put(a, CharacterClass::valid("o")); // to
        let c2 = nfa.put(b2, CharacterClass::valid("m")); // tom

        nfa.acceptance(f1);
        nfa.acceptance(c2);

        let thomas = nfa.process("thomas", |a| a);
        let tom = nfa.process("tom", |a| a);
        let thom = nfa.process("thom", |a| a);
        let nope = nfa.process("nope", |a| a);

        assert!(thomas.unwrap().state == f1, "thomas was parsed correctly");
        assert!(tom.unwrap().state == c2, "tom was parsed correctly");
        assert!(thom.is_err(), "thom didn't reach an acceptance state");
        assert!(nope.is_err(), "nope wasn't parsed");
    }

    #[test]
    fn repetitions() {
        let mut nfa = NFA::<()>::new();
        let a = nfa.put(0, CharacterClass::valid("p")); // p
        let b = nfa.put(a, CharacterClass::valid("o")); // po
        let c = nfa.put(b, CharacterClass::valid("s")); // pos
        let d = nfa.put(c, CharacterClass::valid("t")); // post
        let e = nfa.put(d, CharacterClass::valid("s")); // posts
        let f = nfa.put(e, CharacterClass::valid("/")); // posts/
        let g = nfa.put(f, CharacterClass::invalid("/")); // posts/[^/]
        nfa.put_state(g, g);

        nfa.acceptance(g);

        let post = nfa.process("posts/1", |a| a);
        let new_post = nfa.process("posts/new", |a| a);
        let invalid = nfa.process("posts/", |a| a);

        assert!(post.unwrap().state == g, "posts/1 was parsed");
        assert!(new_post.unwrap().state == g, "posts/new was parsed");
        assert!(invalid.is_err(), "posts/ was invalid");
    }

    #[test]
    fn repetitions_with_ambiguous() {
        let mut nfa = NFA::<()>::new();
        let a = nfa.put(0, CharacterClass::valid("p")); // p
        let b = nfa.put(a, CharacterClass::valid("o")); // po
        let c = nfa.put(b, CharacterClass::valid("s")); // pos
        let d = nfa.put(c, CharacterClass::valid("t")); // post
        let e = nfa.put(d, CharacterClass::valid("s")); // posts
        let f = nfa.put(e, CharacterClass::valid("/")); // posts/
        let g1 = nfa.put(f, CharacterClass::invalid("/")); // posts/[^/]
        let g2 = nfa.put(f, CharacterClass::valid("n")); // posts/n
        let h2 = nfa.put(g2, CharacterClass::valid("e")); // posts/ne
        let i2 = nfa.put(h2, CharacterClass::valid("w")); // posts/new

        nfa.put_state(g1, g1);

        nfa.acceptance(g1);
        nfa.acceptance(i2);

        let post = nfa.process("posts/1", |a| a);
        let ambiguous = nfa.process("posts/new", |a| a);
        let invalid = nfa.process("posts/", |a| a);

        assert!(post.unwrap().state == g1, "posts/1 was parsed");
        assert!(ambiguous.unwrap().state == i2, "posts/new was ambiguous");
        assert!(invalid.is_err(), "posts/ was invalid");
    }

    #[test]
    fn captures() {
        let mut nfa = NFA::<()>::new();
        let a = nfa.put(0, CharacterClass::valid("n"));
        let b = nfa.put(a, CharacterClass::valid("e"));
        let c = nfa.put(b, CharacterClass::valid("w"));

        nfa.acceptance(c);
        nfa.start_capture(a);
        nfa.end_capture(c);

        let post = nfa.process("new", |a| a);

        assert_eq!(post.unwrap().captures, vec!["new"]);
    }

    #[test]
    fn capture_mid_match() {
        let mut nfa = NFA::<()>::new();
        let a = nfa.put(0, valid('p'));
        let b = nfa.put(a, valid('/'));
        let c = nfa.put(b, invalid('/'));
        let d = nfa.put(c, valid('/'));
        let e = nfa.put(d, valid('c'));

        nfa.put_state(c, c);
        nfa.acceptance(e);
        nfa.start_capture(c);
        nfa.end_capture(c);

        let post = nfa.process("p/123/c", |a| a);

        assert_eq!(post.unwrap().captures, vec!["123"]);
    }

    #[test]
    fn capture_multiple_captures() {
        let mut nfa = NFA::<()>::new();
        let a = nfa.put(0, valid('p'));
        let b = nfa.put(a, valid('/'));
        let c = nfa.put(b, invalid('/'));
        let d = nfa.put(c, valid('/'));
        let e = nfa.put(d, valid('c'));
        let f = nfa.put(e, valid('/'));
        let g = nfa.put(f, invalid('/'));

        nfa.put_state(c, c);
        nfa.put_state(g, g);
        nfa.acceptance(g);

        nfa.start_capture(c);
        nfa.end_capture(c);

        nfa.start_capture(g);
        nfa.end_capture(g);

        let post = nfa.process("p/123/c/456", |a| a);
        assert_eq!(post.unwrap().captures, vec!["123", "456"]);
    }

    #[test]
    fn test_ascii_set() {
        let mut set = CharSet::new();
        set.insert('?');
        set.insert('a');
        set.insert('é');

        assert!(set.contains('?'), "The set contains char 63");
        assert!(set.contains('a'), "The set contains char 97");
        assert!(set.contains('é'), "The set contains char 233");
        assert!(!set.contains('q'), "The set does not contain q");
        assert!(!set.contains('ü'), "The set does not contain ü");
    }

    fn valid(char: char) -> CharacterClass {
        CharacterClass::valid_char(char)
    }

    fn invalid(char: char) -> CharacterClass {
        CharacterClass::invalid_char(char)
    }
}
