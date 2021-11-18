macro_rules! impl_raw_value_type {
    () => {
        type RawValueType = Self;
        fn as_raw_value(&self) -> Option<&Self::RawValueType> {
            Some(self)
        }
    };
}
