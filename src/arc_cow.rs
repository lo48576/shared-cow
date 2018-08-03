//! `ArcCow`.

def_shared_cow! {
    #[doc = "[`Cow`][`std::borrow::Cow`] with variant with shared [`Arc`][`std::sync::Arc`] data."]
    pub def ArcCow<B>(std::sync::Arc<B>);
}
impl_cow! { ArcCow<B>(std::sync::Arc<B>); <A> }

impl_str_like! { ArcCow, std::sync::Arc<str>, str, String }
impl_str_like! { ArcCow, std::sync::Arc<std::path::Path>, std::path::Path, std::path::PathBuf }
impl_str_like! { ArcCow, std::sync::Arc<std::ffi::OsStr>, std::ffi::OsStr, std::ffi::OsString }
