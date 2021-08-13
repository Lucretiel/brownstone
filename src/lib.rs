/*!
`brownstone` is a library for building fixed-size arrays. It provides a
collection of optimizer-friendly fallible and infallible function that build
arrays by calling initializer functions in a loop. It also provides a low-level
[builder type][builder::ArrayBuilder], with a [`push`][builder::ArrayBuilder::push`]
+ [`finish`][builder::ArrayBuilder::finish] interface, as well as a [misuse-
immune builder type][move_builder::ArrayBuilder] with a move-based interface
that can never panic or return errors.
*/

#![no_std]

pub mod builder;
pub mod move_builder;

use core::convert::Infallible;

/// Error returned from the fallible `try_build_*` functions in this crate.
/// This includes the original error returned from the input function, along
/// with the index where the error occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TryBuildError<E> {
    pub error: E,
    pub index: usize,
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
    use move_builder::{ArrayBuilder, PushResult};

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

/// Build a fixed-size array with an initializer function. The initializer is
/// called once for each item in the length of the array, and the completed
/// array is returned.
///
/// Each time the method is called, it is provided with context in the form of
/// the prefix of the array that has already been initialized.
#[inline]
pub fn build_with<T, F, const N: usize>(mut next_value: F) -> [T; N]
where
    F: for<'a> FnMut(&'a mut [T]) -> T,
{
    match try_build_with(move |slice| -> Result<T, Infallible> { Ok(next_value(slice)) }) {
        Ok(array) => array,
        Err(inf) => match inf.error {},
    }
}

/// Build a fixed-size array with a fallible initializer function. The
/// initializer is called once for each item in the length of the array, in
/// order; if it ever returns an `Err`, that error is propagated (along with
/// the index of the failed item).
///
/// Each time the method is called, it is provided with the index of the
/// element being produced.
#[inline]
pub fn try_build_indexed<T, F, E, const N: usize>(
    mut next_value: F,
) -> Result<[T; N], TryBuildError<E>>
where
    F: FnMut(usize) -> Result<T, E>,
{
    try_build_with(move |slice| next_value(slice.len()))
}

/// Build a fixed-size array with an initializer function. The initializer is
/// called once for each item in the length of the array, and the completed
/// array is returned.
///
/// Each time the method is called, it is provided with the index of the
/// element being produced.
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

/// Build a fixed-size array with an initializer function. The initializer is
/// called once for each item in the length of the array, and the completed
/// array is returned.
#[inline]
pub fn build<T, F, const N: usize>(mut next_value: F) -> [T; N]
where
    F: FnMut() -> T,
{
    build_with(move |_slice| next_value())
}

/// Build a fixed-size array from an iterator. The first `N` elements of the
/// iterator are collected into an array of length `N`. Returns `None` if the
/// iterator doesn't yield enough elements.
#[inline]
pub fn try_build_iter<I: IntoIterator, const N: usize>(iterator: I) -> Option<[I::Item; N]> {
    let mut iterator = iterator.into_iter();

    let (_min, max) = iterator.size_hint();

    // Premptively check if the iterator will be too short.
    if let Some(max) = max {
        if max < N {
            return None;
        }
    }

    try_build(move || iterator.next().ok_or(())).ok()
}

/// Build a fixed-size array from an iterator. The first `N` elements of the
/// iterator are collected into an array of length `N`.
///
/// # Panics
///
/// Panics if the iterator doesn't yield enough elements
#[inline]
pub fn build_iter<I: IntoIterator, const N: usize>(iterator: I) -> [I::Item; N] {
    try_build_iter(iterator).expect("build_iter: iterator too short")
}

/// Build a fixed-size array out of clones of some element. The element itself
/// is used as the first element in the array.
#[inline]
pub fn build_cloned<T: Clone, const N: usize>(item: T) -> [T; N] {
    // The Option here is a bit clunky, but we rely on that the optimizer will
    // clean it all up
    let mut item = Some(item);

    build_with(|slice: &mut [T]| match item.take() {
        Some(item) => item,
        None => slice.first().unwrap().clone(),
    })
}
