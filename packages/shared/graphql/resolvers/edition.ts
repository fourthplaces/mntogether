import type { GraphQLContext } from "../context";

// =============================================================================
// Helper types for Restate response shapes (after snakeToCamel transform)
// =============================================================================

interface EditionData {
  id: string;
  countyId: string;
  title?: string;
  periodStart: string;
  periodEnd: string;
  status: string;
  publishedAt?: string;
  createdAt: string;
  rows?: EditionRowData[];
}

interface EditionRowData {
  id: string;
  rowTemplateSlug: string;
  rowTemplateId: string;
  rowTemplateDisplayName: string;
  rowTemplateDescription?: string;
  rowTemplateSlots: Array<{
    slotIndex: number;
    weight: string;
    count: number;
    accepts?: string[] | null;
  }>;
  sortOrder: number;
  slots: EditionSlotData[];
  widgets?: EditionWidgetData[];
}

interface EditionWidgetData {
  id: string;
  widgetType: string;
  slotIndex: number;
  config: unknown; // JSON object from Rust
}

interface EditionSlotData {
  id: string;
  postId: string;
  postTemplate: string;
  slotIndex: number;
  // Embedded post data from Rust service (avoids N+1)
  postTitle?: string;
  postPostType?: string;
  postWeight?: string;
  postStatus?: string;
}

// =============================================================================
// Type resolvers — bridge Restate flat data → GraphQL nested types
// =============================================================================

