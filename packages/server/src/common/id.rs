//! Typed UUID wrappers for compile-time type safety.
//!
//! This module provides `Id<T, V>`, a typed wrapper around `uuid::Uuid` that prevents
//! accidentally mixing up different ID types (e.g., passing a `MemberId` where a
//! `ContainerId` was expected).
//!
//! # API Compatibility
//!
//! This API is designed to match [typed-uuid](https://crates.io/crates/typed-uuid) while
//! adding support for UUID v7 and sqlx integration.
//!
//! # Example
//!
//! ```rust
//! use common_rs::id::{Id, V7};
//!
//! // Define entity marker types
//! pub struct Member;
//! pub struct Container;
//!
//! // Create type aliases
//! pub type MemberId = Id<Member>;
//! pub type ContainerId = Id<Container>;
//!
//! // These are now incompatible types:
//! let member_id = MemberId::new();
//! let container_id = ContainerId::new();
//!
//! // This would be a compile error:
//! // let wrong: ContainerId = member_id;
//! ```

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::str::FromStr;
use uuid::Uuid;

/// UUID version 7 marker (time-ordered UUIDs).
///
/// This is the recommended version for database primary keys as it provides
/// natural chronological ordering.
pub struct V7;

/// UUID version 4 marker (random UUIDs).
pub struct V4;

/// A typed wrapper around `Uuid` that provides compile-time type safety.
///
/// The type parameter `T` represents the entity type this ID belongs to,
/// and `V` represents the UUID version (defaults to V7).
///
/// # Type Safety
///
/// IDs with different `T` parameters are incompatible at compile time:
///
/// ```compile_fail
/// use common_rs::id::Id;
///
/// struct User;
/// struct Post;
///
/// let user_id: Id<User> = Id::new();
/// let post_id: Id<Post> = user_id; // Compile error!
/// ```
#[repr(transparent)]
pub struct Id<T, V = V7>(Uuid, PhantomData<fn() -> (T, V)>);

// ============================================================================
// Core implementations
// ============================================================================

impl<T> Id<T, V7> {
    /// Creates a new V7 UUID (time-ordered).
    ///
    /// V7 UUIDs are recommended for database primary keys as they provide
    /// natural chronological ordering and better index locality.
    #[inline]
    pub fn new() -> Self {
        Self(Uuid::now_v7(), PhantomData)
    }
}

impl<T> Default for Id<T, V7> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Id<T, V4> {
    /// Creates a new V4 UUID (random).
    #[inline]
    pub fn new() -> Self {
        Self(Uuid::new_v4(), PhantomData)
    }
}

impl<T> Default for Id<T, V4> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, V> Id<T, V> {
    /// Creates an `Id` from a raw `Uuid`.
    ///
    /// This is useful when loading IDs from the database or deserializing.
    #[inline]
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid, PhantomData)
    }

    /// Returns the inner `Uuid`.
    #[inline]
    pub fn into_uuid(self) -> Uuid {
        self.0
    }

    /// Parses an `Id` from a string.
    ///
    /// This is the primary way to convert GraphQL string inputs to typed IDs.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid UUID.
    #[inline]
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?, PhantomData))
    }

    /// Returns a reference to the inner `Uuid`.
    #[inline]
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Creates a nil (all zeros) ID.
    ///
    /// Useful for testing or sentinel values.
    #[inline]
    pub fn nil() -> Self {
        Self(Uuid::nil(), PhantomData)
    }

    /// Returns `true` if this is a nil UUID.
    #[inline]
    pub fn is_nil(&self) -> bool {
        self.0.is_nil()
    }
}

// ============================================================================
// Standard trait implementations
// ============================================================================

impl<T, V> Clone for Id<T, V> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, V> Copy for Id<T, V> {}

impl<T, V> Debug for Id<T, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Include type name for debugging clarity
        f.debug_tuple(&format!("Id<{}>", std::any::type_name::<T>()))
            .field(&self.0)
            .finish()
    }
}

impl<T, V> Display for Id<T, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<T, V> PartialEq for Id<T, V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T, V> Eq for Id<T, V> {}

