//! `ArcCow`.

use std::rc::Rc;

use crate::RcCow;

def_shared_cow! {
    #[doc = "[`Cow`][`std::borrow::Cow`] with variant with shared [`Arc`][`std::sync::Arc`] data."]
    pub def ArcCow<B>(std::sync::Arc<B>);
}
impl_cow! { ArcCow<B>(std::sync::Arc<B>); <A> }

impl_str_like! { ArcCow, std::sync::Arc<str>, str, String }
impl_str_like! { ArcCow, std::sync::Arc<std::path::Path>, std::path::Path, std::path::PathBuf }
impl_str_like! { ArcCow, std::sync::Arc<std::ffi::OsStr>, std::ffi::OsStr, std::ffi::OsString }

impl<'a, B> ArcCow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
    for<'b> &'b B: Into<Rc<B>>,
{
    /// Creates an [`RcCow`] value.
    #[must_use]
    pub fn to_rccow(&self) -> RcCow<'a, B> {
        use std::borrow::Borrow;
        match *self {
            ArcCow::Borrowed(b) => RcCow::Borrowed(b),
            ArcCow::Owned(ref o) => RcCow::Owned(o.borrow().to_owned()),
            ArcCow::Shared(ref s) => {
                let b: &B = s.borrow();
                RcCow::Shared(b.into())
            },
        }
    }
}
