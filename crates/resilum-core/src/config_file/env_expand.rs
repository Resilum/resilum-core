use std::sync::LazyLock;

use regex::{Captures, Regex};

static PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)(?::-([^}]*))?\}").unwrap());

pub fn expand(input: &str) -> String {
    PATTERN
        .replace_all(input, |caps: &Captures| {
            let var = &caps[1];
            std::env::var(var).unwrap_or_else(|_| caps.get(2).map_or("", |m| m.as_str()).to_owned())
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_set_var() {
        // SAFETY: single-threaded test, no other code observes the env.
        unsafe {
            std::env::set_var("RESILUM_TEST_VAR", "hello");
        }
        assert_eq!(expand("x=${RESILUM_TEST_VAR}"), "x=hello");
    }

    #[test]
    fn falls_back_to_default_when_unset() {
        unsafe {
            std::env::remove_var("RESILUM_TEST_UNSET");
        }
        assert_eq!(
            expand("port=${RESILUM_TEST_UNSET:-10808}"),
            "port=10808"
        );
    }

    #[test]
    fn empty_when_unset_and_no_default() {
        unsafe {
            std::env::remove_var("RESILUM_TEST_NODEF");
        }
        assert_eq!(expand("x=${RESILUM_TEST_NODEF}"), "x=");
    }

    #[test]
    fn leaves_non_matching_dollar_alone() {
        assert_eq!(expand("price $5, $var, $ {x}"), "price $5, $var, $ {x}");
    }

    #[test]
    fn set_var_takes_precedence_over_default() {
        unsafe {
            std::env::set_var("RESILUM_TEST_OVERRIDE", "from-env");
        }
        assert_eq!(
            expand("v=${RESILUM_TEST_OVERRIDE:-fallback}"),
            "v=from-env"
        );
    }
}
