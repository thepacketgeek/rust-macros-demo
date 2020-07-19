# Retryable Macros

This demo will cover two different implementations of similar logic, retrying fallible functions.

- `retry!`
  - Wraps a given function with retry logc (*optional number of retries can be given*)
  - A progression of the previous `timeit!` macro, with added logic defined within the `macro_rules!`
- `retryable!`
  - We'll build a `Retryable` type with flexible `RetryStrategy` options (retries, delay, etc.)
  - `Retryable` can be used without a macro, but requires verbose setup
  - `retryable!` macro will warp the setup logic, offering rules for passing strategy options

## Use Cases
Functions can fail. Some failures are persistent, like trying to open an invalid file path or parsing numeric values out of a string that doesn't contain numbers. Other failures are intermittent, like attempting to read data from a remote server. In the intermittent case it can be useful to have some logic to retry the attempted call in hopes for a successful result. This is exactly what our `retry!` and `retryable!` macros will do!

Here's a function that fails with a given failure rate that we'll use to illustrate the retry functionality:

```rust
use rand::Rng;

/// Given a failure rate percentage (0..=100),
/// fail with that probability
fn sometimes_fail(failure_rate: u8) -> Result<(), ()> {
    assert!(failure_rate <= 100, "Failure rate is a % (0..=100)");
    let mut rng = rand::thread_rng();
    let val = rng.gen_range(0u8, 100u8);
    if val > failure_rate {
        Ok(())
    } else {
        Err(())
    }
}
```

With a given failure rate of 50%, we could hope that retrying the function call would pass given 3 or more retries:
```rust
#[test]
fn test_retry() {
    // Closure invocation
    let fallible = || {
      sometimes_fail(10)
    };
    let res = retry!(fallible; retries = 3);
    assert!(res.is_ok());

    // Alternate func + args invocation
    let res = retry!(sometimes_fail, 10; retries = 3);
    assert!(res.is_ok());
}
```

# A First Attempt
Before we dive into writing our `retry!` macro, let's look at what retrying a fallible function looks like in Rust. A helpful way to approach writing macros is to:

- Write the code in non-macro form
- Look at what parts should/could be parameterized
- Build a macro for a specific use case
- Expand to include additional use cases when it makes sense

## Retry Logic
A pre-req for our retryable logic is that the function or closure the code is retrying should return `Result`. This allows us to check the `Result` variant (`Ok`/`Err`) and retry accordingly. A [example of this is](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=1d7273b8ce7ac487ca5e1e23127c4c42):

```rust
let mut retries = 3;  // How many times to retry on error

let func = || { sometimes_fail(10) };
// Loop until we hit # of retries or success
let res = loop {
    // Run our function, capturing the Result in `res`
    let res = func();
    // Upon success, break out the loop and return the `Result::Ok`
    if res.is_ok() {
        break res;
    }
    // Otherwise, decrement retries and loop around again
    if retries > 0 {
        retries -= 1;
        continue;
    }
    // When retries have been exhausted, finally return the `Result::Err`
    break res;
};

assert!res.is_ok());
```

One very clear parameter for this macro is the function that's being called (in this case, `sometimes_fail`). Turning this into macro form would look something like:

```rust
macro_rules! retry {
    ($f:expr) => {{
        let mut retries = 3;
        loop {
            let res = $f();
            if res.is_ok() {
                break res;
            }
            if retries > 0 {
                retries -= 1;
                continue;
            }
            break res;
        }
    }};
}
```

This mostly looks similar to our non-macro code above, but I'll explain the match rule `($f:expr)` a bit. This rule will only allow a single expression to be passed into the macro. Additionally, since we're eventually calling the `expr` like `$f()`, the expression must be something that results in a function. So a closure seems like a perfect fit and this macro can be used like ([playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=4c42053cf00e6affb9e26ab5acb163d9)):

```rust
let res = retry!(|| sometimes_fail(10));
assert!res.is_ok());
```

Currently, we can't pass a function directly (E.g. `retry!(sometimes_fail(10)`) as the macro expansion would end up like:

```rust
// ...
    let res = sometimes_fail(10)();
// ...
```

And we can't call the `Result` that `sometimes_fail(10)` returns. To make this work we should look at using yet another macro to coerce closures & functions into a common form for the `retry!` macro.

## Nesting Macros
To keep the `retry!` macro_rules implementation clean, we'll create another macro (`_wrapper!`) to faciliate the passing of closures **or** functions with arguments which will add this additional use case to our examples above:

```rust
// Alternate func + args invocation
let res = retry!(sometimes_fail(10));
assert!(res.is_ok());
```

Let's implement the case covered in `retry!` already:

```rust
macro_rules! _wrapper {
    // Single expression (like a function name or closure)
    ($f:expr) => {{
        $f()
    }};
}
```

I'm using `_wrapper` as the name here to signal that it's intended to be use internally by the `retry!` macro and won't be exported by this library (perhaps a bad habit coming from Python). We can now use this example to get the same functionality as the prior `retry!` macro example:

```rust
macro_rules! retry {
    ($f:expr) => {{
        let mut retries = 3;
        loop {
            let res = _wrapper!($f);
            if res.is_ok() {
                break res;
            }
            if retries > 0 {
                retries -= 1;
                continue;
            }
            break res;
        }
    }};
}
```

As far as functionality goes, nothing has been added here, but this will enable us to build matching logic for our multiple use-cases within `_wrapper!` instead of duplicating code in `retry!` for different match rules.

### Repeating matches
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

With these two match rules inside `_wrapper!`, we can now successfully use `retry!` with all of the use cases! Check out the [final implementation](https://github.com/thepacketgeek/rust-macros-demo/blob/master/retryable/src/lib.rs#L39) and [accompanying tests here](https://github.com/thepacketgeek/rust-macros-demo/blob/master/retryable/src/lib.rs#L326).
