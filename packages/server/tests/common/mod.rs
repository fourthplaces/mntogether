// Common test utilities

pub mod fixtures;
pub mod graphql;
pub mod harness;

pub use fixtures::*;
pub use graphql::*;
pub use harness::*;

/// Macro for creating GraphQL variables
#[macro_export]
macro_rules! vars {
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut vars = juniper::Variables::new();
        $(
            vars.insert(
                $key.to_string(),
                juniper::InputValue::scalar($value),
            );
        )*
        vars
    }};
}
