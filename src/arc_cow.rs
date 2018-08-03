//! `ArcCow`.

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
    ($cow:ident, $borrowed:ty, $owned:ty) => {
        impl<'a> From<&'a $borrowed> for $cow<'a, $borrowed> {
            fn from(s: &'a $borrowed) -> Self {
                $cow::Borrowed(s)
            }
        }

        impl<'a> From<$owned> for $cow<'a, $borrowed> {
            fn from(s: $owned) -> Self {
                $cow::Owned(s)
            }
        }

        impl<'a> From<&'a $owned> for $cow<'a, $borrowed> {
            fn from(s: &'a $owned) -> Self {
                $cow::Owned(s.clone())
            }
        }

        impl<'a> From<Arc<$borrowed>> for $cow<'a, $borrowed> {
            fn from(s: Arc<$borrowed>) -> Self {
                $cow::Shared(s)
            }
        }

        impl<'a> Into<$owned> for $cow<'a, $borrowed> {
            fn into(self) -> $owned {
                self.into_owned()
            }
        }

        impl_cmp! { $borrowed, $cow<'a, $borrowed>, $borrowed }
        impl_cmp! { $borrowed, $cow<'a, $borrowed>, &'b $borrowed }
        impl_cmp! { $borrowed, $cow<'a, $borrowed>, $owned }
        impl_cmp! { $borrowed, $cow<'a, $borrowed>, &'b $owned }
        impl_cmp! { $borrowed, $cow<'a, $borrowed>, std::borrow::Cow<'b, $borrowed> }
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
            fn partial_cmp(&self, other: &$rhs) -> Option<std::cmp::Ordering> {
                <$base as PartialOrd>::partial_cmp(self, other)
            }
        }

        impl<'a, 'b> PartialOrd<$lhs> for $rhs {
            #[inline]
            fn partial_cmp(&self, other: &$lhs) -> Option<std::cmp::Ordering> {
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

macro_rules! def_shared_cow {
    ($(#[$meta:meta])* $vis:vis def $cow:ident<$typ:ident>($rc:ty);) => {
        $(#[$meta])*
        $vis enum $cow<'a, $typ>
        where
            $typ: 'a + ToOwned + ?Sized,
        {
            /// Borrowed data.
            Borrowed(&'a $typ),
            /// Owned data.
            Owned(<$typ as ToOwned>::Owned),
            /// Shared data.
            Shared($rc),
        }
    };
}

macro_rules! impl_cow_basic {
    ($cow:ident<$typ:ident>($rc:ty)) => {
        impl<'a, $typ> $cow<'a, $typ>
        where
            $typ: 'a + ToOwned + ?Sized,
        {
            /// Creates a new owned value.
            ///
            /// This always clones the value.
            pub fn to_owned(&self) -> <$typ as ToOwned>::Owned {
                use std::borrow::Borrow;
                let b: &$typ = self.borrow();
                b.to_owned()
            }

            /// Creates a new owned value.
            ///
            /// This behaves like [`Cow::into_owned`][`std::borrow::Cow::into_owned`].
            /// This clones the value if necessary.
            pub fn into_owned(self) -> <$typ as ToOwned>::Owned {
                match self {
                    $cow::Borrowed(borrowed) => borrowed.to_owned(),
                    $cow::Owned(owned) => owned,
                    $cow::Shared(shared) => (*shared).to_owned(),
                }
            }

            /// Returns mutable reference to the `Owned(_)` value.
            ///
            /// This behaves like [`Cow::to_mut`][`std::borrow::Cow::to_mut`].
            /// This clones the value if necessary.
            #[allow(unknown_lints, wrong_self_convention)]
            pub fn to_mut(&mut self) -> &mut <$typ as ToOwned>::Owned {
                *self = $cow::Owned(self.to_owned());
                match *self {
                    $cow::Owned(ref mut owned) => owned,
                    _ => {
                        unreachable!("Should never happen because `*self` must be `Owned` variant")
                    },
                }
            }
        }
    };
}

macro_rules! impl_cow_to_shared {
    ($cow:ident<$typ:ident>($rc:ty)) => {
        impl<'a, $typ> $cow<'a, $typ>
        where
            $typ: 'a + ToOwned + ?Sized,
            for<'b> $rc: From<&'b $typ> + From<<$typ as ToOwned>::Owned>,
        {
            /// Creates a new shared value.
            ///
            /// This clones the value if necessary.
            pub fn into_shared(self) -> $rc {
                match self {
                    $cow::Borrowed(borrowed) => From::from(borrowed),
                    $cow::Owned(owned) => From::from(owned),
                    $cow::Shared(shared) => shared,
                }
            }
        }

        impl<'a, $typ> $cow<'a, $typ>
        where
            $typ: 'a + ToOwned + ?Sized,
            for<'b> &'b $typ: Into<$rc>,
        {
            /// Creates a new shared value.
            ///
            /// This always clones the value.
            pub fn to_shared(&self) -> $rc {
                use std::borrow::Borrow;
                match self {
                    $cow::Borrowed(borrowed) => (*borrowed).into(),
                    $cow::Owned(owned) => owned.borrow().into(),
                    $cow::Shared(shared) => Clone::clone(shared),
                }
            }
        }
    };
}

macro_rules! impl_cow_std_traits {
    ($cow:ident<$typ:ident>($rc:ty)) => {
        impl<'a, T> From<&'a [T]> for $cow<'a, [T]>
        where
            T: Clone,
        {
            fn from(v: &'a [T]) -> Self {
                $cow::Borrowed(v)
            }
        }

        impl<'a, T> Into<Vec<T>> for $cow<'a, [T]>
        where
            [T]: ToOwned<Owned = Vec<T>>,
        {
            fn into(self) -> Vec<T> {
                self.into_owned()
            }
        }

        impl<'a, T> From<Vec<T>> for $cow<'a, [T]>
        where
            T: Clone,
        {
            fn from(v: Vec<T>) -> Self {
                $cow::Owned(v)
            }
        }

        impl<'a, T> From<&'a Vec<T>> for $cow<'a, [T]>
        where
            T: Clone,
        {
            fn from(v: &'a Vec<T>) -> Self {
                $cow::Owned(v.clone())
            }
        }

        impl<'a, B> AsRef<B> for $cow<'a, B>
        where
            B: 'a + ToOwned + ?Sized,
        {
            fn as_ref(&self) -> &B {
                self
            }
        }

        impl<'a, B> std::borrow::Borrow<B> for $cow<'a, B>
        where
            B: 'a + ToOwned + ?Sized,
        {
            fn borrow(&self) -> &B {
                &**self
            }
        }

        impl<'a, B> Clone for $cow<'a, B>
        where
            B: 'a + ToOwned + ?Sized,
        {
            fn clone(&self) -> Self {
                use std::borrow::Borrow;
                match self {
                    $cow::Borrowed(b) => $cow::Borrowed(b),
                    $cow::Owned(o) => $cow::Owned(o.borrow().to_owned()),
                    $cow::Shared(s) => $cow::Shared(Clone::clone(s)),
                }
            }
        }

        impl<'a> From<$cow<'a, str>> for Box<dyn std::error::Error> {
            fn from(err: $cow<'a, str>) -> Self {
                let err: String = err.into();
                From::from(err)
            }
        }

        impl<'a, 'b> From<$cow<'b, str>> for Box<dyn std::error::Error + Send + Sync + 'a> {
            fn from(err: $cow<'b, str>) -> Self {
                let err: String = err.into();
                From::from(err)
            }
        }

        impl<'a, B> std::hash::Hash for $cow<'a, B>
        where
            B: ?Sized + std::hash::Hash + ToOwned,
        {
            #[inline]
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                std::hash::Hash::hash(&**self, state)
            }
        }

        impl<'a> std::iter::FromIterator<char> for $cow<'a, str> {
            fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
                $cow::Owned(std::iter::FromIterator::from_iter(iter))
            }
        }

        impl<'a, 'b> std::iter::FromIterator<&'b str> for $cow<'a, str> {
            fn from_iter<I: IntoIterator<Item = &'b str>>(iter: I) -> Self {
                $cow::Owned(std::iter::FromIterator::from_iter(iter))
            }
        }

        impl<'a> std::iter::FromIterator<String> for $cow<'a, str> {
            fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
                $cow::Owned(std::iter::FromIterator::from_iter(iter))
            }
        }

        impl<'a> std::iter::FromIterator<$cow<'a, str>> for String {
            fn from_iter<I: IntoIterator<Item = $cow<'a, str>>>(iter: I) -> Self {
                let mut buf = String::new();
                buf.extend(iter);
                buf
            }
        }

        impl<'a, T> std::iter::FromIterator<T> for $cow<'a, [T]>
        where
            T: Clone,
        {
            fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
                $cow::Owned(std::iter::FromIterator::from_iter(iter))
            }
        }

        impl<'a> Extend<$cow<'a, str>> for String {
            fn extend<I: IntoIterator<Item = $cow<'a, str>>>(&mut self, iter: I) {
                for s in iter {
                    self.push_str(&s);
                }
            }
        }

        impl<'a, B> std::fmt::Debug for $cow<'a, B>
        where
            B: std::fmt::Debug + ToOwned + ?Sized,
            <B as ToOwned>::Owned: std::fmt::Debug,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $cow::Borrowed(b) => std::fmt::Debug::fmt(b, f),
                    $cow::Owned(o) => std::fmt::Debug::fmt(o, f),
                    $cow::Shared(s) => std::fmt::Debug::fmt(s, f),
                }
            }
        }

        impl<'a, B> std::fmt::Display for $cow<'a, B>
        where
            B: std::fmt::Display + ToOwned + ?Sized,
            <B as ToOwned>::Owned: std::fmt::Display,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $cow::Borrowed(b) => std::fmt::Display::fmt(b, f),
                    $cow::Owned(o) => std::fmt::Display::fmt(o, f),
                    $cow::Shared(s) => std::fmt::Display::fmt(s, f),
                }
            }
        }

        impl<'a, B> std::ops::Deref for $cow<'a, B>
        where
            B: 'a + ToOwned + ?Sized,
        {
            type Target = B;

            fn deref(&self) -> &B {
                use std::borrow::Borrow;
                match self {
                    $cow::Borrowed(borrowed) => *borrowed,
                    $cow::Owned(owned) => owned.borrow(),
                    $cow::Shared(shared) => (**shared).borrow(),
                }
            }
        }

        impl<'a, B> Default for $cow<'a, B>
        where
            B: 'a + ToOwned + ?Sized,
            <B as ToOwned>::Owned: Default,
        {
            fn default() -> Self {
                $cow::Owned(<B as ToOwned>::Owned::default())
            }
        }

        impl<'a> std::ops::Add<&'a str> for $cow<'a, str> {
            type Output = $cow<'a, str>;

            fn add(mut self, rhs: &'a str) -> Self::Output {
                self += rhs;
                self
            }
        }

        impl<'a> std::ops::Add<$cow<'a, str>> for $cow<'a, str> {
            type Output = $cow<'a, str>;

            fn add(mut self, rhs: $cow<'a, str>) -> Self::Output {
                self += rhs;
                self
            }
        }

        impl<'a> std::ops::AddAssign<&'a str> for $cow<'a, str> {
            fn add_assign(&mut self, rhs: &'a str) {
                if self.is_empty() {
                    *self = $cow::Borrowed(rhs)
                } else if rhs.is_empty() {
                    return;
                } else {
                    match *self {
                        $cow::Borrowed(lhs) => {
                            let mut s = String::with_capacity(rhs.len() + rhs.len());
                            s.push_str(lhs);
                            *self = $cow::Owned(s)
                        },
                        $cow::Shared(ref lhs) => {
                            let mut s = String::with_capacity(rhs.len() + rhs.len());
                            s.push_str(lhs);
                            *self = $cow::Owned(s)
                        },
                        _ => {},
                    }
                    self.to_mut().push_str(rhs);
                }
            }
        }

        impl<'a> std::ops::AddAssign<$cow<'a, str>> for $cow<'a, str> {
            fn add_assign(&mut self, rhs: $cow<'a, str>) {
                if self.is_empty() {
                    *self = rhs;
                } else if rhs.is_empty() {
                    return;
                } else {
                    match *self {
                        $cow::Borrowed(lhs) => {
                            let mut s = String::with_capacity(rhs.len() + rhs.len());
                            s.push_str(lhs);
                            *self = $cow::Owned(s)
                        },
                        $cow::Shared(ref lhs) => {
                            let mut s = String::with_capacity(rhs.len() + rhs.len());
                            s.push_str(lhs);
                            *self = $cow::Owned(s)
                        },
                        _ => {},
                    }
                    self.to_mut().push_str(&rhs);
                }
            }
        }
    };
}

