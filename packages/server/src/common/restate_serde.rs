//! Helper macro for implementing Restate SDK serialization traits
//!
//! This bridges between serde::Serialize/Deserialize and Restate's custom
//! serialization traits without needing the Json<> wrapper.

/// Implement Restate SDK serialization traits for types that already have serde derives.
///
/// This allows types to work directly with Restate workflows without Json<> wrapper.
///
/// # Example
/// ```
/// #[derive(serde::Serialize, serde::Deserialize)]
/// pub struct MyType { /* ... */ }
///
/// impl_restate_serde!(MyType);
/// ```
#[macro_export]
macro_rules! impl_restate_serde {
    ($type:ty) => {
        impl restate_sdk::serde::Serialize for $type {
            type Error = serde_json::Error;

            fn serialize(&self) -> Result<bytes::Bytes, Self::Error> {
                serde_json::to_vec(self).map(bytes::Bytes::from)
            }
        }

        impl restate_sdk::serde::Deserialize for $type {
            type Error = serde_json::Error;

            fn deserialize(bytes: &mut bytes::Bytes) -> Result<Self, Self::Error> {
                serde_json::from_slice(bytes)
            }
        }

        impl restate_sdk::serde::WithContentType for $type {
            fn content_type() -> &'static str {
                "application/json"
            }
        }
    };
}

/// Implement Restate SDK serialization traits for `Vec<T>` where T already has serde derives.
///
/// # Example
/// ```
/// impl_restate_serde_vec!(MyType);
/// // Now Vec<MyType> works as a Restate return type
/// ```
#[macro_export]
macro_rules! impl_restate_serde_vec {
    ($type:ty) => {
        impl restate_sdk::serde::Serialize for Vec<$type> {
            type Error = serde_json::Error;

            fn serialize(&self) -> Result<bytes::Bytes, Self::Error> {
                serde_json::to_vec(self).map(bytes::Bytes::from)
            }
        }

        impl restate_sdk::serde::Deserialize for Vec<$type> {
            type Error = serde_json::Error;

            fn deserialize(bytes: &mut bytes::Bytes) -> Result<Self, Self::Error> {
                serde_json::from_slice(bytes)
            }
        }

        impl restate_sdk::serde::WithContentType for Vec<$type> {
            fn content_type() -> &'static str {
                "application/json"
            }
        }
    };
}

/// Implement Restate SDK serialization traits for `Option<T>` where T already has serde derives.
///
/// # Example
/// ```
/// impl_restate_serde_option!(MyType);
/// // Now Option<MyType> works as a Restate return type
/// ```
#[macro_export]
macro_rules! impl_restate_serde_option {
    ($type:ty) => {
        impl restate_sdk::serde::Serialize for Option<$type> {
            type Error = serde_json::Error;

            fn serialize(&self) -> Result<bytes::Bytes, Self::Error> {
                serde_json::to_vec(self).map(bytes::Bytes::from)
            }
        }

        impl restate_sdk::serde::Deserialize for Option<$type> {
            type Error = serde_json::Error;

            fn deserialize(bytes: &mut bytes::Bytes) -> Result<Self, Self::Error> {
                serde_json::from_slice(bytes)
            }
        }

        impl restate_sdk::serde::WithContentType for Option<$type> {
            fn content_type() -> &'static str {
                "application/json"
            }
        }
    };
}
