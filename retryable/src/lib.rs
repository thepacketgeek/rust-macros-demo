use std::time::Duration;

/// Expand a variadic number of macro args to a function call w/ args
///
/// ```
/// fn double_sum(a: u32, b: u32) -> u32 {
///     (a + b) * 2
/// }
///
/// assert_eq!(_wrapper!(double_sum, 4, 2), 12);
/// assert_eq!(_wrapper!(double_sum, 4, 2,), 12);
/// ```
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

/// A simple retry macro to immediately attempt a function call after failure
///
/// To use, pass a function and arguments:
/// ```ignore
/// retry!(my_fallible_func, 0, "something");
/// ```
/// Default retry count is 3 (3rd failure will return Err())
///
/// Specify a different number of retries like:
/// ```ignore
/// retry!(my_fallible_func, 0, "something"; retries=5);
/// ```
#[macro_export]
macro_rules! retry {
    ($( $args:expr$(,)? )+; retries=$r:literal) => {{
        let mut retries = $r;
        loop {
            let res = _wrapper!($( $args, )*);
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
    // Function & args only, use default of 3 retries
    ($( $args:expr$(,)? )+) => {{
        retry!($( $args, )*; retries = 3)
    }};
}

/// Retryable is an step up from the `retry!()` macro in that it allows for even more
/// customization for:
/// - Number of retries
/// - Failure delay (and interval calculation)
/// - Immediate failure Error types (E.g. only retry for io::Error, otherwise fail immediately)
pub struct Retryable<F, T, E>
where
    F: FnMut() -> Result<T, E>,
{
    inner: F,
    strategy: RetryStrategy,
}

impl<F, T, E> Retryable<F, T, E>
where
    F: FnMut() -> Result<T, E>,
{
    /// Wrap a given function/closure in a Retryable, with a given strategy
    pub fn new(func: F, strategy: RetryStrategy) -> Retryable<F, T, E> {
        Self {
            inner: func,
            strategy,
        }
    }

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

/// Specification for how the retryable should behave
///
/// Retries: The number of times to retry after Err
/// Delay: How long to wait after each Err before retrying
#[derive(Clone, Debug)]
pub struct RetryStrategy {
    retries: usize,
    delay: RetryDelay,
}

impl RetryStrategy {
    pub fn new(retries: usize, delay: RetryDelay) -> Self {
        Self { retries, delay }
    }

    pub fn with_retries(&mut self, retries: usize) -> &mut Self {
        self.retries = retries;
        self
    }

    pub fn with_delay(&mut self, delay: RetryDelay) -> &mut Self {
        self.delay = delay;
        self
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            retries: 3,
            delay: RetryDelay::Fixed(std::time::Duration::from_secs(2)),
        }
    }
}

#[derive(Clone, Debug)]
pub enum RetryDelay {
    Fixed(std::time::Duration),
    // TODO?: Exponential { initial_delay: std::time::Duration },
}

/// A simple retry macro to immediately attempt a function call after failure
///
/// To use, pass a function and arguments:
/// ```ignore
/// retryable!(my_fallible_func, 0, "something");
/// ```
/// Default retry count is 3 (3rd failure will return Err())
///
/// Specify a different number of retries like
/// ```ignore
/// retryable!(my_fallible_func, 0, "something"; retries=5);
/// ```
///
/// Or a delay time (in seconds)
/// ```ignore
/// retryable!(my_fallible_func, 0, "something"; delay=3);
/// ```
///
/// Or Both!
/// ```ignore
/// retryable!(|| { do_something(1, 2, 3, 4) }; retries=2; delay=3);
/// ```
#[macro_export]
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
    // Take a closure with delay time (seconds)
    // ```ignore
    // retryable!(|| { do_something(1, 2, 3, 4) }; delay=2);
    // ```
    ($f:expr; delay=$d:expr) => {{
        let _delay = RetryDelay::Fixed(Duration::from_secs($d));
        let _strategy = RetryStrategy::default().with_delay(_delay).to_owned();
        let mut _r = Retryable::new($f, _strategy);
        _r.try_call()
    }};
    // Take a closure with retry count & delay time (seconds)
    // ```ignore
    // retryable!(|| { do_something(1, 2, 3, 4) }; retries=2; delay=2);
    // ```
    ($f:expr; retries=$r:expr; delay=$d:expr) => {{
        let _delay = RetryDelay::Fixed(Duration::from_secs($d));
        let _strategy = RetryStrategy::default().with_delay(_delay).to_owned();
        let mut _r = Retryable::new($f, _strategy);
        _r.try_call()
    }};
    // Take a closure (default of 3 retries)
    // ```ignore
    // retryable!(|| { do_something(1, 2, 3, 4) });
    // ```
    ($f:expr) => {{
        retryable!($f; retries = 3)
    }};
    // Take a function ptr with variadic args (default of 3 retries)
    // ```ignore
    // retryable!(my_fallible_func, 0, "something"; retries=5);
    // ```
    ($($args:expr$(,)?)+) => {{
        retryable($($args)*; retries=3)
    }};
    // Take a function ptr, variadic args, and retry count
    // ```ignore
    // retryable!(my_fallible_func, 0, "something"; retries=5);
    // ```
    ($($args:expr$(,)?)+; retries=$r:expr) => {{
        retryable!(|| { _wrapper!($($args,)*)}; retries=$r)

        // This rule can hit the recursion limit for macros
        // If that's a problem, we can remove some recursion like:
        // let _strategy = RetryStrategy::default().with_retries($r).to_owned();
        // let mut _r = Retryable::new(|| { _wrapper!($($args,)*)}, _strategy);
        // _r.try_call()
    }};
    // Take a function ptr, variadic args, and delay time (seconds)
    // ```ignore
    // retryable!(my_fallible_func, 0, "something"; delay=5);
    // ```
    ($($args:expr$(,)?)+; delay=$d:expr) => {{
        retryable!(|| { _wrapper!($($args,)*)}; delay=$d)
    }};
    // Take a function ptr, variadic args, retry count, and delay time (seconds)
    // ```ignore
    // retryable!(my_fallible_func, 0, "something"; retries=2; delay=5);
    // ```
    ($($args:expr$(,)?)+; retries=$r:expr; delay=$d:expr) => {{
        retryable!(|| { _wrapper!($($args,)*)}; retries=$r; delay=$d)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use std::time::Instant;

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
            move || {
                if _iter.next().is_some() {
                    return Err(());
                }
                Ok(())
            }
        }};
    }

    /// Test helper function
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

    #[test]
    fn test_wrapper() {
        // Closure that fails 1 time, followed by success
        fn one() -> u32 {
            1
        }
        fn double(a: u32) -> u32 {
            a + a
        }
        fn add(a: u32, b: u32) -> u32 {
            a + b
        }

        assert_eq!(_wrapper!(one), 1);
        assert_eq!(_wrapper!(double, 2), 4);
        assert_eq!(_wrapper!(double, 2,), 4);
        assert_eq!(_wrapper!(add, 2, 4), 6);
        assert_eq!(_wrapper!(add, 2, 4,), 6);
    }

    /// Sanity check for how I'm checking eventual success
    #[test]
    fn test_succeed_after() {
        let mut eventually_succeed = succeed_after!(3);
        assert!(eventually_succeed().is_err());
        assert!(eventually_succeed().is_err());
        assert!(eventually_succeed().is_err());
        assert!(eventually_succeed().is_ok());
        assert!(eventually_succeed().is_ok());
    }

    #[test]
    fn test_retry_default() {
        let mut eventually_succeed = succeed_after!(1);
        let res = retry!(eventually_succeed);
        assert!(res.is_ok());

        let fallible = || sometimes_fail(10);
        let res = retry!(fallible);
        assert!(res.is_ok());

        let res = retry!(sometimes_fail, 10);
        assert!(res.is_ok());
    }

    #[test]
    fn test_retry_retries_fail() {
        let res = retry!(succeed_after!(3); retries = 2);
        assert!(res.is_err());

        let will_always_fail = || -> Result<(), ()> { Err(()) };
        let res = retry!(will_always_fail);
        assert!(res.is_err());
    }

    #[test]
    fn test_retry_retries_success() {
        let mut eventually_succeed = succeed_after!(1);
        let res = retry!(eventually_succeed);
        assert!(res.is_ok());
    }

    #[test]
    fn test_retryable_simple() {
        let strategy = RetryStrategy::default().with_retries(3).to_owned();
        let mut r = Retryable::new(succeed_after!(2), strategy);
        let res = r.try_call();
        assert!(res.is_ok());
    }

    #[test]
    fn test_retryable_macro() {
        let start = Instant::now();
        let eventually_succeed = succeed_after!(2);
        let res = retryable!(eventually_succeed);
        assert!(res.is_ok());
        assert!(start.elapsed() > Duration::from_secs(3));
    }

    #[test]
    fn test_retryable_macro_args_retries() {
        let res = retryable!(sometimes_fail, 10; retries = 15; delay = 1);
        assert!(res.is_ok());
        let res = retryable!(|| {sometimes_fail(10)}; retries = 15; delay = 1);
        assert!(res.is_ok());
    }

    #[test]
    fn test_retryable_macro_args_delay() {
        let start = Instant::now();
        let res = retryable!(succeed_after!(2); delay=3);
        assert!(res.is_ok());
        assert!(start.elapsed() > Duration::from_secs(6));

        let start = Instant::now();
        let res = retryable!(succeed_after!(2); delay=3);
        assert!(res.is_ok());
        assert!(start.elapsed() > Duration::from_secs(6));
    }
}