impl<T, V> PartialOrd for Id<T, V> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T, V> Ord for Id<T, V> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T, V> Hash for Id<T, V> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T, V> AsRef<Uuid> for Id<T, V> {
    #[inline]
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl<T, V> From<Uuid> for Id<T, V> {
    #[inline]
    fn from(uuid: Uuid) -> Self {
        Self::from_uuid(uuid)
    }
}

impl<T, V> From<Id<T, V>> for Uuid {
    #[inline]
    fn from(id: Id<T, V>) -> Self {
        id.0
    }
}

impl<T, V> FromStr for Id<T, V> {
    type Err = uuid::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

// ============================================================================
// Serde support
// ============================================================================

impl<T, V> Serialize for Id<T, V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T, V> Deserialize<'de> for Id<T, V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Uuid::deserialize(deserializer).map(Self::from_uuid)
    }
}

// ============================================================================
// Restate SDK serde support
// ============================================================================

impl<T: Send + Sync + 'static, V: Send + Sync + 'static> restate_sdk::serde::Serialize for Id<T, V> {
    type Error = serde_json::Error;

    fn serialize(&self) -> Result<bytes::Bytes, Self::Error> {
        serde_json::to_vec(&self.0).map(bytes::Bytes::from)
    }
}

impl<T: Send + Sync + 'static, V: Send + Sync + 'static> restate_sdk::serde::Deserialize for Id<T, V> {
    type Error = serde_json::Error;

    fn deserialize(bytes: &mut bytes::Bytes) -> Result<Self, Self::Error> {
        let uuid: Uuid = serde_json::from_slice(bytes)?;
        Ok(Self::from_uuid(uuid))
    }
}

impl<T: Send + Sync + 'static, V: Send + Sync + 'static> restate_sdk::serde::WithContentType for Id<T, V> {
    fn content_type() -> &'static str {
        "application/json"
    }
}

// ============================================================================
// sqlx support (always enabled)
// ============================================================================

use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueRef, Postgres};
use sqlx::{Decode, Encode, Type};

impl<T, V> Type<Postgres> for Id<T, V> {
    fn type_info() -> PgTypeInfo {
        <Uuid as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <Uuid as Type<Postgres>>::compatible(ty)
    }
}

impl<T, V> PgHasArrayType for Id<T, V> {
    fn array_type_info() -> PgTypeInfo {
        <Uuid as PgHasArrayType>::array_type_info()
    }
}

impl<T, V> Encode<'_, Postgres> for Id<T, V> {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <Uuid as Encode<Postgres>>::encode_by_ref(&self.0, buf)
    }
}

impl<T, V> Decode<'_, Postgres> for Id<T, V> {
    fn decode(value: PgValueRef<'_>) -> Result<Self, BoxDynError> {
        <Uuid as Decode<Postgres>>::decode(value).map(Self::from_uuid)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    struct User;

    type UserId = Id<User>;

    #[test]
    fn test_new_creates_unique_ids() {
        let id1 = UserId::new();
        let id2 = UserId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_parse_and_display_roundtrip() {
        let id = UserId::new();
        let s = id.to_string();
        let parsed = UserId::parse(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = UserId::from_uuid(uuid);
        assert_eq!(id.into_uuid(), uuid);
    }

    #[test]
    fn test_serde_roundtrip() {
        let id = UserId::new();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_nil() {
        let id = UserId::nil();
        assert!(id.is_nil());
    }

    #[test]
    fn test_hash_map_key() {
        use std::collections::HashMap;
        let mut map: HashMap<UserId, &str> = HashMap::new();
        let id = UserId::new();
        map.insert(id, "test");
        assert_eq!(map.get(&id), Some(&"test"));
    }

    #[test]
    fn test_ordering() {
        // V7 UUIDs should be time-ordered
        let id1 = UserId::new();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = UserId::new();
        assert!(id1 < id2);
    }

    #[test]
    fn test_debug_includes_type_name() {
        let id = UserId::new();
        let debug = format!("{:?}", id);
        assert!(debug.contains("User"));
    }
}
