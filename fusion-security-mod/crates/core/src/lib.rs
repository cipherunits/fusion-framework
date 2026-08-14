//! Core logic for the `security` Fusion module.
//! Keep this crate free of PyO3 / N-API so both bindings can share it.

pub fn hello(name: &str) -> String {
    format!("Hello, {name}! (from security)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {
        assert!(hello("Fusion").contains("security"));
    }
}
