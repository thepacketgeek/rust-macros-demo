# Rust Macros Demo

Practical (*but not-exhaustive*) examples of declarative `macro_rules!`. For an introduction check out the [accompanying blog series](https://thepacketgeek.com/rust/macros/).

## [Timeit](./timeit)

First up is a macro called `timeit!()` which allows you to wrap a function and print out a message of how long it took (*inspired by similar `@timeit` decorators in Python*). This demo will cover the basics with `macro_rules!` and using multiple match rules for flexibility with macro usage.

```rust
fn wait_for_it() -> String {
    std::thread::sleep(std::time::Duration::from_secs(2));
    return String::from("...Legendary!");
}

fn main() {
    eprintln!("This is going to be...");
    let res = timeit!(wait_for_it());
    eprintln!("{}", res);
}
```

Outputs:
```sh
This is going to be...
'wait_for_it' took 2002 ms
...Legendary!
```

## [Retryable](./retryable)

Next we'll get a bit more advanced with variadic arguments to allow retrying of a function like: `retry!(do_something_maybe())`. We can use our `macro_rules!` knowledge to build a new macro to help us test:

```rust
/// Macro to make testing retryable easier
///
/// Returns a closure that will fail for the given count,
/// afterwhich Ok(()) is returned for each call
/// ```
/// let eventually_succeed = succeed_after!(1);
/// assert!(eventually_succeed().is_err());
/// assert!(eventually_succeed().is_ok());
/// ```
#[macro_use]
macro_rules! succeed_after {
    ($count:expr) => {{
        let mut _iter = (0..$count).into_iter();
        let _func = move || {
            if let Some(_) = _iter.next() {
                return Err(());
            }
            Ok(())
        };
        _func
    }};
}
```

We'll write two different macros `retry!()` and `retryable!()`, the latter with even more capabilities:

```rust
fn main() {
    let mut eventually_succeed = succeed_after!(2);
    let res = retry!(eventually_succeed); // Default is 3 retries
    assert!(res.is_ok());
}
```

```rust
fn main() {
    let res = retryable!(succeed_after!(3); retries = 5; delay = 1);
    assert!(res.is_ok());
}
```

# Resources

This is just a small intro to building macros and there are some great resources for diving in and learning more!

- [The Rust Book: Macros](https://doc.rust-lang.org/1.7.0/book/macros.html)
- [The Little Book of Rust Macros](https://danielkeep.github.io/tlborm/book/README.html)
- [Crust of Rust: Macros](https://www.youtube.com/watch?v=q6paRBbLgNw&t=4154s)