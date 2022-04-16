/*!
A low level builder type for creating fixed size arrays. See [`ArrayBuilder`]
for details.
*/

use core::fmt::{self, Debug, Formatter};

use arrayvec::ArrayVec;

/**
Error type returned from [`ArrayBuilder::try_push`], indicating that the
builder was already full. Includes the value that couldn't be pushed to the
array.
*/
#[derive(Debug, Clone, Copy)]
pub struct Overflow<T>(pub T);

/**
Result type returned from [`ArrayBuilder::push`], indicating whether the
array is full after the push. `ArrayBuilder::push` panics on overflow, so this
only indicates if there is room to push additional elements.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushResult {
    NotFull,
    Full,
}

/**
Low-level builder type for `[T; N]` arrays. Uses a
[`push`][ArrayBuilder::push] + [`finish`][ArrayBuilder::finish] interface to
build an array 1 element at a time.

The interface provided by this type is fairly low level; most of its methods
are fallible in some way (returning a [`Result`] or panicking on errors).
Consider instead the misuse-resistant
[`move_builder::ArrayBuilder`][crate::move_builder::ArrayBuilder], which uses
ownership semantics to provide only infallible operations, or the
[`build!`][crate::build] macro at the top level of the crate.
*/
#[derive(Clone)]
pub struct ArrayBuilder<T, const N: usize> {
    vec: ArrayVec<T, N>,
}

impl<T, const N: usize> ArrayBuilder<T, N> {
    /**
    Create a new, empty `ArrayBuilder`.
    */
    pub const fn new() -> Self {
        Self {
            vec: ArrayVec::new_const(),
        }
    }

    /**
    Returns true if every element in the array is initialized. If the
    builder is full, the next call to `finish` will return the built array.
    */
    #[inline]
    pub fn is_full(&self) -> bool {
        self.vec.is_full()
    }

    /**
    Returns true if no elements in the array are initialized.
    */
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    /**
    Returns the number of initialized elements in the array.
    */
    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /**
    Get a PushResult after a push. Indicates if the array is full or not.
    */
    fn push_result(&self) -> PushResult {
        match self.len() >= N {
            true => PushResult::Full,
            false => PushResult::NotFull,
        }
    }

    /// Add an initialized element to the array, without performing a bounds
    /// check.
    ///
    /// # Safety
    ///
    /// This must only be called when the builder is not full.
    #[inline]
    pub unsafe fn push_unchecked(&mut self, value: T) -> PushResult {
        debug_assert!(self.vec.len() < N);

        // Safety: the caller has ensured that the array isn't full yet.
        self.vec.push_unchecked(value);
        self.push_result()
    }

    /**
    Try to add an initialized element to the array. Returns an error if the
    array is already full, or a [`PushResult`] indicating if the array is now full
    and can be retrieved via [`finish`][Self::finish].
    */
    #[inline]
    pub fn try_push(&mut self, value: T) -> Result<PushResult, Overflow<T>> {
        self.vec
            .try_push(value)
            .map(|()| self.push_result())
            .map_err(|err| Overflow(err.element()))
    }

    /**
    Add an initialized element to the array. Returns a [`PushResult`]
    indicating if the array is now full and can be retrieved via
    [`finish`][Self::finish].

    # Panics

    Panics if the array is already full.
    */
    #[inline]
    pub fn push(&mut self, value: T) -> PushResult {
        match self.try_push(value) {
            Ok(result) => result,
            Err(..) => panic!("ArrayBuilder::push overflow"),
        }
    }

    /// Return the fully initialized array without checking that it's fully
    /// initialized.
    ///
    /// # Safety
    ///
    /// This must only be called when the builder is full.
    #[inline]
    pub unsafe fn finish_unchecked(self) -> [T; N] {
        debug_assert!(self.vec.len() == N);
        self.vec.into_inner_unchecked()
    }

    /**
    Try to return the fully initialized array. Returns the builder if the
    array isn't fully initialized yet.
    */
    #[inline]
    pub fn try_finish(self) -> Result<[T; N], Self> {
        self.vec.into_inner().map_err(|vec| Self { vec })
    }

    /**
    Return the fully initialized array.

    # Panics

    Panics if the array isn't fully initialized yet.
    */
    #[inline]
    pub fn finish(self) -> [T; N] {
        match self.try_finish() {
            Ok(array) => array,
            Err(..) => panic!("ArrayBuilder::finish incomplete"),
        }
    }

    /**
    Get the slice of the array that has already been initialized.
    */
    #[inline]
    pub fn finished_slice(&self) -> &[T] {
        self.vec.as_slice()
    }

    /**
    Get the mutable slice of the array that has already been initialized.
    */
    #[inline]
    pub fn finished_slice_mut(&mut self) -> &mut [T] {
        self.vec.as_mut_slice()
    }
}

impl<T, const N: usize> Default for ArrayBuilder<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Debug, const N: usize> Debug for ArrayBuilder<T, N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArrayBuilder")
            .field("array", &self.finished_slice())
            .field("progress", &format_args!("{} / {}", self.len(), N))
            .finish()
    }
}

impl<T, const N: usize> Extend<T> for ArrayBuilder<T, N> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        iter.into_iter().for_each(|item| {
            self.push(item);
        })
    }

    // TODO: extend_one, when it's stable
}
