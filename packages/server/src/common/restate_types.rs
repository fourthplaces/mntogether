//! Shared Restate request/response types used across domains

use serde::{Deserialize, Serialize};

use crate::impl_restate_serde;

/// Empty request for parameterless Restate handlers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyRequest {}

impl_restate_serde!(EmptyRequest);
