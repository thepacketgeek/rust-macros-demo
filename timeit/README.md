# Timeit Macro

Inspired by Python's [`timeit` module](https://docs.python.org/3.8/library/timeit.html) and decorators that allow for easy timing of functions, this demo will introduce some `macro_rules!` concepts to build our own version for Rust.

## Use cases
Before diving into how the macro is implemented let's look at the ways we can use the macro.

#### Timing a block of code
If you have a series of instructions, you can time the overall execution time by wrapping them in a closure:

```rust
use std::io;
use std::fs::read_to_string;

fn main() -> io::Result<()> {
    let file_contents = read_to_string("path/to/file.txt")?;
    let result = timeit!(|| {
        my_lib::process_file(&file_contents)
    });
    println!("{}", results);
    Ok(())
}
```

#### **`output`**
```
Took 0.150 ms
Results: ...
```

The timing output would be ambiguous if there were multiple uses of `timeit!` here, so an optional log prefix can be passed as well:


```rust
// ...
    let result = timeit!(|| {
        my_lib::process_file(&file_contents)
    }, "Processing file");
// ...
```

#### **`output`**
```
Processing file took 0.150 ms
Results: ...
```

#### Timing a function
If you just want to time the execution of a single function call, that can be done also and the macro will attempt to extract the function name to print in the logging output:

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

#### **`output`**
```
This is going to be...
'wait_for_it' took 2002 ms
...Legendary!
```

Now that we see what this macro is doing, let's dig into how it works.

## Implementing timeit!
The essence of the syntax `timeit!` is trying to create shorthand for is:

```rust
{
    let start_time = std::time::Instant::now();
    // Code to time goes here
    eprintln!("Took {:.3} ms", start_time.elapsed().as_millis());
}
```

## Wrapping a Closure via Matching and Expanding
The first use case has the most straight-forward since the closere is matched in `macro_rules!` as a single `expr` match type:

```rust
macro_rules! timeit {
    // This match captures the `expr` passed in as `$e`,
    // which the macro will assume is callable (E.g. a closure or function)
    ($e:expr) => {{
        // Before calling `$e`, track the current instant
        let _start = std::time::Instant::now();
        // `$e` could return something (or the implied unit struct `()`), so capture that in `_res`
        let _res = $e();
        // Log the elapsed time
        eprintln!("Took {:.3} ms", _start.elapsed().as_millis());
        // and return whatever our closure returned
        _res
    }};
}
```

### Adding log prefix option
The `macro_rules!` match rules work very similarly to Rust's `match` blocks:

- Order is significant, the first match will execute
- The macro input is destructured into the variables given in the match rule

With this info, we can see how to add an optional log prefix in the `timeit!` macro call:

```rust
macro_rules! timeit {
    ($e:expr) => { /* block from above */ };
    // New match rule that recives a `literal` match type, like a `&str`
    ($e:expr, $desc:literal) => {{
        // This is the same as our previous rule
        let _start = std::time::Instant::now();
        let _res = $e();
        // Except that we now use the `$desc` str in our log output
        eprintln!("{} took {:.3} ms", $desc, _start.elapsed().as_millis());
        _res
    }};
}
```

## Timeit for a Function Call
You can see how multiple match rules offer additional call invokations. We can use the ordering significance of match rules to try and match a function call in order to extract the function name and use that for the logging output. For that, let's dive into how match rules can capture an arbitrary number of repeating things (like function arguments):


Given a function call like:

```rust
slow_sum(5, 10)
```

We'll use a match rule like:
```rust
/*  |--- function name (in this case: slow_sum)
    v     v--- the open paren before the function args   */
($n:ident ( $($args:expr),*) )
/*                ^          ^--- the closing paren after the function args)
                  |--- A repeating series of `arg` with non-captured comma separators
*/
```

After capturing, a representation of the matches might look something like:
```
$n = slow_sum
$args = [5, 10]
```

When using the captured `$args` in the replacement block, we can re-assemble them with comma separators again like:

```rust
//                  v--- repeat each arg with following comma
let _res = $n( $($args,)* );
//          ^--- our captured function name
```

... which becomes a now callable version of the original function call passed into the macro:
```rust
let _res = slow_sum( 5, 10, );
```

You might be thinking this is a lot of work just to re-assemble what got passed into the macro and you'd be correct! Although this work was worth it since we can now use the `$n` function name in our logging output:

```rust
macro_rules! timeit {
    // This rule
    ($n:ident ( $($args:expr),*)) => {{
        let _start = std::time::Instant::now();
        let _res = $n( $($args,)* );
        // Use the function name (ident) in the log
        eprintln!("'{}' took {:.3} ms", stringify!($n), _start.elapsed().as_millis());
        //                               ^-- Rust built-in to convert Identifiers to str
        _res
    }};
    ($e:expr) => { /* block from above */ };
    ($e:expr, $desc:literal) => { /* block from above */ };
}
```

## Testing
Check out the [full implementation](src/lib.rs) to see some tests using this new macro. You can also run the tests (and see the logging output):

```sh
$ cargo test -- --nocapture
running 4 tests
This is going to be...
Took 1000 ms
Sleeping took 1000 ms
test tests::test_simple ... ok
test tests::test_with_name ... ok
'slow_sum' took 2000 ms
Slow sum result: 14
'wait_for_it' took 2000 ms
...Legendary!
test tests::test_ext_multiple_args ... ok
test tests::test_ext ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```
