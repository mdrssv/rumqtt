use std::{fmt::Debug, ops::Deref, sync::Arc};

use super::Connection;

use crate::protocol::{Publish, PublishProperties};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PublishFilterContext<'a> {
    pub username: Option<&'a String>,
    pub tennant_id: Option<&'a String>,
}

impl<'a> From<&'a Connection> for PublishFilterContext<'a> {
    fn from(value: &'a Connection) -> Self {
        Self {
            username: value.username.as_ref(),
            tennant_id: value.tenant_id.as_ref(),
        }
    }
}

/// Filter for [`Publish`] packets
pub trait PublishFilter {
    /// Determines whether an [`Publish`] packet should be processed
    /// Arguments:
    /// * `connection`: connection which delivered the `packet`, might contain a username
    /// * `packet`: to be published, may be modified if necessary
    /// * `properties`: received along with the packet, may be `None` for older MQTT versions
    /// Returns: [`bool`] indicating if the packet should be processed
    fn filter(
        &self,
        context: &PublishFilterContext,
        packet: &mut Publish,
        properties: Option<&mut PublishProperties>,
    ) -> bool;
}

/// Container for either an owned [`PublishFilter`] or an `'static` reference
#[derive(Clone)]
pub enum PublishFilterRef {
    Owned(Arc<dyn PublishFilter + Send + Sync>),
    Static(&'static (dyn PublishFilter + Send + Sync)),
}

impl Debug for PublishFilterRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owned(_arg0) => f.debug_tuple("Owned").finish(),
            Self::Static(_arg0) => f.debug_tuple("Static").finish(),
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

/// Implements [`PublishFilter`] for any ordinary function
impl<F> PublishFilter for F
where
    F: Fn(&PublishFilterContext, &mut Publish, Option<&mut PublishProperties>) -> bool
        + Send
        + Sync,
{
    fn filter(
        &self,
        context: &PublishFilterContext<'_>,
        packet: &mut Publish,
        properties: Option<&mut PublishProperties>,
    ) -> bool {
        self(context, packet, properties)
    }
}

/// Implements the conversion
/// ```rust
/// # use rumqttd::{protocol::{Publish, PublishProperties}, PublishFilterContext, PublishFilterRef};
/// fn filter_static(context: &PublishFilterContext, packet: &mut Publish, properties: Option<&mut PublishProperties>) -> bool {
///     todo!()
/// }
///
/// let filter = PublishFilterRef::from(&filter_static);
/// # assert!(matches!(filter, PublishFilterRef::Static(_)));
/// ```
impl<F> From<&'static F> for PublishFilterRef
where
    F: Fn(&PublishFilterContext, &mut Publish, Option<&mut PublishProperties>) -> bool
        + Send
        + Sync,
{
    fn from(value: &'static F) -> Self {
        Self::Static(value)
    }
}

/// Implements the conversion
/// ```rust
/// # use std::boxed::Box;
/// # use rumqttd::{protocol::{Publish, PublishProperties}, PublishFilter, PublishFilterContext, PublishFilterRef};
/// #[derive(Clone)]
/// struct MyFilter {}
///
/// impl PublishFilter for MyFilter {
///     fn filter(&self,context: &PublishFilterContext<'_>, packet: &mut Publish, properties: Option<&mut PublishProperties>) -> bool {
///         todo!()
///     }
/// }
/// let boxed: Box<MyFilter> = Box::new(MyFilter {});
///
/// let filter = PublishFilterRef::from(boxed);
/// # assert!(matches!(filter, PublishFilterRef::Owned(_)));
/// ```
impl<T> From<Arc<T>> for PublishFilterRef
where
    T: PublishFilter + 'static + Send + Sync,
{
    fn from(value: Arc<T>) -> Self {
        Self::Owned(value)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn filter_static(
        _context: &PublishFilterContext<'_>,
        _packet: &mut Publish,
        _properties: Option<&mut PublishProperties>,
    ) -> bool {
        true
    }
    struct Prejudiced(bool);

    impl PublishFilter for Prejudiced {
        fn filter(
            &self,
            _context: &PublishFilterContext<'_>,
            _packet: &mut Publish,
            _propertiess: Option<&mut PublishProperties>,
        ) -> bool {
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