export const editionResolvers = {
  // Resolve nested objects on the Edition type
  Edition: {
    county: async (parent: EditionData, _args: unknown, ctx: GraphQLContext) => {
      // countyId is always present (FK constraint in DB)
      return ctx.restate.callService("Editions", "get_county", {
        id: parent.countyId,
      });
    },
    rows: (parent: EditionData) => {
      // Rows are already present from get_edition / current_edition,
      // but may be absent from list_editions results
      return parent.rows ?? [];
    },
  },

  // Resolve nested objects on EditionRow — template data is embedded from Rust service
  EditionRow: {
    rowTemplate: (parent: EditionRowData) => {
      // Template data is embedded in the row result — no separate RPC call needed
      return {
        id: parent.rowTemplateId,
        slug: parent.rowTemplateSlug,
        displayName: parent.rowTemplateDisplayName ?? parent.rowTemplateSlug,
        description: parent.rowTemplateDescription ?? null,
        slots: parent.rowTemplateSlots ?? [],
      };
    },
    widgets: (parent: EditionRowData) => {
      return parent.widgets ?? [];
    },
  },

  // Resolve fields on EditionWidget — config is JSON, serialized to string for GraphQL
  EditionWidget: {
    widgetType: (parent: { widgetType?: string; widget_type?: string }) => {
      return parent.widgetType ?? parent.widget_type ?? "";
    },
    slotIndex: (parent: { slotIndex?: number; slot_index?: number }) => {
      return parent.slotIndex ?? parent.slot_index ?? 0;
    },
    config: (parent: { config?: unknown }) => {
      return typeof parent.config === "string"
        ? parent.config
        : JSON.stringify(parent.config ?? {});
    },
  },

  // Resolve nested objects on EditionSlot — post data is embedded from Rust service
  EditionSlot: {
    post: (parent: EditionSlotData) => {
      return {
        id: parent.postId,
        title: parent.postTitle ?? "Untitled",
        postType: parent.postPostType ?? null,
        weight: parent.postWeight ?? null,
        status: parent.postStatus ?? "active",
      };
    },
  },

  // Resolve nested objects on County (fipsCode from fips_code is already handled by snakeToCamel)
  County: {
    fipsCode: (parent: { fipsCode?: string; fips_code?: string }) => {
      return parent.fipsCode ?? parent.fips_code ?? null;
    },
  },

  // Resolve fields on RowTemplate
  RowTemplate: {
    displayName: (parent: { displayName?: string; display_name?: string }) => {
      return parent.displayName ?? parent.display_name ?? "";
    },
  },

  // Resolve fields on PostTemplateConfig
  PostTemplateConfig: {
    displayName: (parent: { displayName?: string; display_name?: string }) => {
      return parent.displayName ?? parent.display_name ?? "";
    },
    compatibleTypes: (parent: { compatibleTypes?: string[]; compatible_types?: string[] }) => {
      return parent.compatibleTypes ?? parent.compatible_types ?? [];
    },
    bodyTarget: (parent: { bodyTarget?: number; body_target?: number }) => {
      return parent.bodyTarget ?? parent.body_target ?? 0;
    },
    bodyMax: (parent: { bodyMax?: number; body_max?: number }) => {
      return parent.bodyMax ?? parent.body_max ?? 0;
    },
    titleMax: (parent: { titleMax?: number; title_max?: number }) => {
      return parent.titleMax ?? parent.title_max ?? 0;
    },
  },

  // Resolve EditionConnection (map total_count -> totalCount)
  EditionConnection: {
    totalCount: (parent: { totalCount?: number; total_count?: number }) => {
      return parent.totalCount ?? parent.total_count ?? 0;
    },
  },

  Query: {
    counties: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{
        counties: unknown[];
      }>("Editions", "list_counties", {});
      return result.counties;
    },

    county: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "get_county", {
        id: args.id,
      });
    },

    editions: async (
      _parent: unknown,
      args: {
        countyId?: string;
        status?: string;
        periodStart?: string;
        periodEnd?: string;
        limit?: number;
        offset?: number;
      },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "list_editions", {
        county_id: args.countyId ?? null,
        status: args.status ?? null,
        period_start: args.periodStart ?? null,
        period_end: args.periodEnd ?? null,
        limit: args.limit ?? null,
        offset: args.offset ?? null,
      });
    },

    edition: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{
        edition: EditionData;
        rows: EditionRowData[];
      }>("Editions", "get_edition", { id: args.id });
      // Merge edition fields with rows for the GraphQL Edition type
      return { ...result.edition, rows: result.rows };
    },

    currentEdition: async (
      _parent: unknown,
      args: { countyId: string },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{
        edition: EditionData;
        rows: EditionRowData[];
      }>("Editions", "current_edition", {
        county_id: args.countyId,
      });
      return { ...result.edition, rows: result.rows };
    },

    editionKanbanStats: async (
      _parent: unknown,
      args: { periodStart: string; periodEnd: string },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{
        draft: number;
        in_review: number;
        approved: number;
        published: number;
      }>("Editions", "edition_kanban_stats", {
        period_start: args.periodStart,
        period_end: args.periodEnd,
      });
      return {
        draft: result.draft,
        inReview: result.in_review,
        approved: result.approved,
        published: result.published,
      };
    },

    rowTemplates: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{
        templates: unknown[];
      }>("Editions", "row_templates", {});
      return result.templates;
    },

    postTemplates: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{
        templates: unknown[];
      }>("Editions", "post_templates", {});
      return result.templates;
    },
  },

  Mutation: {
    createEdition: async (
      _parent: unknown,
      args: {
        countyId: string;
        periodStart: string;
        periodEnd: string;
        title?: string;
      },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "create_edition", {
        county_id: args.countyId,
        period_start: args.periodStart,
        period_end: args.periodEnd,
        title: args.title ?? null,
      });
    },

    generateEdition: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "generate_edition", {
        id: args.id,
      });
    },

    reviewEdition: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "review_edition", {
        id: args.id,
      });
    },

    approveEdition: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "approve_edition", {
        id: args.id,
      });
    },

    publishEdition: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "publish_edition", {
        id: args.id,
      });
    },

    archiveEdition: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "archive_edition", {
        id: args.id,
      });
    },

    batchGenerateEditions: async (
      _parent: unknown,
      args: { periodStart: string; periodEnd: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService(
        "Editions",
        "batch_generate",
        {
          period_start: args.periodStart,
          period_end: args.periodEnd,
        }
      );
    },

    batchApproveEditions: async (
      _parent: unknown,
      args: { ids: string[] },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "batch_approve_editions", {
        ids: args.ids,
      });
    },

    batchPublishEditions: async (
      _parent: unknown,
      args: { ids: string[] },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "batch_publish_editions", {
        ids: args.ids,
      });
    },

    moveSlot: async (
      _parent: unknown,
      args: { slotId: string; targetRowId: string; slotIndex: number },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "move_slot", {
        slot_id: args.slotId,
        target_row_id: args.targetRowId,
        slot_index: args.slotIndex,
      });
    },

    addPostToEdition: async (
      _parent: unknown,
      args: {
        editionRowId: string;
        postId: string;
        postTemplate: string;
        slotIndex: number;
      },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "add_post_to_edition", {
        edition_row_id: args.editionRowId,
        post_id: args.postId,
        post_template: args.postTemplate,
        slot_index: args.slotIndex,
      });
    },

    addEditionRow: async (
      _parent: unknown,
      args: { editionId: string; rowTemplateSlug: string; sortOrder: number },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "add_edition_row", {
        edition_id: args.editionId,
        row_template_slug: args.rowTemplateSlug,
        sort_order: args.sortOrder,
      });
    },

    deleteEditionRow: async (
      _parent: unknown,
      args: { rowId: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "delete_edition_row", {
        row_id: args.rowId,
      });
    },

    updateEditionRow: async (
      _parent: unknown,
      args: { rowId: string; rowTemplateSlug?: string; sortOrder?: number },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "update_edition_row", {
        row_id: args.rowId,
        row_template_slug: args.rowTemplateSlug ?? null,
        sort_order: args.sortOrder ?? null,
      });
    },

    reorderEditionRows: async (
      _parent: unknown,
      args: { editionId: string; rowIds: string[] },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{
        rows: unknown[];
      }>("Editions", "reorder_rows", {
        edition_id: args.editionId,
        row_ids: args.rowIds,
      });
      return result.rows;
    },

    removePostFromEdition: async (
      _parent: unknown,
      args: { slotId: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "remove_post", {
        slot_id: args.slotId,
      });
    },

    changeSlotTemplate: async (
      _parent: unknown,
      args: { slotId: string; postTemplate: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "change_slot_template", {
        slot_id: args.slotId,
        post_template: args.postTemplate,
      });
    },

    addWidget: async (
      _parent: unknown,
      args: { editionRowId: string; widgetType: string; slotIndex: number; config: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "add_widget", {
        edition_row_id: args.editionRowId,
        widget_type: args.widgetType,
        slot_index: args.slotIndex,
        config: JSON.parse(args.config),
      });
    },

    updateWidget: async (
      _parent: unknown,
      args: { id: string; config: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "update_widget", {
        id: args.id,
        config: JSON.parse(args.config),
      });
    },

    removeWidget: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Editions", "remove_widget", {
        id: args.id,
      });
    },
  },
};
