/*!
A misuse-immune array builder
*/

use crate::builder;

/// The result of pushing to an [`ArrayBuilder`]. If the push resulted in a
/// full array, the array is returned directly; otherwise, the builder is
/// returned with updated state.
#[derive(Debug, Clone)]
pub enum PushResult<T, const N: usize> {
    Full([T; N]),
    NotFull(ArrayBuilder<T, N>),
}

/// Misuse-immune array builder
///
/// This `ArrayBuilder` uses move semantics to provide an array builder that
/// never panics or returns errors. Each call to [`push`][ArrayBuilder::push]
/// takes `self` by move, and returns either the builder (if it's not full yet)
/// or the fully initialized array (if it is). The builder therefore can only
/// exist while the array being built isn't full yet.
#[derive(Debug, Clone)]
pub struct ArrayBuilder<T, const N: usize> {
    builder: builder::ArrayBuilder<T, N>,
    // Invariant: while this instance exists, the builder is not full
}

impl<T, const N: usize> ArrayBuilder<T, N> {
    /// Create a new [`ArrayBuilder`]. If `N == 0`, immediately return an empty
    /// array, rather than the builder.
    #[inline]
    pub fn start() -> PushResult<T, N> {
        // Invariant preserved: if N == 0, return the array immediately
        match builder::ArrayBuilder::new().try_finish() {
            Ok(array) => PushResult::Full(array),
            Err(builder) => PushResult::NotFull(Self { builder }),
        }
    }

    /// Returns true if there are no initialized elements in the array
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.builder.is_empty()
    }

    /// Returns the number of initialized elements in the array
    #[inline]
    pub fn len(&self) -> usize {
        self.builder.len()
    }

    /// Add a new initialized element to the array. If this causes the array
    /// to become fully initialzed, the array is returned; otherwise, the
    /// builder is returned.
    #[inline]
    pub fn push(mut self, value: T) -> PushResult<T, N> {
        // The unsafes here have debug_asserts checking their correctness.
        // Invariant preserved: if this push fills the array, the array is
        // returned.
        match unsafe { self.builder.push_unchecked(value) } {
            builder::PushResult::NotFull => PushResult::NotFull(self),
            builder::PushResult::Full => {
                PushResult::Full(unsafe { self.builder.finish_unchecked() })
            }
        }
    }

    /// Get the slice of the array that has already been initialized.
    #[inline]
    pub fn finished_slice(&self) -> &[T] {
        self.builder.finished_slice()
    }

    /// Get the mutable slice of the array that has already been initialized.
    #[inline]
    pub fn finished_slice_mut(&mut self) -> &mut [T] {
        self.builder.finished_slice_mut()
    }
}
