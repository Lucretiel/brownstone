/*!
`brownstone` is a library for building fixed-size arrays. It provides a
collection of optimizer-friendly fallible and infallible function that build
arrays by calling initializer functions in a loop. It also provides a low-level
[builder type][builder::ArrayBuilder], with a [`push`][builder::ArrayBuilder::push`]
+ [`finish`][builder::ArrayBuilder::finish] interface, as well as a [misuse-
immune builder type][move_builder::ArrayBuilder] with a move-based interface
that can never panic or return errors.
*/

#![cfg_attr(not(feature = "std"), no_std)]

pub mod builder;
pub mod move_builder;

use core::{
    convert::Infallible,
    fmt::{self, Display, Formatter},
};

use move_builder::{ArrayBuilder, PushResult};

/// Error returned from the fallible `try_build_*` functions in this crate.
/// This includes the original error returned from the input function, along
/// with the index where the error occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TryBuildError<E> {
    pub error: E,
    pub index: usize,
}

impl<E> Display for TryBuildError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "error building array at index {}", self.index)
    }
}

#[cfg(feature = "std")]
impl<E: std::error::Error + 'static> std::error::Error for TryBuildError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

#[inline]
const fn infallible<T>(value: T) -> Result<T, Infallible> {
    Ok(value)
}

/// Build a fixed-size array with a fallible initializer function. The
/// initializer is called once for each item in the length of the array, in
/// order; if it ever returns an `Err`, that error is propagated (along with
/// the index of the failed item).
///
/// Each time the method is called, it is provided with context in the form of
/// the prefix of the array that has already been initialized.
#[inline]
pub fn try_build_with<T, F, E, const N: usize>(
    mut next_value: F,
) -> Result<[T; N], TryBuildError<E>>
where
    F: for<'a> FnMut(&'a mut [T]) -> Result<T, E>,
{
    match ArrayBuilder::start() {
        PushResult::Full(array) => Ok(array),
        PushResult::NotFull(mut builder) => loop {
            let value =
                next_value(builder.finished_slice_mut()).map_err(|error| TryBuildError {
                    error,
                    index: builder.len(),
                })?;

            match builder.push(value) {
                PushResult::Full(array) => break Ok(array),
                PushResult::NotFull(updated) => builder = updated,
            }
        },
    }
}

/**
Build a fixed-size array with an initializer function. The initializer is
called once for each item in the length of the array, and the completed
array is returned.

Each time the method is called, it is provided with context in the form of
the prefix of the array that has already been initialized.

# Example

```
let fib: [i32; 10] = brownstone::build_with(|prefix| match *prefix {
    [] => 0,
    [_] => 1,
    [.., a, b] => a + b,
});

assert_eq!(fib, [0, 1, 1, 2, 3, 5, 8, 13, 21, 34])
```
*/
#[inline]
pub fn build_with<T, F, const N: usize>(mut next_value: F) -> [T; N]
where
    F: for<'a> FnMut(&'a mut [T]) -> T,
{
    match try_build_with(move |slice| infallible(next_value(slice))) {
        Ok(array) => array,
        Err(inf) => match inf.error {},
    }
}

/**
Build a fixed-size array with a fallible initializer function. The
initializer is called once for each item in the length of the array, in
order; if it ever returns an `Err`, that error is propagated (along with
the index of the failed item).

Each time the method is called, it is provided with the index of the
element being produced.
*/
#[inline]
pub fn try_build_indexed<T, F, E, const N: usize>(
    mut next_value: F,
) -> Result<[T; N], TryBuildError<E>>
where
    F: FnMut(usize) -> Result<T, E>,
{
    try_build_with(move |slice| next_value(slice.len()))
}

/**
Build a fixed-size array with an initializer function. The initializer is
called once for each item in the length of the array, and the completed
array is returned.

Each time the method is called, it is provided with the index of the
element being produced.

# Example

```
let array: [usize; 5] = brownstone::build_indexed(|i| i + 1);
assert_eq!(array, [1, 2, 3, 4, 5]);
```

*/
#[inline]
pub fn build_indexed<T, F, const N: usize>(mut next_value: F) -> [T; N]
where
    F: FnMut(usize) -> T,
{
    build_with(move |slice| next_value(slice.len()))
}

/// Build a fixed-size array with a fallible initializer function. The
/// initializer is called once for each item in the length of the array, in
/// order; if it ever returns an `Err`, that error is propagated (along with
/// the index of the failed item).
#[inline]
pub fn try_build<T, F, E, const N: usize>(mut next_value: F) -> Result<[T; N], TryBuildError<E>>
where
    F: FnMut() -> Result<T, E>,
{
    try_build_with(move |_slice| next_value())
}

/**
Build a fixed-size array with an initializer function. The initializer is
called once for each item in the length of the array, and the completed
array is returned.

# Example

```
let array: [String; 5] = brownstone::build(|| format!("Hello"));
assert_eq!(array, ["Hello", "Hello", "Hello", "Hello", "Hello"]);
```
*/
#[inline]
pub fn build<T, F, const N: usize>(mut next_value: F) -> [T; N]
where
    F: FnMut() -> T,
{
    build_with(move |_slice| next_value())
}

/**
Build a fixed-size array from an iterator. The first `N` elements of the
iterator are collected into an array of length `N`. Returns `None` if the
iterator doesn't yield enough elements.

# Example

```
let array: [i32; 5] = brownstone::try_build_iter(1..).unwrap();
assert_eq!(array, [1, 2, 3, 4, 5]);
```

## Iterator too short

```
let array: Option<[i32; 10]> = brownstone::try_build_iter(1..5);
assert!(array.is_none());
```
*/
#[inline]
pub fn try_build_iter<I: IntoIterator, const N: usize>(iterator: I) -> Option<[I::Item; N]> {
    let mut iterator = iterator.into_iter();

    let (_min, max) = iterator.size_hint();

    // Preemptively check if the iterator will be too short.
    if let Some(max) = max {
        if max < N {
            return None;
        }
    }

    try_build(move || iterator.next().ok_or(())).ok()
}

/**
Build a fixed-size array from an iterator. The first `N` elements of the
iterator are collected into an array of length `N`.

# Panics

Panics if the iterator doesn't yield enough elements

# Example

```
let array: [i32; 5] = brownstone::build_iter(1..);
assert_eq!(array, [1, 2, 3, 4, 5]);
```
*/
#[inline]
pub fn build_iter<I: IntoIterator, const N: usize>(iterator: I) -> [I::Item; N] {
    try_build_iter(iterator).expect("build_iter: iterator too short")
}

/**
Build a fixed-size array out of clones of some element. The element itself
is used as the first element in the array.

# Example

```
let array: [Vec<i32>; 4] = brownstone::build_cloned(vec![1, 2, 3]);
assert_eq!(
    array,
    [
        [1, 2, 3],
        [1, 2, 3],
        [1, 2, 3],
        [1, 2, 3],
    ]
)
```
*/
#[inline]
pub fn build_cloned<T: Clone, const N: usize>(mut item: T) -> [T; N] {
    match ArrayBuilder::start() {
        PushResult::Full(array) => array,
        PushResult::NotFull(mut builder) => loop {
            builder = match builder.push(item) {
                PushResult::Full(array) => break array,
                PushResult::NotFull(builder) => builder,
            };
            item = builder.finished_slice()[0].clone();
        },
    }
}
