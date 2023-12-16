//! An archived version of `Option`.

use core::{
    cmp, hash,
    iter::DoubleEndedIterator,
    mem,
    ops::{Deref, DerefMut},
    pin::Pin,
};

/// An archived [`Option`].
///
/// It functions identically to [`Option`] but has a different internal
/// representation to allow for archiving.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "validation", derive(bytecheck::CheckBytes))]
#[repr(u8)]
pub enum ArchivedOption<T> {
    /// No value
    None,
    /// Some value `T`
    Some(T),
}

impl<T> ArchivedOption<T> {
    /// Returns `true` if the option is a `None` value.
    #[inline]
    pub fn is_none(&self) -> bool {
        match self {
            ArchivedOption::None => true,
            ArchivedOption::Some(_) => false,
        }
    }

    /// Returns `true` if the option is a `Some` value.
    #[inline]
    pub fn is_some(&self) -> bool {
        match self {
            ArchivedOption::None => false,
            ArchivedOption::Some(_) => true,
        }
    }

    /// Converts to an `Option<&T>`.
    #[inline]
    pub const fn as_ref(&self) -> Option<&T> {
        match self {
            ArchivedOption::None => None,
            ArchivedOption::Some(value) => Some(value),
        }
    }

    /// Converts to an `Option<&mut T>`.
    #[inline]
    pub fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            ArchivedOption::None => None,
            ArchivedOption::Some(value) => Some(value),
        }
    }

    /// Converts from `Pin<&ArchivedOption<T>>` to `Option<Pin<&T>>`.
    #[inline]
    pub fn as_pin_ref(self: Pin<&Self>) -> Option<Pin<&T>> {
        unsafe { Pin::get_ref(self).as_ref().map(|x| Pin::new_unchecked(x)) }
    }

    /// Converts from `Pin<&mut ArchivedOption<T>>` to `Option<Pin<&mut T>>`.
    #[inline]
    pub fn as_pin_mut(self: Pin<&mut Self>) -> Option<Pin<&mut T>> {
        unsafe {
            Pin::get_unchecked_mut(self)
                .as_mut()
                .map(|x| Pin::new_unchecked(x))
        }
    }

    /// Returns an iterator over the possibly contained value.
    #[inline]
    pub const fn iter(&self) -> Iter<'_, T> {
        Iter {
            inner: self.as_ref(),
        }
    }

    /// Returns a mutable iterator over the possibly contained value.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            inner: self.as_mut(),
        }
    }

    /// Inserts `v` into the option if it is `None`, then returns a mutable
    /// reference to the contained value.
    #[inline]
    pub fn get_or_insert(&mut self, v: T) -> &mut T {
        self.get_or_insert_with(move || v)
    }

    /// Inserts a value computed from `f` into the option if it is `None`, then
    /// returns a mutable reference to the contained value.
    #[inline]
    pub fn get_or_insert_with<F: FnOnce() -> T>(&mut self, f: F) -> &mut T {
        if let ArchivedOption::Some(ref mut value) = self {
            value
        } else {
            *self = ArchivedOption::Some(f());
            self.as_mut().unwrap()
        }
    }
}

impl<T: Deref> ArchivedOption<T> {
    /// Converts from `&ArchivedOption<T>` to `Option<&T::Target>`.
    ///
    /// Leaves the original `ArchivedOption` in-place, creating a new one with a reference to the
    /// original one, additionally coercing the contents via `Deref`.
    #[inline]
    pub fn as_deref(&self) -> Option<&<T as Deref>::Target> {
        self.as_ref().map(|x| x.deref())
    }
}

impl<T: DerefMut> ArchivedOption<T> {
    /// Converts from `&mut ArchivedOption<T>` to `Option<&mut T::Target>`.
    ///
    /// Leaves the original `ArchivedOption` in-place, creating a new `Option` with a mutable
    /// reference to the inner type's `Deref::Target` type.
    #[inline]
    pub fn as_deref_mut(&mut self) -> Option<&mut <T as Deref>::Target> {
        self.as_mut().map(|x| x.deref_mut())
    }
}

impl<T: Eq> Eq for ArchivedOption<T> {}

impl<T: hash::Hash> hash::Hash for ArchivedOption<T> {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<T: Ord> Ord for ArchivedOption<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ref().cmp(&other.as_ref())
    }
}

