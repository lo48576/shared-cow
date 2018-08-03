//! `RcCow`.

def_shared_cow! {
    #[doc = "[`Cow`][`std::borrow::Cow`] with variant with shared [`Rc`][`std::rc::Rc`] data."]
    pub def RcCow<B>(std::rc::Rc<B>);
}
impl_cow! { RcCow<B>(std::rc::Rc<B>); <A> }

impl_str_like! { RcCow, std::rc::Rc<str>, str, String }
impl_str_like! { RcCow, std::rc::Rc<std::path::Path>, std::path::Path, std::path::PathBuf }
impl_str_like! { RcCow, std::rc::Rc<std::ffi::OsStr>, std::ffi::OsStr, std::ffi::OsString }
