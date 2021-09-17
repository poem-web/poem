macro_rules! impl_value_type {
    () => {
        type ValueType = Self;
        fn as_value(&self) -> Option<&Self::ValueType> {
            Some(self)
        }
    };
}
