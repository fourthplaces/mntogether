//! Core extraction types - the output of the extraction pipeline.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// The result of an extraction operation.
///
/// Contains the extracted content as markdown, along with metadata about
/// sources, gaps, grounding quality, and any detected conflicts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extraction {
    /// The extracted content as markdown
    pub content: String,

    /// Pages that contributed to this extraction
    pub sources: Vec<Source>,

    /// Machine-readable queries for missing info.
    ///
    /// Each gap contains a search query that can be piped directly to
    /// `search_for_gap()` or `WebSearcher::search()` without reformulation.
    pub gaps: Vec<MissingField>,

    /// How well-grounded is this extraction?
    ///
    /// Replaces arbitrary confidence floats with meaningful categories.
    pub grounding: GroundingGrade,

    /// Contradictions detected across sources.
    ///
    /// The library doesn't resolve conflicts - it exposes them for
    /// application-level resolution.
    pub conflicts: Vec<Conflict>,

    /// Overall status of the extraction.
    ///
    /// Indicates whether the requested information was found, is missing,
    /// or has contradictory data across sources.
    pub status: ExtractionStatus,
}

/// Overall status of an extraction.
///
/// This tells the application at a glance whether the extraction succeeded
/// and what kind of follow-up might be needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExtractionStatus {
    /// The requested information was found in the sources.
    #[default]
    Found,

    /// The requested information is partially available.
    ///
    /// Some fields were extracted, but gaps remain.
    Partial,

    /// The requested information was not found.
    ///
    /// Check `gaps` for details on what's missing and why.
    Missing,

    /// Sources contain contradictory information.
    ///
    /// Check `conflicts` for the specific contradictions.
    Contradictory,
}

impl Extraction {
    /// Create a new extraction with the given content.
    pub fn new(content: String) -> Self {
        Self {
            content,
            sources: Vec::new(),
            gaps: Vec::new(),
            grounding: GroundingGrade::SingleSource,
            conflicts: Vec::new(),
            status: ExtractionStatus::Found,
        }
    }

    /// Create an extraction representing "not found".
    pub fn not_found(gaps: Vec<MissingField>) -> Self {
        Self {
            content: String::new(),
            sources: Vec::new(),
            gaps,
            grounding: GroundingGrade::SingleSource,
            conflicts: Vec::new(),
            status: ExtractionStatus::Missing,
        }
    }

    /// Calculate the extraction status from the current state.
    pub fn calculate_status(&self) -> ExtractionStatus {
        if !self.conflicts.is_empty() {
            return ExtractionStatus::Contradictory;
        }
        if self.content.is_empty() && !self.gaps.is_empty() {
            return ExtractionStatus::Missing;
        }
        if !self.gaps.is_empty() {
            return ExtractionStatus::Partial;
        }
        ExtractionStatus::Found
    }

    /// Update the status based on current state.
    pub fn update_status(&mut self) {
        self.status = self.calculate_status();
    }

    /// Check if extraction needs enrichment (has gaps or is missing).
    pub fn needs_enrichment(&self) -> bool {
        matches!(
            self.status,
            ExtractionStatus::Missing | ExtractionStatus::Partial
        )
    }

    /// Calculate the grounding grade from source analysis.
    pub fn calculate_grounding(
        sources: &[Source],
        conflicts: &[Conflict],
        has_inference: bool,
    ) -> GroundingGrade {
        if !conflicts.is_empty() {
            return GroundingGrade::Conflicted;
        }
        if has_inference {
            return GroundingGrade::Inferred;
        }
        if sources.len() >= 2 {
            return GroundingGrade::Verified;
        }
        GroundingGrade::SingleSource
    }

    /// Check if this extraction has any gaps.
    pub fn has_gaps(&self) -> bool {
        !self.gaps.is_empty()
    }

    /// Check if this extraction has conflicts.
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Check if the extraction is well-grounded (Verified or SingleSource).
    pub fn is_well_grounded(&self) -> bool {
        matches!(
            self.grounding,
            GroundingGrade::Verified | GroundingGrade::SingleSource
        )
    }

