//! `ArcCow`.

use std::borrow::{Borrow, Cow};
use std::cmp::Ordering;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::hash;
use std::iter;
use std::ops;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// See <https://github.com/rust-lang/rust/blob/1.27.2/src/liballoc/vec.rs#L2097>.
macro_rules! impl_eq_slice {
    ($lhs:ty, $rhs:ty) => {
        impl_eq_slice! { $lhs, $rhs, Sized }
    };
    ($lhs:ty, $rhs:ty, $bound:ident) => {
        impl<'a, 'b, A, B> PartialEq<$rhs> for $lhs
        where
            A: $bound + PartialEq<B>,
        {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                self[..] == other[..]
            }
        }
    };
}

macro_rules! impl_str_like {
    ($borrowed:ty, $owned:ty) => {
        impl<'a> From<&'a $borrowed> for ArcCow<'a, $borrowed> {
            fn from(s: &'a $borrowed) -> Self {
                ArcCow::Borrowed(s)
            }
        }

        impl<'a> From<$owned> for ArcCow<'a, $borrowed> {
            fn from(s: $owned) -> Self {
                ArcCow::Owned(s)
            }
        }

        impl<'a> From<&'a $owned> for ArcCow<'a, $borrowed> {
            fn from(s: &'a $owned) -> Self {
                ArcCow::Owned(s.clone())
            }
        }

        impl<'a> From<Arc<$borrowed>> for ArcCow<'a, $borrowed> {
            fn from(s: Arc<$borrowed>) -> Self {
                ArcCow::Shared(s)
            }
        }

        impl<'a> Into<$owned> for ArcCow<'a, $borrowed> {
            fn into(self) -> $owned {
                self.into_owned()
            }
        }

        impl_cmp! { $borrowed, ArcCow<'a, $borrowed>, $borrowed }
        impl_cmp! { $borrowed, ArcCow<'a, $borrowed>, &'b $borrowed }
        impl_cmp! { $borrowed, ArcCow<'a, $borrowed>, $owned }
        impl_cmp! { $borrowed, ArcCow<'a, $borrowed>, &'b $owned }
        impl_cmp! { $borrowed, ArcCow<'a, $borrowed>, Cow<'b, $borrowed> }
    };
}

macro_rules! impl_eq {
    ($lhs:ty, $rhs:ty) => {
        impl<'a, 'b> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                self == other
            }
        }

        impl<'a, 'b> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                self == other
            }
        }
    };
}

macro_rules! impl_partial_ord {
    ($base: ty, $lhs:ty, $rhs:ty) => {
        impl<'a, 'b> PartialOrd<$rhs> for $lhs {
            #[inline]
            fn partial_cmp(&self, other: &$rhs) -> Option<Ordering> {
                <$base as PartialOrd>::partial_cmp(self, other)
            }
        }

        impl<'a, 'b> PartialOrd<$lhs> for $rhs {
            #[inline]
            fn partial_cmp(&self, other: &$lhs) -> Option<Ordering> {
                <$base as PartialOrd>::partial_cmp(self, other)
            }
        }
    };
}

macro_rules! impl_cmp {
    ($base: ty, $lhs:ty, $rhs:ty) => {
        impl_eq! { $lhs, $rhs }
        impl_partial_ord! { $base, $lhs, $rhs }
    };
}

/// `Cow` with variant with shared `Arc` data.
pub enum ArcCow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
{
    /// Borrowed data.
    Borrowed(&'a B),
    /// Owned data.
    Owned(<B as ToOwned>::Owned),
    /// Shared data.
    Shared(Arc<B>),
}

impl<'a, B> ArcCow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
{
    /// Creates a new owned value.
    ///
    /// This always clones the value.
    pub fn to_owned(&self) -> <B as ToOwned>::Owned {
        let b: &B = self.borrow();
        b.to_owned()
    }

    /// Creates a new owned value.
    ///
    /// This behaves like [`Cow::into_owned`].
    /// This clones the value if necessary.
    pub fn into_owned(self) -> <B as ToOwned>::Owned {
        match self {
            ArcCow::Borrowed(borrowed) => borrowed.to_owned(),
            ArcCow::Owned(owned) => owned,
            ArcCow::Shared(shared) => (*shared).to_owned(),
        }
    }

