//! Editions service (stateless)
//!
//! CRUD for editions, counties, templates. Layout engine generation.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domains::editions::activities;
use crate::domains::editions::models::county::County;
use crate::domains::editions::models::edition::{Edition, EditionFilters};
use crate::domains::editions::models::edition_row::EditionRow;
use crate::domains::editions::models::edition_slot::EditionSlot;
use crate::domains::editions::models::edition_widget::EditionWidget;
use crate::domains::editions::models::post_template_config::PostTemplateConfig;
use crate::domains::editions::models::row_template_config::RowTemplateConfig;
use crate::domains::editions::models::row_template_slot::RowTemplateSlot;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCountiesRequest {}
impl_restate_serde!(ListCountiesRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCountyRequest {
    pub id: Uuid,
}
impl_restate_serde!(GetCountyRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListEditionsRequest {
    pub county_id: Option<Uuid>,
    pub status: Option<String>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}
impl_restate_serde!(ListEditionsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetEditionRequest {
    pub id: Uuid,
}
impl_restate_serde!(GetEditionRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentEditionRequest {
    pub county_id: Uuid,
}
impl_restate_serde!(CurrentEditionRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEditionRequest {
    pub county_id: Uuid,
    pub period_start: String, // "2026-02-24"
    pub period_end: String,   // "2026-03-02"
    pub title: Option<String>,
}
impl_restate_serde!(CreateEditionRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateEditionRequest {
    pub id: Uuid,
}
impl_restate_serde!(GenerateEditionRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishEditionRequest {
    pub id: Uuid,
}
impl_restate_serde!(PublishEditionRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEditionRequest {
    pub id: Uuid,
}
impl_restate_serde!(ArchiveEditionRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGenerateRequest {
    pub period_start: String,
    pub period_end: String,
}
impl_restate_serde!(BatchGenerateRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTemplatesRequest {}
impl_restate_serde!(ListTemplatesRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEditionRowRequest {
    pub row_id: Uuid,
    pub row_template_slug: Option<String>,
    pub sort_order: Option<i32>,
}
impl_restate_serde!(UpdateEditionRowRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderRowsRequest {
    pub edition_id: Uuid,
    pub row_ids: Vec<Uuid>,
}
impl_restate_serde!(ReorderRowsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovePostFromEditionRequest {
    pub slot_id: Uuid,
}
impl_restate_serde!(RemovePostFromEditionRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSlotTemplateRequest {
    pub slot_id: Uuid,
    pub post_template: String,
}
impl_restate_serde!(ChangeSlotTemplateRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveSlotRequest {
    pub slot_id: Uuid,
    pub target_row_id: Uuid,
    pub slot_index: i32,
}
impl_restate_serde!(MoveSlotRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPostToEditionRequest {
    pub edition_row_id: Uuid,
    pub post_id: Uuid,
    pub post_template: String,
    pub slot_index: i32,
}
impl_restate_serde!(AddPostToEditionRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEditionRowRequest {
    pub edition_id: Uuid,
    pub row_template_slug: String,
    pub sort_order: i32,
}
impl_restate_serde!(AddEditionRowRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteEditionRowRequest {
    pub row_id: Uuid,
}
impl_restate_serde!(DeleteEditionRowRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewEditionRequest {
    pub id: Uuid,
}
impl_restate_serde!(ReviewEditionRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveEditionRequest {
    pub id: Uuid,
}
impl_restate_serde!(ApproveEditionRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchApproveEditionsRequest {
    pub ids: Vec<Uuid>,
}
impl_restate_serde!(BatchApproveEditionsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPublishEditionsRequest {
    pub ids: Vec<Uuid>,
}
impl_restate_serde!(BatchPublishEditionsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionKanbanStatsRequest {
    pub period_start: String,
    pub period_end: String,
}
impl_restate_serde!(EditionKanbanStatsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddWidgetRequest {
    pub edition_row_id: Uuid,
    pub widget_type: String,
    pub slot_index: i32,
    pub config: serde_json::Value,
}
impl_restate_serde!(AddWidgetRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWidgetRequest {
    pub id: Uuid,
    pub config: serde_json::Value,
}
impl_restate_serde!(UpdateWidgetRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveWidgetRequest {
    pub id: Uuid,
}
impl_restate_serde!(RemoveWidgetRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountyResult {
    pub id: Uuid,
    pub fips_code: String,
    pub name: String,
    pub state: String,
}
impl_restate_serde!(CountyResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountyListResult {
    pub counties: Vec<CountyResult>,
}
impl_restate_serde!(CountyListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionResult {
    pub id: Uuid,
    pub county_id: Uuid,
    pub title: Option<String>,
    pub period_start: String,
    pub period_end: String,
    pub status: String,
    pub published_at: Option<String>,
    pub created_at: String,
}
impl_restate_serde!(EditionResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionListResult {
    pub editions: Vec<EditionResult>,
    pub total_count: i64,
}
impl_restate_serde!(EditionListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionDetailResult {
    pub edition: EditionResult,
    pub rows: Vec<EditionRowResult>,
}
impl_restate_serde!(EditionDetailResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionRowResult {
    pub id: Uuid,
    pub row_template_slug: String,
    pub row_template_id: Uuid,
    pub row_template_display_name: String,
    pub row_template_description: Option<String>,
    pub row_template_slots: Vec<RowTemplateSlotResult>,
    pub sort_order: i32,
    pub slots: Vec<EditionSlotResult>,
    pub widgets: Vec<EditionWidgetResult>,
}
impl_restate_serde!(EditionRowResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionWidgetResult {
    pub id: Uuid,
    pub widget_type: String,
    pub slot_index: i32,
    pub config: serde_json::Value,
}
impl_restate_serde!(EditionWidgetResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionSlotResult {
    pub id: Uuid,
    pub post_id: Uuid,
    pub post_template: String,
    pub slot_index: i32,
    // Embedded post data (avoids N+1 from GraphQL)
    pub post_title: Option<String>,
    pub post_post_type: Option<String>,
    pub post_weight: Option<String>,
    pub post_status: Option<String>,
}
impl_restate_serde!(EditionSlotResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowTemplateResult {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub slots: Vec<RowTemplateSlotResult>,
}
impl_restate_serde!(RowTemplateResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowTemplateSlotResult {
    pub slot_index: i32,
    pub weight: String,
    pub count: i32,
    pub accepts: Option<Vec<String>>,
}
impl_restate_serde!(RowTemplateSlotResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowTemplateListResult {
    pub templates: Vec<RowTemplateResult>,
}
impl_restate_serde!(RowTemplateListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostTemplateResult {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub compatible_types: Vec<String>,
    pub body_target: i32,
    pub body_max: i32,
    pub title_max: i32,
}
impl_restate_serde!(PostTemplateResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostTemplateListResult {
    pub templates: Vec<PostTemplateResult>,
}
impl_restate_serde!(PostTemplateListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGenerateEditionsResult {
    pub created: i32,
    pub failed: i32,
    pub total_counties: i32,
}
impl_restate_serde!(BatchGenerateEditionsResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderRowsResult {
    pub rows: Vec<EditionRowResult>,
}
impl_restate_serde!(ReorderRowsResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEditionsResult {
    pub succeeded: i32,
    pub failed: i32,
}
impl_restate_serde!(BatchEditionsResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionKanbanStatsResult {
    pub draft: i32,
    pub in_review: i32,
    pub approved: i32,
    pub published: i32,
}
impl_restate_serde!(EditionKanbanStatsResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Editions"]
pub trait EditionsService {
    async fn list_counties(req: ListCountiesRequest) -> Result<CountyListResult, HandlerError>;
    async fn get_county(req: GetCountyRequest) -> Result<CountyResult, HandlerError>;
    async fn list_editions(req: ListEditionsRequest) -> Result<EditionListResult, HandlerError>;
    async fn get_edition(req: GetEditionRequest) -> Result<EditionDetailResult, HandlerError>;
    async fn current_edition(
        req: CurrentEditionRequest,
    ) -> Result<EditionDetailResult, HandlerError>;
    async fn create_edition(req: CreateEditionRequest) -> Result<EditionResult, HandlerError>;
    async fn generate_edition(req: GenerateEditionRequest) -> Result<EditionResult, HandlerError>;
    async fn publish_edition(req: PublishEditionRequest) -> Result<EditionResult, HandlerError>;
    async fn archive_edition(req: ArchiveEditionRequest) -> Result<EditionResult, HandlerError>;
    async fn batch_generate(
        req: BatchGenerateRequest,
    ) -> Result<BatchGenerateEditionsResult, HandlerError>;
    async fn row_templates(req: ListTemplatesRequest) -> Result<RowTemplateListResult, HandlerError>;
    async fn post_templates(
        req: ListTemplatesRequest,
    ) -> Result<PostTemplateListResult, HandlerError>;
    async fn update_edition_row(
        req: UpdateEditionRowRequest,
    ) -> Result<EditionRowResult, HandlerError>;
    async fn reorder_rows(req: ReorderRowsRequest) -> Result<ReorderRowsResult, HandlerError>;
    async fn remove_post(req: RemovePostFromEditionRequest) -> Result<bool, HandlerError>;
    async fn change_slot_template(
        req: ChangeSlotTemplateRequest,
    ) -> Result<EditionSlotResult, HandlerError>;
    async fn move_slot(req: MoveSlotRequest) -> Result<EditionSlotResult, HandlerError>;
    async fn add_post_to_edition(
        req: AddPostToEditionRequest,
    ) -> Result<EditionSlotResult, HandlerError>;
    async fn add_edition_row(req: AddEditionRowRequest) -> Result<EditionRowResult, HandlerError>;
    async fn delete_edition_row(req: DeleteEditionRowRequest) -> Result<bool, HandlerError>;
    async fn review_edition(req: ReviewEditionRequest) -> Result<EditionResult, HandlerError>;
    async fn approve_edition(req: ApproveEditionRequest) -> Result<EditionResult, HandlerError>;
    async fn batch_approve_editions(
        req: BatchApproveEditionsRequest,
    ) -> Result<BatchEditionsResult, HandlerError>;
    async fn batch_publish_editions(
        req: BatchPublishEditionsRequest,
    ) -> Result<BatchEditionsResult, HandlerError>;
    async fn edition_kanban_stats(
        req: EditionKanbanStatsRequest,
    ) -> Result<EditionKanbanStatsResult, HandlerError>;
    async fn add_widget(req: AddWidgetRequest) -> Result<EditionWidgetResult, HandlerError>;
    async fn update_widget(req: UpdateWidgetRequest) -> Result<EditionWidgetResult, HandlerError>;
    async fn remove_widget(req: RemoveWidgetRequest) -> Result<bool, HandlerError>;
}

// =============================================================================
// Implementation
// =============================================================================

pub struct EditionsServiceImpl {
    deps: Arc<ServerDeps>,
}

impl EditionsServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }

    fn edition_to_result(e: &Edition) -> EditionResult {
        EditionResult {
            id: e.id,
            county_id: e.county_id,
            title: e.title.clone(),
            period_start: e.period_start.to_string(),
            period_end: e.period_end.to_string(),
            status: e.status.clone(),
            published_at: e.published_at.map(|t| t.to_rfc3339()),
            created_at: e.created_at.to_rfc3339(),
        }
    }

    async fn load_edition_detail(&self, edition: &Edition) -> Result<EditionDetailResult, HandlerError> {
        let pool = &self.deps.db_pool;
        let rows = EditionRow::find_by_edition(edition.id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        // Load all templates + slots upfront (2 queries total, avoids N+1)
        let all_templates = RowTemplateConfig::find_all(pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;
        let all_template_slots = RowTemplateSlot::find_all(pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let mut row_results = Vec::new();
        for row in &rows {
            let template = all_templates.iter().find(|t| t.id == row.row_template_config_id);
            let template_slot_results: Vec<RowTemplateSlotResult> = all_template_slots
                .iter()
                .filter(|s| s.row_template_config_id == row.row_template_config_id)
                .map(|s| RowTemplateSlotResult {
                    slot_index: s.slot_index,
                    weight: s.weight.clone(),
                    count: s.count,
                    accepts: s.accepts.clone(),
                })
                .collect();

            let slots = EditionSlot::find_by_row_with_posts(row.id, pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

            let widgets = EditionWidget::find_by_row(row.id, pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

            row_results.push(EditionRowResult {
                id: row.id,
                row_template_slug: template.map(|t| t.slug.clone()).unwrap_or_default(),
                row_template_id: row.row_template_config_id,
                row_template_display_name: template.map(|t| t.display_name.clone()).unwrap_or_default(),
                row_template_description: template.and_then(|t| t.description.clone()),
                row_template_slots: template_slot_results,
                sort_order: row.sort_order,
                slots: slots
                    .iter()
                    .map(|s| EditionSlotResult {
                        id: s.id,
                        post_id: s.post_id,
                        post_template: s.post_template.clone(),
                        slot_index: s.slot_index,
                        post_title: Some(s.post_title.clone()),
                        post_post_type: s.post_post_type.clone(),
                        post_weight: s.post_weight.clone(),
                        post_status: Some(s.post_status.clone()),
                    })
                    .collect(),
                widgets: widgets
                    .iter()
                    .map(|w| EditionWidgetResult {
                        id: w.id,
                        widget_type: w.widget_type.clone(),
                        slot_index: w.slot_index,
                        config: w.config.clone(),
                    })
                    .collect(),
            });
        }

        Ok(EditionDetailResult {
            edition: Self::edition_to_result(edition),
            rows: row_results,
        })
    }
}

impl EditionsService for EditionsServiceImpl {
    async fn list_counties(
        &self,
        _ctx: Context<'_>,
        _req: ListCountiesRequest,
    ) -> Result<CountyListResult, HandlerError> {
        let counties = County::find_all(&self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(CountyListResult {
            counties: counties
                .iter()
                .map(|c| CountyResult {
                    id: c.id,
                    fips_code: c.fips_code.clone(),
                    name: c.name.clone(),
                    state: c.state.clone(),
                })
                .collect(),
        })
    }

    async fn get_county(
        &self,
        _ctx: Context<'_>,
        req: GetCountyRequest,
    ) -> Result<CountyResult, HandlerError> {
        let county = County::find_by_id(req.id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new(format!("County not found: {}", req.id)))?;

        Ok(CountyResult {
            id: county.id,
            fips_code: county.fips_code,
            name: county.name,
            state: county.state,
        })
    }

    async fn list_editions(
        &self,
        _ctx: Context<'_>,
        req: ListEditionsRequest,
    ) -> Result<EditionListResult, HandlerError> {
        let period_start = req
            .period_start
            .as_deref()
            .map(|s| s.parse::<NaiveDate>())
            .transpose()
            .map_err(|e| TerminalError::new(format!("Invalid period_start: {}", e)))?;
        let period_end = req
            .period_end
            .as_deref()
            .map(|s| s.parse::<NaiveDate>())
            .transpose()
            .map_err(|e| TerminalError::new(format!("Invalid period_end: {}", e)))?;

        let filters = EditionFilters {
            county_id: req.county_id,
            status: req.status,
            period_start,
            period_end,
            limit: req.limit.map(|l| l as i64),
            offset: req.offset.map(|o| o as i64),
        };

        let (editions, total_count) = Edition::list(&filters, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EditionListResult {
            editions: editions.iter().map(Self::edition_to_result).collect(),
            total_count,
        })
    }

    async fn get_edition(
        &self,
        _ctx: Context<'_>,
        req: GetEditionRequest,
    ) -> Result<EditionDetailResult, HandlerError> {
        let edition = Edition::find_by_id(req.id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new(format!("Edition not found: {}", req.id)))?;

        self.load_edition_detail(&edition).await
    }

    async fn current_edition(
        &self,
        _ctx: Context<'_>,
        req: CurrentEditionRequest,
    ) -> Result<EditionDetailResult, HandlerError> {
        let edition = Edition::find_published(req.county_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| {
                TerminalError::new(format!(
                    "No published edition for county: {}",
                    req.county_id
                ))
            })?;

        self.load_edition_detail(&edition).await
    }

    async fn create_edition(
        &self,
        _ctx: Context<'_>,
        req: CreateEditionRequest,
    ) -> Result<EditionResult, HandlerError> {
        let period_start = req
            .period_start
            .parse::<NaiveDate>()
            .map_err(|e| TerminalError::new(format!("Invalid period_start: {}", e)))?;
        let period_end = req
            .period_end
            .parse::<NaiveDate>()
            .map_err(|e| TerminalError::new(format!("Invalid period_end: {}", e)))?;

        let edition = activities::create_edition(
            req.county_id,
            period_start,
            period_end,
            req.title.as_deref(),
            &self.deps,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(Self::edition_to_result(&edition))
    }

    async fn generate_edition(
        &self,
        _ctx: Context<'_>,
        req: GenerateEditionRequest,
    ) -> Result<EditionResult, HandlerError> {
        let edition = activities::generate_edition(req.id, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(Self::edition_to_result(&edition))
    }

    async fn publish_edition(
        &self,
        _ctx: Context<'_>,
        req: PublishEditionRequest,
    ) -> Result<EditionResult, HandlerError> {
        let edition = activities::publish_edition(req.id, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(Self::edition_to_result(&edition))
    }

    async fn archive_edition(
        &self,
        _ctx: Context<'_>,
        req: ArchiveEditionRequest,
    ) -> Result<EditionResult, HandlerError> {
        let edition = activities::archive_edition(req.id, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(Self::edition_to_result(&edition))
    }

    async fn batch_generate(
        &self,
        _ctx: Context<'_>,
        req: BatchGenerateRequest,
    ) -> Result<BatchGenerateEditionsResult, HandlerError> {
        let period_start = req
            .period_start
            .parse::<NaiveDate>()
            .map_err(|e| TerminalError::new(format!("Invalid period_start: {}", e)))?;
        let period_end = req
            .period_end
            .parse::<NaiveDate>()
            .map_err(|e| TerminalError::new(format!("Invalid period_end: {}", e)))?;

        let result = activities::batch_generate_editions(period_start, period_end, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(BatchGenerateEditionsResult {
            created: result.created,
            failed: result.failed,
            total_counties: result.total_counties,
        })
    }

    async fn row_templates(
        &self,
        _ctx: Context<'_>,
        _req: ListTemplatesRequest,
    ) -> Result<RowTemplateListResult, HandlerError> {
        let pool = &self.deps.db_pool;
        let configs = RowTemplateConfig::find_all(pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let all_slots = RowTemplateSlot::find_all(pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let templates = configs
            .into_iter()
            .map(|c| {
                let slots: Vec<RowTemplateSlotResult> = all_slots
                    .iter()
                    .filter(|s| s.row_template_config_id == c.id)
                    .map(|s| RowTemplateSlotResult {
                        slot_index: s.slot_index,
                        weight: s.weight.clone(),
                        count: s.count,
                        accepts: s.accepts.clone(),
                    })
                    .collect();
                RowTemplateResult {
                    id: c.id,
                    slug: c.slug,
                    display_name: c.display_name,
                    description: c.description,
                    slots,
                }
            })
            .collect();

        Ok(RowTemplateListResult { templates })
    }

    async fn post_templates(
        &self,
        _ctx: Context<'_>,
        _req: ListTemplatesRequest,
    ) -> Result<PostTemplateListResult, HandlerError> {
        let configs = PostTemplateConfig::find_all(&self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(PostTemplateListResult {
            templates: configs
                .into_iter()
                .map(|c| PostTemplateResult {
                    id: c.id,
                    slug: c.slug,
                    display_name: c.display_name,
                    compatible_types: c.compatible_types,
                    body_target: c.body_target,
                    body_max: c.body_max,
                    title_max: c.title_max,
                })
                .collect(),
        })
    }

    async fn update_edition_row(
        &self,
        _ctx: Context<'_>,
        req: UpdateEditionRowRequest,
    ) -> Result<EditionRowResult, HandlerError> {
        let pool = &self.deps.db_pool;

        // Resolve template slug to ID if provided
        let template_id = match &req.row_template_slug {
            Some(slug) => {
                let tmpl = RowTemplateConfig::find_by_slug(slug, pool)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?
                    .ok_or_else(|| TerminalError::new(format!("Row template not found: {}", slug)))?;
                Some(tmpl.id)
            }
            None => None,
        };

        let row = EditionRow::update(req.row_id, template_id, req.sort_order, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let template = RowTemplateConfig::find_by_id(row.row_template_config_id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let template_slots = RowTemplateSlot::find_by_template(row.row_template_config_id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let slots = EditionSlot::find_by_row_with_posts(row.id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EditionRowResult {
            id: row.id,
            row_template_slug: template.as_ref().map(|t| t.slug.clone()).unwrap_or_default(),
            row_template_id: row.row_template_config_id,
            row_template_display_name: template.as_ref().map(|t| t.display_name.clone()).unwrap_or_default(),
            row_template_description: template.and_then(|t| t.description),
            row_template_slots: template_slots
                .iter()
                .map(|s| RowTemplateSlotResult {
                    slot_index: s.slot_index,
                    weight: s.weight.clone(),
                    count: s.count,
                    accepts: s.accepts.clone(),
                })
                .collect(),
            sort_order: row.sort_order,
            slots: slots
                .iter()
                .map(|s| EditionSlotResult {
                    id: s.id,
                    post_id: s.post_id,
                    post_template: s.post_template.clone(),
                    slot_index: s.slot_index,
                    post_title: Some(s.post_title.clone()),
                    post_post_type: s.post_post_type.clone(),
                    post_weight: s.post_weight.clone(),
                    post_status: Some(s.post_status.clone()),
                })
                .collect(),
            widgets: vec![],
        })
    }

    async fn reorder_rows(
        &self,
        _ctx: Context<'_>,
        req: ReorderRowsRequest,
    ) -> Result<ReorderRowsResult, HandlerError> {
        let pool = &self.deps.db_pool;

        let rows = EditionRow::reorder(req.edition_id, &req.row_ids, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        // Load all templates + slots upfront (avoids N+1)
        let all_templates = RowTemplateConfig::find_all(pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;
        let all_template_slots = RowTemplateSlot::find_all(pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let mut results = Vec::new();
        for row in &rows {
            let template = all_templates.iter().find(|t| t.id == row.row_template_config_id);
            let template_slot_results: Vec<RowTemplateSlotResult> = all_template_slots
                .iter()
                .filter(|s| s.row_template_config_id == row.row_template_config_id)
                .map(|s| RowTemplateSlotResult {
                    slot_index: s.slot_index,
                    weight: s.weight.clone(),
                    count: s.count,
                    accepts: s.accepts.clone(),
                })
                .collect();

            let slots = EditionSlot::find_by_row_with_posts(row.id, pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

            results.push(EditionRowResult {
                id: row.id,
                row_template_slug: template.map(|t| t.slug.clone()).unwrap_or_default(),
                row_template_id: row.row_template_config_id,
                row_template_display_name: template.map(|t| t.display_name.clone()).unwrap_or_default(),
                row_template_description: template.and_then(|t| t.description.clone()),
                row_template_slots: template_slot_results,
                sort_order: row.sort_order,
                slots: slots
                    .iter()
                    .map(|s| EditionSlotResult {
                        id: s.id,
                        post_id: s.post_id,
                        post_template: s.post_template.clone(),
                        slot_index: s.slot_index,
                        post_title: Some(s.post_title.clone()),
                        post_post_type: s.post_post_type.clone(),
                        post_weight: s.post_weight.clone(),
                        post_status: Some(s.post_status.clone()),
                    })
                    .collect(),
                widgets: vec![],
            });
        }

        Ok(ReorderRowsResult { rows: results })
    }

    async fn remove_post(
        &self,
        _ctx: Context<'_>,
        req: RemovePostFromEditionRequest,
    ) -> Result<bool, HandlerError> {
        EditionSlot::delete(req.slot_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;
        Ok(true)
    }

    async fn change_slot_template(
        &self,
        _ctx: Context<'_>,
        req: ChangeSlotTemplateRequest,
    ) -> Result<EditionSlotResult, HandlerError> {
        let pool = &self.deps.db_pool;
        let slot = EditionSlot::change_template(req.slot_id, &req.post_template, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        // Re-fetch with post data to return consistent response
        let slots_with_posts = EditionSlot::find_by_row_with_posts(slot.edition_row_id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let slot_with_post = slots_with_posts.into_iter().find(|s| s.id == slot.id);

        match slot_with_post {
            Some(s) => Ok(EditionSlotResult {
                id: s.id,
                post_id: s.post_id,
                post_template: s.post_template,
                slot_index: s.slot_index,
                post_title: Some(s.post_title),
                post_post_type: s.post_post_type,
                post_weight: s.post_weight,
                post_status: Some(s.post_status),
            }),
            None => Ok(EditionSlotResult {
                id: slot.id,
                post_id: slot.post_id,
                post_template: slot.post_template,
                slot_index: slot.slot_index,
                post_title: None,
                post_post_type: None,
                post_weight: None,
                post_status: None,
            }),
        }
    }
    async fn move_slot(
        &self,
        _ctx: Context<'_>,
        req: MoveSlotRequest,
    ) -> Result<EditionSlotResult, HandlerError> {
        let pool = &self.deps.db_pool;
        let slot = EditionSlot::move_to(req.slot_id, req.target_row_id, req.slot_index, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        // Re-fetch with post data
        let slots_with_posts = EditionSlot::find_by_row_with_posts(slot.edition_row_id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        match slots_with_posts.into_iter().find(|s| s.id == slot.id) {
            Some(s) => Ok(EditionSlotResult {
                id: s.id,
                post_id: s.post_id,
                post_template: s.post_template,
                slot_index: s.slot_index,
                post_title: Some(s.post_title),
                post_post_type: s.post_post_type,
                post_weight: s.post_weight,
                post_status: Some(s.post_status),
            }),
            None => Ok(EditionSlotResult {
                id: slot.id,
                post_id: slot.post_id,
                post_template: slot.post_template,
                slot_index: slot.slot_index,
                post_title: None,
                post_post_type: None,
                post_weight: None,
                post_status: None,
            }),
        }
    }

    async fn add_post_to_edition(
        &self,
        _ctx: Context<'_>,
        req: AddPostToEditionRequest,
    ) -> Result<EditionSlotResult, HandlerError> {
        let pool = &self.deps.db_pool;
        let slot = EditionSlot::create(
            req.edition_row_id,
            req.post_id,
            &req.post_template,
            req.slot_index,
            pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        // Re-fetch with post data
        let slots_with_posts = EditionSlot::find_by_row_with_posts(slot.edition_row_id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        match slots_with_posts.into_iter().find(|s| s.id == slot.id) {
            Some(s) => Ok(EditionSlotResult {
                id: s.id,
                post_id: s.post_id,
                post_template: s.post_template,
                slot_index: s.slot_index,
                post_title: Some(s.post_title),
                post_post_type: s.post_post_type,
                post_weight: s.post_weight,
                post_status: Some(s.post_status),
            }),
            None => Ok(EditionSlotResult {
                id: slot.id,
                post_id: slot.post_id,
                post_template: slot.post_template,
                slot_index: slot.slot_index,
                post_title: None,
                post_post_type: None,
                post_weight: None,
                post_status: None,
            }),
        }
    }

    async fn add_edition_row(
        &self,
        _ctx: Context<'_>,
        req: AddEditionRowRequest,
    ) -> Result<EditionRowResult, HandlerError> {
        let pool = &self.deps.db_pool;

        // Resolve template slug to ID
        let template = RowTemplateConfig::find_by_slug(&req.row_template_slug, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| {
                TerminalError::new(format!("Row template not found: {}", req.row_template_slug))
            })?;

        let row = EditionRow::create(req.edition_id, template.id, req.sort_order, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let template_slots = RowTemplateSlot::find_by_template(template.id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EditionRowResult {
            id: row.id,
            row_template_slug: template.slug,
            row_template_id: template.id,
            row_template_display_name: template.display_name,
            row_template_description: template.description,
            row_template_slots: template_slots
                .iter()
                .map(|s| RowTemplateSlotResult {
                    slot_index: s.slot_index,
                    weight: s.weight.clone(),
                    count: s.count,
                    accepts: s.accepts.clone(),
                })
                .collect(),
            sort_order: row.sort_order,
            slots: vec![],
            widgets: vec![],
        })
    }

    async fn delete_edition_row(
        &self,
        _ctx: Context<'_>,
        req: DeleteEditionRowRequest,
    ) -> Result<bool, HandlerError> {
        EditionRow::delete(req.row_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;
        Ok(true)
    }

    async fn review_edition(
        &self,
        _ctx: Context<'_>,
        req: ReviewEditionRequest,
    ) -> Result<EditionResult, HandlerError> {
        let edition = activities::review_edition(req.id, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(Self::edition_to_result(&edition))
    }

    async fn approve_edition(
        &self,
        _ctx: Context<'_>,
        req: ApproveEditionRequest,
    ) -> Result<EditionResult, HandlerError> {
        let edition = activities::approve_edition(req.id, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(Self::edition_to_result(&edition))
    }

    async fn batch_approve_editions(
        &self,
        _ctx: Context<'_>,
        req: BatchApproveEditionsRequest,
    ) -> Result<BatchEditionsResult, HandlerError> {
        let (succeeded, failed) = activities::batch_approve_editions(&req.ids, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(BatchEditionsResult { succeeded, failed })
    }

    async fn batch_publish_editions(
        &self,
        _ctx: Context<'_>,
        req: BatchPublishEditionsRequest,
    ) -> Result<BatchEditionsResult, HandlerError> {
        let (succeeded, failed) = activities::batch_publish_editions(&req.ids, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(BatchEditionsResult { succeeded, failed })
    }

    async fn edition_kanban_stats(
        &self,
        _ctx: Context<'_>,
        req: EditionKanbanStatsRequest,
    ) -> Result<EditionKanbanStatsResult, HandlerError> {
        let period_start = req
            .period_start
            .parse::<NaiveDate>()
            .map_err(|e| TerminalError::new(format!("Invalid period_start: {}", e)))?;
        let period_end = req
            .period_end
            .parse::<NaiveDate>()
            .map_err(|e| TerminalError::new(format!("Invalid period_end: {}", e)))?;

        let counts = Edition::count_by_status(period_start, period_end, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let mut result = EditionKanbanStatsResult {
            draft: 0,
            in_review: 0,
            approved: 0,
            published: 0,
        };

        for (status, count) in counts {
            match status.as_str() {
                "draft" => result.draft = count as i32,
                "in_review" => result.in_review = count as i32,
                "approved" => result.approved = count as i32,
                "published" => result.published = count as i32,
                _ => {} // archived or other statuses ignored for kanban
            }
        }

        Ok(result)
    }

    async fn add_widget(
        &self,
        _ctx: Context<'_>,
        req: AddWidgetRequest,
    ) -> Result<EditionWidgetResult, HandlerError> {
        let widget = EditionWidget::create(
            req.edition_row_id,
            &req.widget_type,
            req.slot_index,
            req.config,
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EditionWidgetResult {
            id: widget.id,
            widget_type: widget.widget_type,
            slot_index: widget.slot_index,
            config: widget.config,
        })
    }

    async fn update_widget(
        &self,
        _ctx: Context<'_>,
        req: UpdateWidgetRequest,
    ) -> Result<EditionWidgetResult, HandlerError> {
        let widget = EditionWidget::update(req.id, req.config, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EditionWidgetResult {
            id: widget.id,
            widget_type: widget.widget_type,
            slot_index: widget.slot_index,
            config: widget.config,
        })
    }

    async fn remove_widget(
        &self,
        _ctx: Context<'_>,
        req: RemoveWidgetRequest,
    ) -> Result<bool, HandlerError> {
        EditionWidget::delete(req.id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;
        Ok(true)
    }
}

// Needed for NaiveDate parsing
use chrono::NaiveDate;