    /// Merge another extraction into this one.
    ///
    /// This is the "Synthesis" step in the Detective loop where new information
    /// from gap resolution gets combined with existing knowledge.
    ///
    /// # Behavior
    /// - **Content**: Appends new content with a separator
    /// - **Sources**: Adds new sources, deduplicating by URL
    /// - **Gaps**: Removes gaps that were resolved by new sources
    /// - **Grounding**: Recalculates based on combined sources
    /// - **Conflicts**: Merges conflict lists
    ///
    /// # Example
    /// ```rust,ignore
    /// let mut extraction = index.extract("board members", None).await?;
    ///
    /// // Resolve a gap
    /// let pages = index.search_for_gap(&extraction.gaps[0].query, 5, None).await?;
    /// let supplement = index.extract_from("board members", &pages).await?;
    ///
    /// // Merge the new information
    /// extraction.merge(supplement);
    /// assert!(extraction.sources.len() > 1); // Now has more sources
    /// ```
    pub fn merge(&mut self, other: Extraction) {
        // Track which gaps were resolved by checking which fields the new sources address
        let resolved_fields: HashSet<String> = other
            .sources
            .iter()
            .filter_map(|s| s.metadata.get("resolved_field").cloned())
            .collect();

        // Collect existing URLs for deduplication (owned to avoid lifetime issues)
        let existing_urls: HashSet<String> = self.sources.iter().map(|s| s.url.clone()).collect();

        // Check if we have primary/supporting sources (for role upgrade decision)
        let has_primary_or_supporting = self
            .sources
            .iter()
            .any(|s| s.role == SourceRole::Primary || s.role == SourceRole::Supporting);

        // Append content with separator
        if !other.content.is_empty() {
            if !self.content.is_empty() {
                self.content.push_str("\n\n---\n\n");
            }
            self.content.push_str(&other.content);
        }

        // Add new sources (deduplicate by URL)
        for source in other.sources {
            if !existing_urls.contains(&source.url) {
                // Upgrade role: new sources that corroborate become Corroborating
                let upgraded_source = if has_primary_or_supporting {
                    Source {
                        role: SourceRole::Corroborating,
                        ..source
                    }
                } else {
                    source
                };
                self.sources.push(upgraded_source);
            }
        }

        // Remove resolved gaps
        if !resolved_fields.is_empty() {
            self.gaps
                .retain(|g| !resolved_fields.contains(&g.field.to_lowercase()));
        }

        // Merge conflicts
        self.conflicts.extend(other.conflicts);

        // Recalculate grounding based on combined sources
        // Note: we don't have has_inference here, so we check current grounding
        let has_inference = self.grounding == GroundingGrade::Inferred;
        self.grounding = Self::calculate_grounding(&self.sources, &self.conflicts, has_inference);

        // Update status based on new state
        self.update_status();
    }

    /// Merge multiple extractions.
    pub fn merge_all(&mut self, others: impl IntoIterator<Item = Extraction>) {
        for other in others {
            self.merge(other);
        }
    }

    /// Create a combined extraction from multiple extractions.
    pub fn combine(extractions: impl IntoIterator<Item = Extraction>) -> Self {
        let mut iter = extractions.into_iter();
        let mut combined = iter
            .next()
            .unwrap_or_else(|| Extraction::new(String::new()));
        combined.merge_all(iter);
        combined
    }

    /// Get the URLs of all sources.
    pub fn source_urls(&self) -> Vec<&str> {
        self.sources.iter().map(|s| s.url.as_str()).collect()
    }

    /// Get the count of sources by role.
    pub fn source_count_by_role(&self, role: SourceRole) -> usize {
        self.sources.iter().filter(|s| s.role == role).count()
    }
}

/// How well-grounded is an extraction?
///
/// This replaces arbitrary confidence floats (0.73 vs 0.71 is meaningless)
/// with meaningful categories that tell the application what to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroundingGrade {
    /// Multiple independent sources agree.
    ///
    /// This is the highest quality - cross-referenced information.
    Verified,

    /// Only one page mentioned it.
    ///
    /// Accurate but not cross-referenced. Application may want to
    /// verify important facts.
    SingleSource,

    /// Sources disagree (see conflicts field).
    ///
    /// Application should check the `conflicts` field and decide
    /// how to handle the contradiction.
    Conflicted,

    /// Not explicitly stated, LLM inferred.
    ///
    /// WARNING: This is often hallucination. Application should treat
    /// with skepticism or discard in strict mode.
    Inferred,
}

