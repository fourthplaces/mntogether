import type { GraphQLContext } from "../context";

// =============================================================================
// Helper types for server response shapes (after snakeToCamel transform)
// =============================================================================

interface EditionSectionData {
  id: string;
  editionId: string;
  title: string;
  subtitle?: string;
  topicSlug?: string;
  sortOrder: number;
  createdAt: string;
}

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
  sections?: EditionSectionData[];
}

interface EditionRowData {
  id: string;
  rowTemplateSlug: string;
  rowTemplateId: string;
  rowTemplateDisplayName: string;
  rowTemplateDescription?: string;
  layoutVariant: string;
  rowTemplateSlots: Array<{
    slotIndex: number;
    weight: string;
    count: number;
    accepts?: string[] | null;
    postTemplateSlug?: string | null;
  }>;
  sortOrder: number;
  sectionId?: string;
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
// Type resolvers — bridge server flat data → GraphQL nested types
// =============================================================================

// =============================================================================
// Helper types for public broadsheet response
// =============================================================================

interface PublicBroadsheetSectionData {
  id: string;
  title: string;
  subtitle?: string;
  topicSlug?: string;
  sortOrder: number;
}

interface PublicBroadsheetData {
  edition: EditionData;
  county: { id: string; fipsCode?: string; fips_code?: string; name: string; state: string };
  rows: PublicBroadsheetRowData[];
  sections: PublicBroadsheetSectionData[];
}

interface PublicBroadsheetRowData {
  rowTemplateSlug: string;
  layoutVariant: string;
  sortOrder: number;
  sectionId?: string;
  slots: PublicBroadsheetSlotData[];
  widgets: EditionWidgetData[];
}

interface PublicBroadsheetSlotData {
  postTemplate: string;
  slotIndex: number;
  post: PublicBroadsheetPostData;
}

interface PublicBroadsheetPostData {
  id: string;
  title: string;
  description: string;
  postType: string;
  weight: string;
  urgency?: string;
  location?: string;
  sourceUrl?: string;
  organizationName?: string;
  publishedAt?: string;
  tags: Array<{ kind: string; value: string; displayName?: string; color?: string }>;
  contacts: Array<{ contactType: string; contactValue: string; contactLabel?: string }>;
  urgentNotes: Array<{ content: string; ctaText?: string }>;
  bodyHeavy?: string;
  bodyMedium?: string;
  bodyLight?: string;
}

export const editionResolvers = {
  // Resolve nested objects on the Edition type
  Edition: {
    county: async (parent: EditionData, _args: unknown, ctx: GraphQLContext) => {
      // countyId is always present (FK constraint in DB)
      return ctx.server.callService("Editions", "get_county", {
        id: parent.countyId,
      });
    },
    rows: (parent: EditionData) => {
      // Rows are already present from get_edition / current_edition,
      // but may be absent from list_editions results
      return parent.rows ?? [];
    },
    sections: (parent: EditionData) => {
      return parent.sections ?? [];
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
        layoutVariant: parent.layoutVariant ?? 'full',
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

  // Public broadsheet type resolvers
  BroadsheetCounty: {
    fipsCode: (parent: { fipsCode?: string; fips_code?: string }) => {
      return parent.fipsCode ?? parent.fips_code ?? "";
    },
  },

  // Coerce nullable arrays to empty arrays for non-nullable schema fields
  BroadsheetPost: {
    urgentNotes: (parent: PublicBroadsheetPostData) => parent.urgentNotes ?? [],
    tags: (parent: PublicBroadsheetPostData) => parent.tags ?? [],
    contacts: (parent: PublicBroadsheetPostData) => parent.contacts ?? [],
  },

  BroadsheetWidget: {
    config: (parent: { config?: unknown }) => {
      return typeof parent.config === "string"
        ? parent.config
        : JSON.stringify(parent.config ?? {});
    },
  },

  Query: {
    publicBroadsheet: async (
      _parent: unknown,
      args: { countyId: string },
      ctx: GraphQLContext
    ) => {
      const result =
        await ctx.server.callService<PublicBroadsheetData>(
          "Public",
          "current_broadsheet",
          { county_id: args.countyId }
        );
      // Flatten edition fields + county + rows + sections into a single PublicBroadsheet object
      return {
        ...result.edition,
        county: result.county,
        rows: result.rows,
        sections: result.sections ?? [],
      };
    },

    editionPreview: async (
      _parent: unknown,
      args: { editionId: string },
      ctx: GraphQLContext
    ) => {
      const result =
        await ctx.server.callService<PublicBroadsheetData>(
          "Editions",
          "preview_broadsheet",
          { edition_id: args.editionId }
        );
      return {
        ...result.edition,
        county: result.county,
        rows: result.rows,
        sections: result.sections ?? [],
      };
    },

    counties: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      const result = await ctx.server.callService<{
        counties: unknown[];
      }>("Editions", "list_counties", {});
      return result.counties;
    },

    county: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "get_county", {
        id: args.id,
      });
    },

    latestEditions: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      const result = await ctx.server.callService<{
        editions: EditionData[];
        total_count: number;
      }>("Editions", "latest_editions", {});
      return result.editions;
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
      return ctx.server.callService("Editions", "list_editions", {
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
      const result = await ctx.server.callService<{
        edition: EditionData;
        rows: EditionRowData[];
        sections: EditionSectionData[];
      }>("Editions", "get_edition", { id: args.id });
      // Merge edition fields with rows + sections for the GraphQL Edition type
      return { ...result.edition, rows: result.rows, sections: result.sections ?? [] };
    },

    currentEdition: async (
      _parent: unknown,
      args: { countyId: string },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.server.callService<{
        edition: EditionData;
        rows: EditionRowData[];
        sections: EditionSectionData[];
      }>("Editions", "current_edition", {
        county_id: args.countyId,
      });
      return { ...result.edition, rows: result.rows, sections: result.sections ?? [] };
    },

    editionKanbanStats: async (
      _parent: unknown,
      args: { periodStart: string; periodEnd: string },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.server.callService<{
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
      const result = await ctx.server.callService<{
        templates: unknown[];
      }>("Editions", "row_templates", {});
      return result.templates;
    },

    postTemplates: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      const result = await ctx.server.callService<{
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
      return ctx.server.callService("Editions", "create_edition", {
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
      return ctx.server.callService("Editions", "generate_edition", {
        id: args.id,
      });
    },

    reviewEdition: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "review_edition", {
        id: args.id,
      });
    },

    approveEdition: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "approve_edition", {
        id: args.id,
      });
    },

    publishEdition: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "publish_edition", {
        id: args.id,
      });
    },

    archiveEdition: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "archive_edition", {
        id: args.id,
      });
    },

    batchGenerateEditions: async (
      _parent: unknown,
      args: { periodStart: string; periodEnd: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService(
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
      return ctx.server.callService("Editions", "batch_approve_editions", {
        ids: args.ids,
      });
    },

    batchPublishEditions: async (
      _parent: unknown,
      args: { ids: string[] },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "batch_publish_editions", {
        ids: args.ids,
      });
    },

    moveSlot: async (
      _parent: unknown,
      args: { slotId: string; targetRowId: string; slotIndex: number },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "move_slot", {
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
      return ctx.server.callService("Editions", "add_post_to_edition", {
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
      return ctx.server.callService("Editions", "add_edition_row", {
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
      return ctx.server.callService("Editions", "delete_edition_row", {
        row_id: args.rowId,
      });
    },

    updateEditionRow: async (
      _parent: unknown,
      args: { rowId: string; rowTemplateSlug?: string; sortOrder?: number },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "update_edition_row", {
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
      const result = await ctx.server.callService<{
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
      return ctx.server.callService("Editions", "remove_post", {
        slot_id: args.slotId,
      });
    },

    changeSlotTemplate: async (
      _parent: unknown,
      args: { slotId: string; postTemplate: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "change_slot_template", {
        slot_id: args.slotId,
        post_template: args.postTemplate,
      });
    },

    addWidget: async (
      _parent: unknown,
      args: { editionRowId: string; widgetType: string; slotIndex: number; config: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "add_widget", {
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
      return ctx.server.callService("Editions", "update_widget", {
        id: args.id,
        config: JSON.parse(args.config),
      });
    },

    removeWidget: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "remove_widget", {
        id: args.id,
      });
    },

    addSection: async (
      _parent: unknown,
      args: { editionId: string; title: string; subtitle?: string; topicSlug?: string; sortOrder: number },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "add_section", {
        edition_id: args.editionId,
        title: args.title,
        subtitle: args.subtitle ?? null,
        topic_slug: args.topicSlug ?? null,
        sort_order: args.sortOrder,
      });
    },

    updateSection: async (
      _parent: unknown,
      args: { id: string; title?: string; subtitle?: string; topicSlug?: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "update_section", {
        id: args.id,
        title: args.title ?? null,
        subtitle: args.subtitle !== undefined ? args.subtitle : null,
        topic_slug: args.topicSlug !== undefined ? args.topicSlug : null,
      });
    },

    reorderSections: async (
      _parent: unknown,
      args: { editionId: string; sectionIds: string[] },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "reorder_sections", {
        edition_id: args.editionId,
        section_ids: args.sectionIds,
      });
    },

    deleteSection: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callService("Editions", "delete_section", {
        id: args.id,
      });
      return true;
    },

    assignRowToSection: async (
      _parent: unknown,
      args: { rowId: string; sectionId?: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callService("Editions", "assign_row_to_section", {
        row_id: args.rowId,
        section_id: args.sectionId ?? null,
      });
      return true;
    },
  },
};
