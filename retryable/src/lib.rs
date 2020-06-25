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
    ($f:expr) => {{
        $f()
    }};
    // Variadic number of args (Allowing trailing comma)
    ($f:expr, $($args:expr$(,)?)*) => {{
        $f($($args,)*)
    }};
}

/// A simple retry macro to immediately attempt a function call after failure
///
/// To use, pass a function and arguments:
/// ```ignore
/// retry!(my_falible_func, 0, "something");
/// ```
/// Default retry count is 3 (3rd failure will return Err())
///
/// Specify a different number of retries like:
/// ```ignore
/// retry!(my_falible_func, 0, "something"; 5); // 5 retries
/// ```
#[macro_export]
macro_rules! retry {
    ($($args:expr$(,)?)+; count=$r:expr) => {{
        let mut retries = $r;
        loop {
            let res = _wrapper!($($args)*);
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
    ($($args:expr$(,)?)+) => {{
        retry!($($args,)*; count = 3)
    }};
}

/// Retryable is an step up from the `retry!()` macro in that it allows for even more
/// customization for:
/// - Number of retries
/// - Failure delay (and backoff strategy)
/// - Immediate failure Error types (E.g. only retry for io::Error, otherwise fail immediately)
pub struct Retryable<F, T, E>
where
    F: FnMut() -> Result<T, E>,
{
    inner: F,
    policy: RetryPolicy,
}

impl<F, T, E> Retryable<F, T, E>
where
    F: FnMut() -> Result<T, E>,
{
    fn new(func: F, policy: RetryPolicy) -> Retryable<F, T, E> {
        Self {
            inner: func,
            policy,
        }
    }

    fn try_call(&mut self) -> Result<T, E> {
        let mut retries = self.policy.retries;
        loop {
            let res = (self.inner)();
            if res.is_ok() {
                break res;
            }
            if retries > 0 {
                retries -= 1;
                continue;
            }
            break res;
        }
    }
}

/// Specification for how the retryable should behave
///
/// Retries: The number of times to
#[derive(Debug)]
pub struct RetryPolicy {
    retries: usize,
    delay: RetryDelay,
}

impl RetryPolicy {
    pub fn with_retries(count: usize) -> Self {
        Self {
            retries: count,
            ..Default::default()
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            retries: 3,
            delay: RetryDelay::Fixed(std::time::Duration::from_secs(2)),
        }
    }
}

#[derive(Debug)]
pub enum RetryDelay {
    Fixed(std::time::Duration),
    Backoff {
        initial_delay: std::time::Duration,
        multiplier: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_eventual_success() {
        let mut fail_count = 3;
        // Closure that fails 1 time, followed by success
        let mut eventually_succeed = || {
            if fail_count > 0 {
                fail_count -= 1;
                Err(())
            } else {
                Ok(())
            }
        };
        assert!(eventually_succeed().is_err());
        assert!(eventually_succeed().is_err());
        assert!(eventually_succeed().is_err());
        assert!(eventually_succeed().is_ok());
        assert!(eventually_succeed().is_ok());
    }

    #[test]
    fn test_retry_default() {
        let mut should_fail: bool = true;
        // Closure that fails 1 time, followed by success
        let mut subsequently_fail = || {
            if should_fail {
                should_fail = false;
                Err(())
            } else {
                Ok(())
            }
        };

        let res = retry!(subsequently_fail);
        assert!(res.is_ok());
    }

    #[test]
    fn test_retry_count_fail() {
        let mut fail_count = 3;
        // Closure that fails 1 time, followed by success
        let mut subsequently_fail = || {
            if fail_count >= 0 {
                fail_count -= 1;
                Err(())
            } else {
                Ok(())
            }
        };

        let res = retry!(subsequently_fail; count = 2);
        assert!(res.is_err());

        let will_always_fail = || -> Result<(), ()> { Err(()) };
        let res = retry!(will_always_fail);
        assert!(res.is_err());
    }

    #[test]
    fn test_retry_count_success() {
        let mut fail_count = 2;
        // Closure that fails 1 time, followed by success
        let mut subsequently_fail = || {
            if fail_count > 0 {
                println!("fail!");
                fail_count -= 1;
                Err(())
            } else {
                Ok(())
            }
        };
        let res = retry!(subsequently_fail; count = 3);
        assert!(res.is_ok());
    }

    #[test]
    fn test_retryable_simple() {
        let mut fail_count = 2;
        // Closure that fails 1 time, followed by success
        let subsequently_fail = || {
            if fail_count > 0 {
                println!("fail!");
                fail_count -= 1;
                Err(())
            } else {
                Ok(())
            }
        };

        let policy = RetryPolicy::with_retries(3);
        let mut r = Retryable::new(subsequently_fail, policy);
        let res = r.try_call();
        assert!(res.is_ok());
    }
}
