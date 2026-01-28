// Effects (side effects) for organization domain
// - Database operations
// - External API calls (Firecrawl, OpenAI)

pub mod ai_effects;
pub mod scraper_effects;
pub mod submit_effects;
pub mod sync_effects;

pub use ai_effects::*;
pub use scraper_effects::*;
pub use submit_effects::*;
pub use sync_effects::*;
