use crate::*;

#[derive(Default, Debug)]
pub struct Lazy<T>(OnceLock<T>);

impl<T: Default> Lazy<T> {
    pub const fn new() -> Self {
        Self(OnceLock::new())
    }

    pub fn get(this: &Self) -> Option<&T> {
        this.0.get()
    }

    pub fn force(this: &Self) -> &T {
        this.0.get_or_init(T::default)
    }
}

impl<T: Default> Deref for Lazy<T> {
    type Target = T;

    fn deref(&self) -> &T {
        Self::force(self)
    }
}

impl<T: 'static + Send + Sync> Metric for Lazy<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