macro_rules! impl_cow_cmp_traits {
    ($cow:ident<$typ:ident>($rc:ty); <$other_typ:ident>) => {
        impl<'a, 'b, $other_typ, $typ> PartialEq<$cow<'b, $typ>> for $cow<'a, $other_typ>
        where
            $other_typ: ?Sized + PartialEq<$typ> + ToOwned,
            $typ: ?Sized + ToOwned,
        {
            #[inline]
            fn eq(&self, other: &$cow<'b, $typ>) -> bool {
                **self == **other
            }
        }

        impl_eq_slice! { $cow<'a, [$other_typ]>, &'b [$typ], Clone }
        impl_eq_slice! { $cow<'a, [$other_typ]>, &'b mut [$typ], Clone }
        impl_eq_slice! { $cow<'a, [$other_typ]>, Vec<$typ>, Clone }
        impl_eq_slice! { $cow<'a, [$other_typ]>, &'b Vec<$typ>, Clone }

        impl<'a, 'b, $other_typ, $typ> PartialEq<std::borrow::Cow<'b, [$typ]>> for $cow<'a, [$other_typ]>
        where
            $other_typ: Clone + ToOwned + PartialEq<$typ>,
            $typ: Clone + ToOwned,
        {
            #[inline]
            fn eq(&self, other: &std::borrow::Cow<'b, [$typ]>) -> bool {
                self[..] == other[..]
            }
        }

        impl<'a, $typ> Eq for $cow<'a, $typ> where $typ: ?Sized + Eq + ToOwned {}

        impl<'a, $typ> PartialOrd for $cow<'a, $typ>
        where
            $typ: ?Sized + PartialOrd + ToOwned,
        {
            #[inline]
            fn partial_cmp(&self, other: &$cow<'a, $typ>) -> Option<std::cmp::Ordering> {
                PartialOrd::partial_cmp(&**self, &**other)
            }
        }

        impl<'a, $typ> Ord for $cow<'a, $typ>
        where
            $typ: ?Sized + Ord + ToOwned,
        {
            #[inline]
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                Ord::cmp(&**self, &**other)
            }
        }
    };
}

macro_rules! impl_cow {
    ($cow:ident<$typ:ident>($rc:ty); <$other_typ:ident>) => {
        impl_cow_basic! { $cow<$typ>($rc) }
        impl_cow_to_shared! { $cow<$typ>($rc) }
        impl_cow_std_traits! { $cow<$typ>($rc) }
        impl_cow_cmp_traits! { $cow<$typ>($rc); <$other_typ> }
    };
}

def_shared_cow! {
    #[doc = "[`Cow`][`std::borrow::Cow`] with variant with shared [`Arc`][`std::sync::Arc`] data."]
    pub def ArcCow<B>(Arc<B>);
}
impl_cow! { ArcCow<B>(Arc<B>); <A> }

impl_str_like! { ArcCow, str, String }
impl_str_like! { ArcCow, std::path::Path, std::path::PathBuf }
impl_str_like! { ArcCow, std::ffi::OsStr, std::ffi::OsString }
