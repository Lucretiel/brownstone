/*!
A misuse-immune array builder. See [`ArrayBuilder`] for details and examples.
*/

use crate::builder;

/**
The result of pushing to an [`ArrayBuilder`]. If the push resulted in a
full array, the array is returned directly; otherwise, the builder is
returned with updated state. See [`ArrayBuilder`] for details and examples.
*/
#[derive(Debug, Clone)]
pub enum PushResult<T, const N: usize> {
    Full([T; N]),
    NotFull(ArrayBuilder<T, N>),
}

/**
Misuse-immune array builder

This `ArrayBuilder` uses move semantics to provide an array builder that
never panics or returns errors. Each call to [`push`][ArrayBuilder::push]
takes `self` by move, and returns either the builder (if it's not full yet)
or the fully initialized array (if it is). The builder therefore can only
exist while the array being built isn't full yet.


```
use brownstone::move_builder::{ArrayBuilder, PushResult};

let builder = match ArrayBuilder::start() {
    PushResult::Full(_) => unreachable!(),
    PushResult::NotFull(builder) => builder,
};

assert!(builder.is_empty());

let builder = match builder.push(5) {
    PushResult::Full(_) => unreachable!(),
    PushResult::NotFull(builder) => builder,
};

assert_eq!(builder.len(), 1);

let builder = match builder.push(6) {
    PushResult::Full(_) => unreachable!(),
    PushResult::NotFull(builder) => builder,
};

assert_eq!(builder.len(), 2);
assert_eq!(builder.finished_slice(), [5, 6]);

let array = match builder.push(7) {
    PushResult::Full(array) => array,
    PushResult::NotFull(_) => unreachable!(),
};

assert_eq!(array, [5, 6, 7]);
```
*/
#[derive(Debug, Clone)]
pub struct ArrayBuilder<T, const N: usize> {
    builder: builder::ArrayBuilder<T, N>,
    // Invariant: while this instance exists, the builder is not full
}

impl<T, const N: usize> ArrayBuilder<T, N> {
    /**
    Create a new [`ArrayBuilder`]. If `N == 0`, immediately return an empty
    array, rather than the builder.
    */
    #[inline]
    pub fn start() -> PushResult<T, N> {
        // Invariant preserved: if N == 0, return the array immediately
        match builder::ArrayBuilder::new().try_finish() {
            Ok(array) => PushResult::Full(array),
            Err(builder) => PushResult::NotFull(Self { builder }),
        }
    }

    /**
    Returns true if there are no initialized elements in the array.
    */
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.builder.is_empty()
    }

    /**
    Returns the number of initialized elements in the array. Guaranteed to
    be less than `N`.
    */
    #[inline]
    pub fn len(&self) -> usize {
        self.builder.len()
    }

    /**
    Add a new initialized element to the array. If this causes the array
    to become fully initialized, the array is returned; otherwise, a new
    builder is returned.
    */
    #[inline]
    pub fn push(self, value: T) -> PushResult<T, N> {
        // Destructure self to ensure that an ArrayBuilder never exists with
        // a full array
        let mut builder = self.builder;

        // The unsafes here have debug_asserts checking their correctness.

        // Invariant presumed: the array is not full, so this push is safe
        match unsafe { builder.push_unchecked(value) } {
            // Invariant preserved: We only create a new ArrayBuilder if the
            // array is not full yet
            builder::PushResult::NotFull => PushResult::NotFull(Self { builder }),
            builder::PushResult::Full => {
                // Invariant preserved: if this push fills the array, the array
                // is returned immediately
                PushResult::Full(unsafe { builder.finish_unchecked() })
            }
        }
    }

    /**
    Get the slice of the array that has already been initialized.
    */
    #[inline]
    pub fn finished_slice(&self) -> &[T] {
        self.builder.finished_slice()
    }

    /**
    Get the mutable slice of the array that has already been initialized.
    */
    #[inline]
    pub fn finished_slice_mut(&mut self) -> &mut [T] {
        self.builder.finished_slice_mut()
    }
}
