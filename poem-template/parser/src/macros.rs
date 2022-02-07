#[cfg(test)]
macro_rules! check {
    ($parser:expr, $s:expr, $value:expr) => {
        assert_eq!(
            $parser($crate::common::LocatedSpan::new($s)).unwrap().1,
            $value
        );
    };
}

#[cfg(test)]
macro_rules! check_spanned {
    ($parser:expr, $s:expr, $value:expr) => {
        assert_eq!(
            $parser($crate::common::LocatedSpan::new($s))
                .unwrap()
                .1
                .value,
            $value
        );
    };
}
