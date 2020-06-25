#[macro_use]
macro_rules! timeit {
    ($e:expr) => {{
        let _start = std::time::Instant::now();
        $e();
        eprintln!("Took {:.3} ms", _start.elapsed().as_millis());
    }};
    ($e:expr, $n:tt) => {{
        let _start = std::time::Instant::now();
        $e();
        eprintln!("{} took {:.3} ms", $n, _start.elapsed().as_millis());
    }};
}

#[macro_use]
macro_rules! timeit_ext {
    ($n:ident ( $($args:expr $(,)?)*)) => {{
        let _start = std::time::Instant::now();
        $n($($args,)*);
        eprintln!("Took {:.3} ms", _start.elapsed().as_millis());
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

    #[test]
    fn test_with_name() {
        timeit!(
            || { std::thread::sleep(std::time::Duration::from_secs(1)) },
            "Sleeping"
        );
    }

    #[test]
    fn test_ext() {
        use std::thread::sleep;
        timeit_ext!(sleep(std::time::Duration::from_secs(1)));
    }
}
