use std::{fmt::Debug, ops::Deref, sync::Arc};

use crate::protocol::{Publish, PublishProperties};

pub trait PublishFilter {
    fn filter(&self, packet: &mut Publish, properties: Option<&mut PublishProperties>) -> bool;
}

#[derive(Clone)]
pub enum PublishFilterRef {
    Owned(Arc<dyn PublishFilter + Send + Sync>),
    Static(&'static (dyn PublishFilter + Send + Sync)),
}

impl Debug for PublishFilterRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owned(arg0) => f.debug_tuple("Owned").finish(),
            Self::Static(arg0) => f.debug_tuple("Static").finish(),
        }
    }
}

impl Deref for PublishFilterRef {
    type Target = dyn PublishFilter;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Static(filter) => *filter,
            Self::Owned(filter) => &**filter,
        }
    }
}

impl<F> PublishFilter for F
where
    F: Fn(&mut Publish, Option<&mut PublishProperties>) -> bool + Send + Sync,
{
    fn filter(&self, packet: &mut Publish, properties: Option<&mut PublishProperties>) -> bool {
        self(packet, properties)
    }
}

impl<F> From<&'static F> for PublishFilterRef
where
    F: Fn(&mut Publish, Option<&mut PublishProperties>) -> bool + Send + Sync,
{
    fn from(value: &'static F) -> Self {
        Self::Static(value)
    }
}

impl<T> From<Box<T>> for PublishFilterRef
where
    T: PublishFilter + 'static + Send + Sync,
{
    fn from(value: Box<T>) -> Self {
        Self::Owned(Arc::<T>::from(value))
    }
}
impl<T> From<Arc<T>> for PublishFilterRef
where
    T: PublishFilter + 'static + Send + Sync,
{
    fn from(value: Arc<T>) -> Self {
        Self::Owned(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter_static(packet: &mut Publish, properties: Option<&mut PublishProperties>) -> bool {
        true
    }
    struct Prejudiced(bool);

    impl PublishFilter for Prejudiced {
        fn filter(&self, packet: &mut Publish, properties: Option<&mut PublishProperties>) -> bool {
            self.0
        }
    }
    #[test]
    fn static_filter() {
        fn is_send<T: Send>(_: &T) {}
        fn takes_static_filter(filter: impl Into<PublishFilterRef>) {
            assert!(matches!(filter.into(), PublishFilterRef::Static(_)));
        }
        fn takes_owned_filter(filter: impl Into<PublishFilterRef>) {
            assert!(matches!(filter.into(), PublishFilterRef::Owned(_)));
        }
        takes_static_filter(&filter_static);
        let boxed: PublishFilterRef = Box::new(Prejudiced(false)).into();
        is_send(&boxed);
        takes_owned_filter(boxed);
    }
}
