#[macro_use]
macro_rules! timeit {
    // Attempt to match function name & args
    // ```ignore
    // timeit!(something_slow());
    // ```
    // > 'wait_for_it' took 2000 ms
    ($n:ident ( $($args:expr $(,)?)*)) => {{
        let _start = std::time::Instant::now();
        let _res = $n($($args,)*);
        // Use the function name (ident) in the log
        eprintln!("'{}' took {:.3} ms", stringify!($n), _start.elapsed().as_millis());
        _res
    }};
    // Otherwise take a function by name:
    // ```ignore
    // timeit!(my_func);
    // ```
    // > Took 2000 ms
    ($e:expr) => {{
        let _start = std::time::Instant::now();
        let _res = $e();
        eprintln!("Took {:.3} ms", _start.elapsed().as_millis());
        _res
    }};
    // Otherwise take a function by name, and a log prefix
    // ```ignore
    // timeit!(my_func, "My Func");
    // ```
    // > My Func took 2000 ms
    ($e:expr, $n:tt) => {{
        let _start = std::time::Instant::now();
        let _res = $e();
        eprintln!("{} took {:.3} ms", $n, _start.elapsed().as_millis());
        _res
    }};
}

/// Run `cargo test -- --nocapture` to see stderr output
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        timeit!(|| { std::thread::sleep(std::time::Duration::from_secs(1)) });
    }

    /// Pass a prefix
    #[test]
    fn test_with_name() {
        timeit!(
            || { std::thread::sleep(std::time::Duration::from_secs(1)) },
            "Sleeping"
        );
    }

    #[test]
    fn test_ext() {
        fn wait_for_it() -> String {
            std::thread::sleep(std::time::Duration::from_secs(2));
            return String::from("...Legendary!");
        }
        eprintln!("This is going to be...");
        let res = timeit!(wait_for_it());
        eprintln!("{}", res);
    }
}