impl Default for GroundingGrade {
    fn default() -> Self {
        Self::SingleSource
    }
}

/// A source page that contributed to an extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// URL of the source page
    pub url: String,

    /// Page title if available
    pub title: Option<String>,

    /// When the page was fetched
    pub fetched_at: DateTime<Utc>,

    /// Role this source played in the extraction
    pub role: SourceRole,

    /// Application-provided metadata (pass-through)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Source {
    /// Create a new primary source.
    pub fn primary(url: String, fetched_at: DateTime<Utc>) -> Self {
        Self {
            url,
            title: None,
            fetched_at,
            role: SourceRole::Primary,
            metadata: HashMap::new(),
        }
    }

    /// Create a new supporting source.
    pub fn supporting(url: String, fetched_at: DateTime<Utc>) -> Self {
        Self {
            url,
            title: None,
            fetched_at,
            role: SourceRole::Supporting,
            metadata: HashMap::new(),
        }
    }

    /// Set the title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// Role a source played in the extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceRole {
    /// Primary source - main information came from here
    Primary,

    /// Supporting source - additional details
    Supporting,

    /// Corroborating source - confirms information from other sources
    Corroborating,
}

impl Default for SourceRole {
    fn default() -> Self {
        Self::Supporting
    }
}

/// Why a field is missing.
///
/// This helps the application decide whether to pursue external search
/// or accept that the information doesn't exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissingReason {
    /// No sources mentioned this field at all.
    ///
    /// External search might find pages that have the answer.
    NotInSources,

    /// A source exists but the value is redacted/hidden.
    ///
    /// Example: "Contact us for pricing" instead of actual prices.
    Redacted,

    /// The field was explicitly stated as not applicable.
    ///
    /// Example: "We do not accept volunteers at this time."
    NotApplicable,

    /// The information appears to be outdated/removed.
    ///
    /// Example: Page exists but content was removed.
    Stale,

    /// Sources conflict on this field (see `conflicts`).
    Conflicting,
}

impl Default for MissingReason {
    fn default() -> Self {
        Self::NotInSources
    }
}

/// Machine-readable missing field for agent-driven refinement.
///
/// Each missing field contains:
/// - What's missing (`field`)
/// - A search query to find it (`query`)
/// - Why it's missing (`reason`)
///
/// The `query` can be piped directly to `search_for_gap()` or
/// `WebSearcher::search()` without reformulation.
///
/// # Example
///
/// ```rust,ignore
/// let extraction = index.extract("contact info", None).await?;
///
/// for gap in &extraction.gaps {
///     match gap.reason {
///         MissingReason::NotInSources => {
///             // Worth searching externally
///             let urls = searcher.search(&gap.query).await?;
///         }
///         MissingReason::NotApplicable => {
///             // Don't search - it explicitly doesn't exist
///             println!("{} is not applicable", gap.field);
///         }
///         _ => {}
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingField {
    /// Human-readable field name (e.g., "contact email").
    pub field: String,

    /// Search query - pipe directly to `search_for_gap()` or `WebSearcher`.
    ///
    /// Example: "volunteer coordinator email address for Red Cross Portland"
    pub query: String,

    /// Why this field is missing.
    ///
    /// Helps the app decide whether external search is worthwhile.
    pub reason: MissingReason,
}

impl MissingField {
    /// Create a new missing field.
    pub fn new(field: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            query: query.into(),
            reason: MissingReason::NotInSources,
        }
    }

    /// Create with a specific reason.
    pub fn with_reason(mut self, reason: MissingReason) -> Self {
        self.reason = reason;
        self
    }

    /// Create a "not in sources" gap.
    pub fn not_in_sources(field: impl Into<String>, query: impl Into<String>) -> Self {
        Self::new(field, query).with_reason(MissingReason::NotInSources)
    }

    /// Create a "redacted" gap.
    pub fn redacted(field: impl Into<String>, query: impl Into<String>) -> Self {
        Self::new(field, query).with_reason(MissingReason::Redacted)
    }

    /// Create a "not applicable" gap.
    pub fn not_applicable(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            query: String::new(), // No query - won't be found
            reason: MissingReason::NotApplicable,
        }
    }

    /// Check if this gap is worth searching externally.
    pub fn is_searchable(&self) -> bool {
        matches!(
            self.reason,
            MissingReason::NotInSources | MissingReason::Stale
        )
    }
}

