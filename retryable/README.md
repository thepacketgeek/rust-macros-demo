# Retryable Macros

This demo will cover two different implementations of similar logic, retrying fallible functions. For a more detailed walkthrough of building these macros, check out the [accompanying blog article](https://thepacketgeek.com/rust/macros/macro-matching-and-nesting/).

- `retry!`
  - Wraps a given function with retry logc (*optional number of retries can be given*)
  - A progression of the previous `timeit!` macro, with added logic defined within the `macro_rules!`
- `retryable!`
  - We'll build a `Retryable` type with flexible `RetryStrategy` options (retries, delay, etc.)
  - `Retryable` can be used without a macro, but requires verbose setup
  - `retryable!` macro will warp the setup logic, offering rules for passing strategy options

## Use Cases
Functions can fail. Some failures are persistent, like trying to open an invalid file path or parsing numeric values out of a string that doesn't contain numbers. Other failures are intermittent, like attempting to read data from a remote server. In the intermittent case it can be useful to have some logic to retry the attempted call in hopes for a successful result. This is exactly what our `retry!` and `retryable!` macros will do!

# First Attempt with `retry!`
The first macro, `retry!`, contains all the retry logic in the macro, expanding around the passed in function or closure.

## _Wrapper macro
To support wrapping closures and functions, and to keep the `retry!` macro_rules implementation clean, we'll create another macro (`_wrapper!`) for the following use-cases:

```rust
let res = retry!(|| { sometimes_fail(10) });
assert!(res.is_ok());

let res = retry!(sometimes_fail, 10; retries = 3);
assert!(res.is_ok());
```

The implementation with match rules for each case looks like:

```rust
macro_rules! _wrapper {
    // Single expression (like a function name or closure)
    ($f:expr) => {{
        $f()
    }};
    // Variadic number of args (Allowing trailing comma)
    ($f:expr, $( $args:expr $(,)? )* ) => {{
        $f( $( $args, )* )
    }};
}
```

## Repeating matches
Something we learned with the `timeit!` macro was that we can match on repeating items, and then add code-expansion for each item. We'll use that same trick here to match on multiple arguments for the case of a function & args being passed into `retry!`:

```rust
macro_rules! _wrapper {
    ($f:expr) => {{ /* code from previous section */ }};
    // Variadic number of args (Allowing trailing comma)
    ($f:expr, $( $args:expr $(,)? )* ) => {{
        $f( $($args,)* )
    }};
}
```

There's a lot going on in this single line so let's break it down:
- `$f:expr`: The function passed in for retrying
- `,`: Comma separator before the function arguments
- `$( .. )*`: Anything in these parentheses can repeat (zero or more times, like `*` in regex)
- `$args:expr`: Capture each repeating expr into `$args`
- `$(,)?`: Allow optional commas (? == 0 or 1 times, like `regex` )

This match rule will capture something like `_wrapper!(my_func, 10, 20)` into something that resembles:
- `$f` == `my_func`
- `$args` == `[10, 20]`

And let's break down the expansion: `$f( $( $args, )* )`:

- `$f( ... )`: Function name, with literal parenthesis wrapping whatever is inside
- `$( ... )*`: Repeat what's inside per expr in `$args`
- `$args,`: Write out an expr, followed by a literal comma

Which expands to:
```rust
my_func(10, 20,)
```

# Second Attempt with `retryable!`
The `retry!` macro contained all the retry logic in the macro. As the logic gains capabilities (like delay time and backoff strategy), the macro code grows and becomes more complex. Another approach to retrying functions is to create code to handle retries outside of macros, and use a macro to make setting up the usage of our retry logic easier.

## Retryable & RetryStrategy
Forgoing macros for a bit, let's setup some retry structs and implementations. First is a `Retryable` struct to contain our function/closure to retry, and a `RetryStrategy` with options for retrying (number of retries, delay, etc.):

```rust
pub struct Retryable<F, T, E>
where
    F: FnMut() -> Result<T, E>,
{
    inner: F,
    strategy: RetryStrategy,
}

/// Specification for how the retryable should behave
pub struct RetryStrategy {
    retries: usize,
    delay: RetryDelay,
}

pub enum RetryDelay {
    Fixed(std::time::Duration),
    // TODO: More options here
}
```

The core of our implementation for this struct looks like the retry logic from `retry!`, although we now use the delay options from `RetryStrategy`.

```rust
impl<F, T, E> Retryable<F, T, E>
where
    F: FnMut() -> Result<T, E>,
{
    /// Start calling the wrapped function, responding to Errors
    /// as the specified strategy dictates
    pub fn try_call(&mut self) -> Result<T, E> {
        let mut retries = self.strategy.retries;
        let mut delay_time = Duration::from_millis(0);
        loop {
            std::thread::sleep(delay_time);
            let res = (self.inner)();
            if res.is_ok() {
                break res;
            }
            if retries > 0 {
                retries -= 1;
                delay_time = self.next_run_time();
                continue;
            }
            break res;
        }
    }

    fn next_run_time(&self) -> Duration {
        match self.strategy.delay {
            RetryDelay::Fixed(delay) => delay,
        }
    }
}
```

Breaking out this logic into the `RetryStrategy` gives us much more flexibility with retrying, but now we have a problem with a more tedius setup:

```rust
let strategy = RetryStrategy::default().with_retries(3).to_owned();
let mut r = Retryable::new(succeed_after!(2), strategy);
let res = r.try_call();
assert!(res.is_ok());
```

## Automating Retryable Setup
Luckily for us we have an awesome tool in the toolbox that we can use to make this setup much easier: a macro! Using some similar matching rules we used with `retry!`, we can setup a very flexible macro to allow for optional specification of retries:

```rust
macro_rules! retryable {
    // Take a closure with retry count
    // ```ignore
    // retryable!(|| { do_something(1, 2, 3, 4) }; retries=2);
    // ```
    ($f:expr; retries=$r:expr) => {{
        let _strategy = RetryStrategy::default().with_retries($r).to_owned();
        let mut _r = Retryable::new($f, _strategy);
        _r.try_call()
    }};
    // Take a function ptr, variadic args, and retry count
    // ```ignore
    // retryable!(my_fallible_func, 0, "something"; retries=5);
    // ```
    ($( $args:expr $(,)? )+; retries=$r:expr) => {{
        retryable!(|| { _wrapper!($($args,)*)}; retries=$r)
    }};
```

Check out the [full implementation](https://github.com/thepacketgeek/rust-macros-demo/blob/master/retryable/src/lib.rs#L174) which adds options for passing `retries` and `delay` args for more advanced usage that is similar our previous `retry!` macro:

```rust
let res = retryable!(sometimes_fail, 10; retries = 15; delay = 1);
assert!(res.is_ok());

let res = retryable!(|| {sometimes_fail(10)}; retries = 15; delay = 1);
assert!(res.is_ok());
```