    /// Returns mutable reference to the `Owned(_)` value.
    ///
    /// This behaves like [`Cow::to_mut`].
    /// This clones the value if necessary.
    #[allow(unknown_lints, wrong_self_convention)]
    pub fn to_mut(&mut self) -> &mut <B as ToOwned>::Owned {
        *self = ArcCow::Owned(self.to_owned());
        match *self {
            ArcCow::Owned(ref mut owned) => owned,
            _ => unreachable!("Should never happen because `*self` must be `Owned` variant"),
        }
    }
}

impl<'a, B> ArcCow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
    for<'b> Arc<B>: From<&'b B> + From<<B as ToOwned>::Owned>,
{
    /// Creates a new shared value.
    ///
    /// This clones the value if necessary.
    pub fn into_shared(self) -> Arc<B> {
        match self {
            ArcCow::Borrowed(borrowed) => From::from(borrowed),
            ArcCow::Owned(owned) => From::from(owned),
            ArcCow::Shared(shared) => shared,
        }
    }
}

impl<'a, B> ArcCow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
    for<'b> &'b B: Into<Arc<B>>,
{
    /// Creates a new shared value.
    ///
    /// This always clones the value.
    pub fn to_shared(&self) -> Arc<B> {
        match self {
            ArcCow::Borrowed(borrowed) => (*borrowed).into(),
            ArcCow::Owned(owned) => owned.borrow().into(),
            ArcCow::Shared(shared) => Clone::clone(shared),
        }
    }
}

impl<'a, T> From<&'a [T]> for ArcCow<'a, [T]>
where
    T: Clone,
{
    fn from(v: &'a [T]) -> Self {
        ArcCow::Borrowed(v)
    }
}

impl<'a, T> Into<Vec<T>> for ArcCow<'a, [T]>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    fn into(self) -> Vec<T> {
        self.into_owned()
    }
}

impl<'a, T> From<Vec<T>> for ArcCow<'a, [T]>
where
    T: Clone,
{
    fn from(v: Vec<T>) -> Self {
        ArcCow::Owned(v)
    }
}

impl<'a, T> From<&'a Vec<T>> for ArcCow<'a, [T]>
where
    T: Clone,
{
    fn from(v: &'a Vec<T>) -> Self {
        ArcCow::Owned(v.clone())
    }
}

impl<'a, B> AsRef<B> for ArcCow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
{
    fn as_ref(&self) -> &B {
        self
    }
}

impl<'a, B> Borrow<B> for ArcCow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
{
    fn borrow(&self) -> &B {
        &**self
    }
}

impl<'a, B> Clone for ArcCow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
{
    fn clone(&self) -> Self {
        match self {
            ArcCow::Borrowed(b) => ArcCow::Borrowed(b),
            ArcCow::Owned(o) => ArcCow::Owned(o.borrow().to_owned()),
            ArcCow::Shared(s) => ArcCow::Shared(Clone::clone(s)),
        }
    }
}

impl<'a> From<ArcCow<'a, str>> for Box<dyn std::error::Error> {
    fn from(err: ArcCow<'a, str>) -> Self {
        let err: String = err.into();
        From::from(err)
    }
}

impl<'a, 'b> From<ArcCow<'b, str>> for Box<dyn std::error::Error + Send + Sync + 'a> {
    fn from(err: ArcCow<'b, str>) -> Self {
        let err: String = err.into();
        From::from(err)
    }
}

impl<'a, B> hash::Hash for ArcCow<'a, B>
where
    B: ?Sized + hash::Hash + ToOwned,
{
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        hash::Hash::hash(&**self, state)
    }
}

impl<'a> iter::FromIterator<char> for ArcCow<'a, str> {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        ArcCow::Owned(iter::FromIterator::from_iter(iter))
    }
}

impl<'a, 'b> iter::FromIterator<&'b str> for ArcCow<'a, str> {
    fn from_iter<I: IntoIterator<Item = &'b str>>(iter: I) -> Self {
        ArcCow::Owned(iter::FromIterator::from_iter(iter))
    }
}

impl<'a> iter::FromIterator<String> for ArcCow<'a, str> {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        ArcCow::Owned(iter::FromIterator::from_iter(iter))
    }
}

impl<'a> iter::FromIterator<ArcCow<'a, str>> for String {
    fn from_iter<I: IntoIterator<Item = ArcCow<'a, str>>>(iter: I) -> Self {
        let mut buf = String::new();
        buf.extend(iter);
        buf
    }
}

impl<'a, T> iter::FromIterator<T> for ArcCow<'a, [T]>
where
    T: Clone,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        ArcCow::Owned(iter::FromIterator::from_iter(iter))
    }
}

impl<'a> Extend<ArcCow<'a, str>> for String {
    fn extend<I: IntoIterator<Item = ArcCow<'a, str>>>(&mut self, iter: I) {
        for s in iter {
            self.push_str(&s);
        }
    }
}

impl<'a, B> fmt::Debug for ArcCow<'a, B>
where
    B: fmt::Debug + ToOwned + ?Sized,
    <B as ToOwned>::Owned: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArcCow::Borrowed(b) => fmt::Debug::fmt(b, f),
            ArcCow::Owned(o) => fmt::Debug::fmt(o, f),
            ArcCow::Shared(s) => fmt::Debug::fmt(s, f),
        }
    }
}

