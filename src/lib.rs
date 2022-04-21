/*!
`brownstone` is a library for building fixed-size arrays. It provides [`build`],
a macro that builds an array by evaluating an expression once for each element
in the array. It also provides a low-level [builder type][builder::ArrayBuilder],
with a [`push`][builder::ArrayBuilder::push`] +
[`finish`][builder::ArrayBuilder::finish] interface, as well as a
[misuse-immune builder type][move_builder::ArrayBuilder] with a move-based
interface that can never panic or return errors.
*/

#![no_std]

pub mod builder;
pub mod move_builder;

/**
Build an array with an expression.

This macro builds an array by calling an expression once for each element
in the array:

```rust
use brownstone::build;

let x: [String; 3] = build!["hello".to_owned()];
assert_eq!(x, ["hello", "hello", "hello"])
```

You can also provide an explicit length in the macro, if the length can't be
inferred from context:

```rust
use brownstone::build;

let x = build!["hello".to_owned(); 3];
assert_eq!(x, ["hello", "hello", "hello"]);
```

If needed, you can use a closure syntax (with a `usize` parameter) to evaluate
your expression with the index of the item being evaluated:

```rust
use brownstone::build;

let x = build!(|index: usize| (index as i32) * 2; 4);
assert_eq!(x, [0, 2, 4, 6]);
```

You can also instead use an `&[T]` parameter to to evaluate your expression with
the full prefix of the array that has already been built:

```rust
use brownstone::build;

let x = build!(|prefix: &[i32]| match prefix {
    [.., a, b] => a + b,
    _ => 1,
});

assert_eq!(x, [1, 1, 2, 3, 5, 8, 13]);
```

All of these forms (even the closure-style forms) are evaluated locally, which
means that you can use arbitrary* control flow inside the builder, such as `?`
or `return` or `await`:

```
use std::{io, num};
use brownstone::build;

#[derive(Debug)]
enum Error {
    Io(io::Error),
    Parse(num::ParseIntError),
}

fn read_4_ints(mut input: impl io::BufRead) -> Result<[i32; 4], Error> {
    let mut line = String::new();

    let array = build!({
        line.clear();
        input.read_line(&mut line).map_err(Error::Io)?;
        line.trim().parse().map_err(Error::Parse)?
    });

    Ok(array)
}

let data = b"12\n4\n6\n21\n5";
let data = read_4_ints(data.as_slice()).expect("failed to read or parse");
assert_eq!(data, [12, 4, 6, 21]);
```

<sub>* Currently it doesn't support `break` or `continue`; this is a bug and
will be fixed in a future release.</sub>
*/
#[macro_export]
macro_rules! build {
    (|$prefix:ident : &[$type:ty]| $item:expr $(; $len:expr)?) => {{
        use $crate::move_builder::{ArrayBuilder, PushResult};

        let mut builder = ArrayBuilder $(::< $type, $len >)? ::start();

        loop {
            builder = match builder {
                PushResult::Full(array) => break array,
                PushResult::NotFull(builder) => {
                    let $prefix: &[$type] = builder.finished_slice();
                    let item = $item;
                    builder.push(item)
                }
            }
        }
    }};

    (|$index:ident : usize| $item:expr $(; $len:expr)?) => {
        $crate::build!(|prefix: &[_]| {
            let $index = prefix.len();
            $item
        } $(; $len)?)
    };

    ($item:expr $(; $len:expr)?) => {
        $crate::build!(|_prefix: &[_]| $item $(; $len)?)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let array: [&str; 4] = build!["string"];
        assert_eq!(array, ["string", "string", "string", "string"]);
    }

    #[test]
    fn indexed() {
        let array: [i32; 5] = build!(|idx: usize| idx as i32);
        assert_eq!(array, [0, 1, 2, 3, 4]);
    }

    #[test]
    fn prefixed() {
        let array: [_; 8] = build!(|prefix: &[i32]| match prefix {
            [] => 0,
            [_] => 1,
            [.., a, b] => a + b,
        });

        assert_eq!(array, [0, 1, 1, 2, 3, 5, 8, 13]);
    }

    #[test]
    fn control_flow() {
        fn build_iter(iter: impl IntoIterator<Item = i32>) -> Option<[i32; 6]> {
            let mut iter = iter.into_iter();

            Some(build!(iter.next()?))
        }

        assert_eq!(
            build_iter([1, 2, 3, 4, 5, 6, 7, 8, 9]),
            Some([1, 2, 3, 4, 5, 6])
        );

        assert_eq!(build_iter([1, 2, 3]), None);
    }

    #[test]
    fn explicit_len() {
        let array = build!["hello"; 5];

        // Convert to slice to prevent length inference in the assertion
        let array = array.as_slice();

        assert_eq!(array, ["hello", "hello", "hello", "hello", "hello"]);
    }
}