/// Alias for backwards compatibility.
pub type GapQuery = MissingField;

/// A detected conflict between sources.
///
/// The library doesn't resolve conflicts - it exposes them.
/// Application decides: "Trust /calendar over /volunteer" or "Flag for human review".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Topic of the conflict (e.g., "Schedule", "Contact Info")
    pub topic: String,

    /// Conflicting claims from different sources
    pub claims: Vec<ConflictingClaim>,
}

impl Conflict {
    /// Create a new conflict.
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            claims: Vec::new(),
        }
    }

    /// Add a conflicting claim.
    pub fn with_claim(
        mut self,
        statement: impl Into<String>,
        source_url: impl Into<String>,
    ) -> Self {
        self.claims.push(ConflictingClaim {
            statement: statement.into(),
            source_url: source_url.into(),
        });
        self
    }
}

/// A single conflicting claim from a source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictingClaim {
    /// The statement being made
    pub statement: String,

    /// URL of the source making this claim
    pub source_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grounding_grade_verified() {
        let sources = vec![
            Source::primary("https://a.com".into(), Utc::now()),
            Source::supporting("https://b.com".into(), Utc::now()),
        ];
        let grade = Extraction::calculate_grounding(&sources, &[], false);
        assert_eq!(grade, GroundingGrade::Verified);
    }

    #[test]
    fn test_grounding_grade_single_source() {
        let sources = vec![Source::primary("https://a.com".into(), Utc::now())];
        let grade = Extraction::calculate_grounding(&sources, &[], false);
        assert_eq!(grade, GroundingGrade::SingleSource);
    }

    #[test]
    fn test_grounding_grade_conflicted() {
        let sources = vec![
            Source::primary("https://a.com".into(), Utc::now()),
            Source::supporting("https://b.com".into(), Utc::now()),
        ];
        let conflicts = vec![Conflict::new("Schedule")
            .with_claim("Open Monday", "https://a.com")
            .with_claim("Open Tuesday", "https://b.com")];
        let grade = Extraction::calculate_grounding(&sources, &conflicts, false);
        assert_eq!(grade, GroundingGrade::Conflicted);
    }

    #[test]
    fn test_grounding_grade_inferred() {
        let sources = vec![
            Source::primary("https://a.com".into(), Utc::now()),
            Source::supporting("https://b.com".into(), Utc::now()),
        ];
        let grade = Extraction::calculate_grounding(&sources, &[], true);
        assert_eq!(grade, GroundingGrade::Inferred);
    }

    #[test]
    fn test_merge_deduplicates_sources() {
        let mut base = Extraction::new("Base content".to_string());
        base.sources
            .push(Source::primary("https://a.com".into(), Utc::now()));

        let mut supplement = Extraction::new("Supplement content".to_string());
        supplement
            .sources
            .push(Source::supporting("https://a.com".into(), Utc::now())); // Duplicate
        supplement
            .sources
            .push(Source::supporting("https://b.com".into(), Utc::now())); // New

        base.merge(supplement);

        assert_eq!(base.sources.len(), 2); // a.com and b.com
        assert!(base.content.contains("Base content"));
        assert!(base.content.contains("Supplement content"));
    }

    #[test]
    fn test_merge_upgrades_grounding() {
        let mut base = Extraction::new("Base".to_string());
        base.sources
            .push(Source::primary("https://a.com".into(), Utc::now()));
        base.grounding = GroundingGrade::SingleSource;

        let mut supplement = Extraction::new("Supplement".to_string());
        supplement
            .sources
            .push(Source::supporting("https://b.com".into(), Utc::now()));

        base.merge(supplement);

        // With 2 sources, grounding should be Verified
        assert_eq!(base.grounding, GroundingGrade::Verified);
    }

    #[test]
    fn test_merge_upgrades_source_role() {
        let mut base = Extraction::new("Base".to_string());
        base.sources
            .push(Source::primary("https://a.com".into(), Utc::now()));

        let mut supplement = Extraction::new("Supplement".to_string());
        supplement
            .sources
            .push(Source::supporting("https://b.com".into(), Utc::now()));

        base.merge(supplement);

        // New source should be marked as Corroborating
        let new_source = base
            .sources
            .iter()
            .find(|s| s.url == "https://b.com")
            .unwrap();
        assert_eq!(new_source.role, SourceRole::Corroborating);
    }

    #[test]
    fn test_merge_combines_conflicts() {
        let mut base = Extraction::new("Base".to_string());
        base.conflicts.push(Conflict::new("Schedule"));

        let mut supplement = Extraction::new("Supplement".to_string());
        supplement.conflicts.push(Conflict::new("Contact Info"));

        base.merge(supplement);

        assert_eq!(base.conflicts.len(), 2);
    }

    #[test]
    fn test_combine_multiple() {
        let e1 = {
            let mut e = Extraction::new("First".to_string());
            e.sources
                .push(Source::primary("https://1.com".into(), Utc::now()));
            e
        };
        let e2 = {
            let mut e = Extraction::new("Second".to_string());
            e.sources
                .push(Source::primary("https://2.com".into(), Utc::now()));
            e
        };
        let e3 = {
            let mut e = Extraction::new("Third".to_string());
            e.sources
                .push(Source::primary("https://3.com".into(), Utc::now()));
            e
        };

        let combined = Extraction::combine(vec![e1, e2, e3]);

        assert_eq!(combined.sources.len(), 3);
        assert!(combined.content.contains("First"));
        assert!(combined.content.contains("Second"));
        assert!(combined.content.contains("Third"));
        assert_eq!(combined.grounding, GroundingGrade::Verified);
    }

    #[test]
    fn test_extraction_status_found() {
        let extraction = Extraction::new("Some content".to_string());
        assert_eq!(extraction.status, ExtractionStatus::Found);
        assert!(!extraction.needs_enrichment());
    }

    #[test]
    fn test_extraction_status_partial() {
        let mut extraction = Extraction::new("Partial content".to_string());
        extraction
            .gaps
            .push(MissingField::new("email", "contact email"));
        extraction.update_status();

        assert_eq!(extraction.status, ExtractionStatus::Partial);
        assert!(extraction.needs_enrichment());
    }

    #[test]
    fn test_extraction_status_missing() {
        let extraction = Extraction::not_found(vec![
            MissingField::new("email", "contact email"),
            MissingField::new("phone", "phone number"),
        ]);

        assert_eq!(extraction.status, ExtractionStatus::Missing);
        assert!(extraction.needs_enrichment());
    }

    #[test]
    fn test_extraction_status_contradictory() {
        let mut extraction = Extraction::new("Some content".to_string());
        extraction.conflicts.push(
            Conflict::new("Hours")
                .with_claim("Open 9-5", "https://a.com")
                .with_claim("Open 10-6", "https://b.com"),
        );
        extraction.update_status();

        assert_eq!(extraction.status, ExtractionStatus::Contradictory);
    }

    #[test]
    fn test_missing_field_is_searchable() {
        let not_in_sources = MissingField::not_in_sources("email", "contact email");
        assert!(not_in_sources.is_searchable());

        let redacted = MissingField::redacted("pricing", "pricing information");
        assert!(!redacted.is_searchable());

        let not_applicable = MissingField::not_applicable("volunteers");
        assert!(!not_applicable.is_searchable());
    }

    #[test]
    fn test_missing_reason_determines_searchability() {
        // NotInSources and Stale are searchable
        let stale = MissingField::new("data", "query").with_reason(MissingReason::Stale);
        assert!(stale.is_searchable());

        // Redacted, NotApplicable, Conflicting are not searchable
        let conflicting =
            MissingField::new("data", "query").with_reason(MissingReason::Conflicting);
        assert!(!conflicting.is_searchable());
    }

    #[test]
    fn test_gap_query_alias() {
        // GapQuery is an alias for MissingField
        let gap: GapQuery = MissingField::new("field", "query");
        assert_eq!(gap.field, "field");
        assert_eq!(gap.query, "query");
    }
}
