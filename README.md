# Rust Macros Demo

## [Timeit](./timeit)

First up is a macro called `timeit!()` which allows you to wrap a function and print out a message of how long it took. This will cover the basics with `macro_rules!` and using multiple match rules for flexibility with macro usage;

## [Retryable](./retryable)

Next we'll get a bit more advanced with variadic arguments to allow retrying of a function like: `retry!(do_something_maybe())`

# Resources

This is just a small intro to building macros and there are some great resources for diving in and learning more!

- [The Rust Book: Macros](https://doc.rust-lang.org/1.7.0/book/macros.html)
- [The Little Book of Rust Macros](https://danielkeep.github.io/tlborm/book/README.html)
- [Crust of Rust: Macros](https://www.youtube.com/watch?v=q6paRBbLgNw&t=4154s)