impl<T: PartialEq> PartialEq for ArchivedOption<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(&other.as_ref())
    }
}

impl<T: PartialOrd> PartialOrd for ArchivedOption<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_ref().partial_cmp(&other.as_ref())
    }
}

impl<T, U: PartialOrd<T>> PartialOrd<Option<T>> for ArchivedOption<U> {
    #[inline]
    fn partial_cmp(&self, other: &Option<T>) -> Option<cmp::Ordering> {
        match (self, other) {
            (ArchivedOption::None, None) => Some(cmp::Ordering::Equal),
            (ArchivedOption::None, Some(_)) => Some(cmp::Ordering::Less),
            (ArchivedOption::Some(_), None) => Some(cmp::Ordering::Greater),
            (ArchivedOption::Some(self_value), Some(other_value)) => self_value.partial_cmp(other_value),
        }
    }
}

impl<T: PartialOrd<U>, U> PartialOrd<ArchivedOption<T>> for Option<U> {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedOption<T>) -> Option<cmp::Ordering> {
        match (self, other) {
            (None, ArchivedOption::None) => Some(cmp::Ordering::Equal),
            (None, ArchivedOption::Some(_)) => Some(cmp::Ordering::Less),
            (Some(_), ArchivedOption::None) => Some(cmp::Ordering::Greater),
            (Some(self_value), ArchivedOption::Some(other_value)) => other_value.partial_cmp(self_value).map(|ord| ord.reverse()),
        }
    }
}

impl<T, U: PartialEq<T>> PartialEq<Option<T>> for ArchivedOption<U> {
    #[inline]
    fn eq(&self, other: &Option<T>) -> bool {
        if let ArchivedOption::Some(self_value) = self {
            if let Some(other_value) = other {
                self_value.eq(other_value)
            } else {
                false
            }
        } else {
            other.is_none()
        }
    }
}

impl<T: PartialEq<U>, U> PartialEq<ArchivedOption<T>> for Option<U> {
    #[inline]
    fn eq(&self, other: &ArchivedOption<T>) -> bool {
        other.eq(self)
    }
}

impl<T> From<T> for ArchivedOption<T> {
    /// Moves `val` into a new [`Some`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use rkyv::option::ArchivedOption;
    /// let o: ArchivedOption<u8> = ArchivedOption::from(67);
    ///
    /// assert_eq!(Some(67), o);
    /// ```
    fn from(val: T) -> ArchivedOption<T> {
        ArchivedOption::Some(val)
    }
}

/// An iterator over a reference to the `Some` variant of an `ArchivedOption`.
///
/// This iterator yields one value if the `ArchivedOption` is a `Some`, otherwise none.
///
/// This `struct` is created by the [`ArchivedOption::iter`] function.
pub struct Iter<'a, T> {
    pub(crate) inner: Option<&'a T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let mut result = None;
        mem::swap(&mut self.inner, &mut result);
        result
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

/// An iterator over a mutable reference to the `Some` variant of an `ArchivedOption`.
///
/// This iterator yields one value if the `ArchivedOption` is a `Some`, otherwise none.
///
/// This `struct` is created by the [`ArchivedOption::iter_mut`] function.
pub struct IterMut<'a, T> {
    pub(crate) inner: Option<&'a mut T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        let mut result = None;
        mem::swap(&mut self.inner, &mut result);
        result
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

#[cfg(test)]
mod tests {
    use super::ArchivedOption;
    use core::cmp::Ordering;

    #[test]
    fn partial_ord_option() {
        let a: ArchivedOption<u8> = ArchivedOption::Some(42);
        let b = Some(42);
        assert_eq!(Some(Ordering::Equal), a.partial_cmp(&b));
        assert_eq!(Some(Ordering::Equal), b.partial_cmp(&a));

        let a: ArchivedOption<u8> = ArchivedOption::Some(1);
        let b = Some(2);
        assert_eq!(Some(Ordering::Less), a.partial_cmp(&b));
        assert_eq!(Some(Ordering::Greater), b.partial_cmp(&a));

        let a: ArchivedOption<u8> = ArchivedOption::Some(2);
        let b = Some(1);
        assert_eq!(Some(Ordering::Greater), a.partial_cmp(&b));
        assert_eq!(Some(Ordering::Less), b.partial_cmp(&a));
    }
}
