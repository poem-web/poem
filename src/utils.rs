use std::ops::{Deref, DerefMut};

pub(crate) struct InternalData<T>(pub(crate) T);

impl<T> Deref for InternalData<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for InternalData<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
