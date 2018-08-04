//! `RcCow`.

use std::sync::Arc;

use crate::ArcCow;

def_shared_cow! {
    #[doc = "[`Cow`][`std::borrow::Cow`] with variant with shared [`Rc`][`std::rc::Rc`] data."]
    pub def RcCow<B>(std::rc::Rc<B>);
}
impl_cow! { RcCow<B>(std::rc::Rc<B>); <A> }

impl_str_like! { RcCow, std::rc::Rc<str>, str, String }
impl_str_like! { RcCow, std::rc::Rc<std::path::Path>, std::path::Path, std::path::PathBuf }
impl_str_like! { RcCow, std::rc::Rc<std::ffi::OsStr>, std::ffi::OsStr, std::ffi::OsString }

impl<'a, B> RcCow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
    for<'b> &'b B: Into<Arc<B>>,
{
    /// Creates an [`ArcCow`] value.
    pub fn to_arccow(&self) -> ArcCow<'a, B> {
        use std::borrow::Borrow;
        match *self {
            RcCow::Borrowed(b) => ArcCow::Borrowed(b),
            RcCow::Owned(ref o) => ArcCow::Owned(o.borrow().to_owned()),
            RcCow::Shared(ref s) => {
                let b: &B = s.borrow();
                ArcCow::Shared(b.into())
            },
        }
    }
}