impl<'a, B> fmt::Display for ArcCow<'a, B>
where
    B: fmt::Display + ToOwned + ?Sized,
    <B as ToOwned>::Owned: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArcCow::Borrowed(b) => fmt::Display::fmt(b, f),
            ArcCow::Owned(o) => fmt::Display::fmt(o, f),
            ArcCow::Shared(s) => fmt::Display::fmt(s, f),
        }
    }
}

impl<'a, B> ops::Deref for ArcCow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
{
    type Target = B;

    fn deref(&self) -> &B {
        match self {
            ArcCow::Borrowed(borrowed) => *borrowed,
            ArcCow::Owned(owned) => owned.borrow(),
            ArcCow::Shared(shared) => (**shared).borrow(),
        }
    }
}

impl<'a, B> Default for ArcCow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
    <B as ToOwned>::Owned: Default,
{
    fn default() -> Self {
        ArcCow::Owned(<B as ToOwned>::Owned::default())
    }
}

impl<'a, 'b, A, B> PartialEq<ArcCow<'b, B>> for ArcCow<'a, A>
where
    A: ?Sized + PartialEq<B> + ToOwned,
    B: ?Sized + ToOwned,
{
    #[inline]
    fn eq(&self, other: &ArcCow<'b, B>) -> bool {
        **self == **other
    }
}

impl_eq_slice! { ArcCow<'a, [A]>, &'b [B], Clone }
impl_eq_slice! { ArcCow<'a, [A]>, &'b mut [B], Clone }
impl_eq_slice! { ArcCow<'a, [A]>, Vec<B>, Clone }
impl_eq_slice! { ArcCow<'a, [A]>, &'b Vec<B>, Clone }

impl<'a, 'b, A, B> PartialEq<Cow<'b, [B]>> for ArcCow<'a, [A]>
where
    A: Clone + ToOwned + PartialEq<B>,
    B: Clone + ToOwned,
{
    #[inline]
    fn eq(&self, other: &Cow<'b, [B]>) -> bool {
        self[..] == other[..]
    }
}

impl<'a, B> Eq for ArcCow<'a, B> where B: ?Sized + Eq + ToOwned {}

impl<'a, B> PartialOrd for ArcCow<'a, B>
where
    B: ?Sized + PartialOrd + ToOwned,
{
    #[inline]
    fn partial_cmp(&self, other: &ArcCow<'a, B>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<'a, B> Ord for ArcCow<'a, B>
where
    B: ?Sized + Ord + ToOwned,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<'a> ops::Add<&'a str> for ArcCow<'a, str> {
    type Output = ArcCow<'a, str>;

    fn add(mut self, rhs: &'a str) -> Self::Output {
        self += rhs;
        self
    }
}

impl<'a> ops::Add<ArcCow<'a, str>> for ArcCow<'a, str> {
    type Output = ArcCow<'a, str>;

    fn add(mut self, rhs: ArcCow<'a, str>) -> Self::Output {
        self += rhs;
        self
    }
}

impl<'a> ops::AddAssign<&'a str> for ArcCow<'a, str> {
    fn add_assign(&mut self, rhs: &'a str) {
        if self.is_empty() {
            *self = ArcCow::Borrowed(rhs)
        } else if rhs.is_empty() {
            return;
        } else {
            match *self {
                ArcCow::Borrowed(lhs) => {
                    let mut s = String::with_capacity(rhs.len() + rhs.len());
                    s.push_str(lhs);
                    *self = ArcCow::Owned(s)
                },
                ArcCow::Shared(ref lhs) => {
                    let mut s = String::with_capacity(rhs.len() + rhs.len());
                    s.push_str(lhs);
                    *self = ArcCow::Owned(s)
                },
                _ => {},
            }
            self.to_mut().push_str(rhs);
        }
    }
}

impl<'a> ops::AddAssign<ArcCow<'a, str>> for ArcCow<'a, str> {
    fn add_assign(&mut self, rhs: ArcCow<'a, str>) {
        if self.is_empty() {
            *self = rhs;
        } else if rhs.is_empty() {
            return;
        } else {
            match *self {
                ArcCow::Borrowed(lhs) => {
                    let mut s = String::with_capacity(rhs.len() + rhs.len());
                    s.push_str(lhs);
                    *self = ArcCow::Owned(s)
                },
                ArcCow::Shared(ref lhs) => {
                    let mut s = String::with_capacity(rhs.len() + rhs.len());
                    s.push_str(lhs);
                    *self = ArcCow::Owned(s)
                },
                _ => {},
            }
            self.to_mut().push_str(&rhs);
        }
    }
}

impl_str_like! { str, String }
impl_str_like! { Path, PathBuf }
impl_str_like! { OsStr, OsString }
