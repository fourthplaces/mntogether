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
  rowCount?: number;
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
}

interface WidgetData {
  id: string;
  widgetType: string;
  authoringMode: string;
  data: unknown; // JSON object from Rust
  zip_code?: string;
  city?: string;
  county_id?: string;
  start_date?: string;
  end_date?: string;
  createdAt: string;
  updatedAt: string;
}

interface EditionSlotData {
  id: string;
  kind: string;
  slotIndex: number;
  sortOrder: number;
  // Post fields (present when kind='post')
  postId?: string;
  postTemplate?: string;
  postTitle?: string;
  postPostType?: string;
  postWeight?: string;
  postStatus?: string;
  postIsSeed?: boolean;
  // Widget fields (present when kind='widget')
  widgetId?: string;
  widgetType?: string;
  widgetAuthoringMode?: string;
  widgetData?: unknown;
  widgetIsSeed?: boolean;
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
}

interface PublicBroadsheetSlotData {
  kind: string;
  postTemplate?: string;
  widget_template?: string;
  slotIndex: number;
  post?: PublicBroadsheetPostData;
  widget?: { id: string; widgetType: string; authoringMode: string; data: unknown };
}

interface PublicBroadsheetPostData {
  id: string;
  title: string;
  description: string;
  postType: string;
  weight: string;
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
  // Field groups (snake_case from Rust serde)
  media?: Array<{ image_url?: string; caption?: string; credit?: string }>;
  items?: Array<{ name: string; detail?: string }>;
  person?: { name?: string; role?: string; bio?: string; photo_url?: string; quote?: string };
  link?: { label?: string; url?: string; deadline?: string };
  source_attribution?: { source_name?: string; attribution?: string };
  meta?: { kicker?: string; byline?: string; timestamp?: string; updated?: string; deck?: string };
  datetime?: { start?: string; end?: string; cost?: string; recurring?: boolean };
  post_status?: { state?: string; verified?: string };
  schedule?: Array<{ day: string; opens: string; closes: string }>;
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
    rowCount: (parent: EditionData) => {
      // Use explicit row_count from list endpoints, or fall back to rows array length
      return parent.rowCount ?? parent.rows?.length ?? 0;
    },
    rows: (parent: EditionData) => {
      // Rows are already present from get_edition / current_edition,
      // but may be absent from list_editions results
      return parent.rows ?? [];
    },
    sections: (parent: EditionData) => {
      return parent.sections ?? [];
    },
    // True if any slotted post or widget is seed data. Derived by walking
    // the already-loaded row tree — no extra RPC. If `rows` aren't loaded
    // (e.g. the list-edition endpoints skip them), this returns false,
    // which is safe: the admin UI only runs the publish mutation from
    // surfaces that do load rows (edition detail, workflow batch-publish
    // uses server-side contamination check before calling the mutation).
    containsSeedContent: (parent: EditionData) => {
      const rows = parent.rows ?? [];
      for (const row of rows) {
        for (const slot of row.slots ?? []) {
          if (slot.postIsSeed === true) return true;
          if (slot.widgetIsSeed === true) return true;
        }
      }
      return false;
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
  },

  // Resolve standalone Widget — data is JSON, serialized to string for GraphQL
  Widget: {
    data: (parent: { data?: unknown }) => {
      return typeof parent.data === "string"
        ? parent.data
        : JSON.stringify(parent.data ?? {});
    },
    zipCode: (parent: { zipCode?: string; zip_code?: string }) =>
      parent.zipCode ?? parent.zip_code ?? null,
    countyId: (parent: { countyId?: string; county_id?: string }) =>
      parent.countyId ?? parent.county_id ?? null,
    startDate: (parent: { startDate?: string; start_date?: string }) =>
      parent.startDate ?? parent.start_date ?? null,
    endDate: (parent: { endDate?: string; end_date?: string }) =>
      parent.endDate ?? parent.end_date ?? null,
    county: async (
      parent: { countyId?: string; county_id?: string },
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      const id = parent.countyId ?? parent.county_id;
      if (!id) return null;
      return ctx.server.callService("Editions", "get_county", { id });
    },
  },

  // Resolve nested objects on EditionSlot — polymorphic (post or widget)
  EditionSlot: {
    post: (parent: EditionSlotData) => {
      if (parent.kind !== "post" || !parent.postId) return null;
      return {
        id: parent.postId,
        title: parent.postTitle ?? "Untitled",
        postType: parent.postPostType ?? null,
        weight: parent.postWeight ?? null,
        status: parent.postStatus ?? "active",
        // Forward is_seed from the slot-embedded post row so the admin
        // UI can render a SEED badge on slot cards and Edition can
        // compute containsSeedContent without refetching each post.
        isSeed: parent.postIsSeed ?? false,
      };
    },
    widget: (parent: EditionSlotData) => {
      if (parent.kind !== "widget" || !parent.widgetId) return null;
      const rawData = parent.widgetData ?? {};
      return {
        id: parent.widgetId,
        widgetType: parent.widgetType ?? "",
        authoringMode: parent.widgetAuthoringMode ?? "human",
        data: typeof rawData === "string" ? rawData : JSON.stringify(rawData),
        createdAt: "",
        updatedAt: "",
        isSeed: parent.widgetIsSeed ?? false,
      };
    },
  },

  // Resolve nested objects on County (fipsCode from fips_code is already handled by snakeToCamel)
  County: {
    fipsCode: (parent: { fipsCode?: string; fips_code?: string }) => {
      return parent.fipsCode ?? parent.fips_code ?? null;
    },
    isPseudo: (parent: { isPseudo?: boolean; is_pseudo?: boolean }) => {
      return parent.isPseudo ?? parent.is_pseudo ?? false;
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
  // and map snake_case field group data to camelCase GraphQL fields
  BroadsheetPost: {
    urgentNotes: (parent: PublicBroadsheetPostData) => parent.urgentNotes ?? [],
    tags: (parent: PublicBroadsheetPostData) => parent.tags ?? [],
    contacts: (parent: PublicBroadsheetPostData) => parent.contacts ?? [],
    media: (parent: PublicBroadsheetPostData) => parent.media ?? [],
    items: (parent: PublicBroadsheetPostData) => parent.items ?? [],
    schedule: (parent: PublicBroadsheetPostData) => parent.schedule ?? [],
    person: (parent: PublicBroadsheetPostData) => parent.person ?? null,
    link: (parent: PublicBroadsheetPostData) => parent.link ?? null,
    sourceAttribution: (parent: PublicBroadsheetPostData) => parent.source_attribution ?? null,
    meta: (parent: PublicBroadsheetPostData) => parent.meta ?? null,
    datetime: (parent: PublicBroadsheetPostData) => parent.datetime ?? null,
    postStatus: (parent: PublicBroadsheetPostData) => parent.post_status ?? null,
  },

  // snake_case → camelCase for nested field group types
  BroadsheetMedia: {
    imageUrl: (parent: { image_url?: string; imageUrl?: string }) =>
      parent.imageUrl ?? parent.image_url ?? null,
  },
  BroadsheetPerson: {
    photoUrl: (parent: { photo_url?: string; photoUrl?: string }) =>
      parent.photoUrl ?? parent.photo_url ?? null,
  },
  BroadsheetSourceAttribution: {
    sourceName: (parent: { source_name?: string; sourceName?: string }) =>
      parent.sourceName ?? parent.source_name ?? null,
  },

  BroadsheetSlot: {
    widgetTemplate: (parent: PublicBroadsheetSlotData) =>
      parent.widget_template ?? (parent as unknown as Record<string, unknown>).widgetTemplate ?? null,
  },

  BroadsheetWidget: {
    data: (parent: { data?: unknown }) => {
      return typeof parent.data === "string"
        ? parent.data
        : JSON.stringify(parent.data ?? {});
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

    countyDashboard: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      // Fetch counties and latest editions in parallel
      const [countiesResult, editionsResult] = await Promise.all([
        ctx.server.callService<{ counties: Array<{ id: string; fipsCode: string; name: string; state: string }> }>(
          "Editions", "list_counties", {}
        ),
        ctx.server.callService<{ editions: EditionData[]; total_count: number }>(
          "Editions", "latest_editions", {}
        ),
      ]);

      const counties = countiesResult.counties;
      const editions = editionsResult.editions;

      // Build a map of county_id → latest edition
      const editionByCounty = new Map<string, EditionData>();
      for (const edition of editions) {
        editionByCounty.set(edition.countyId, edition);
      }

      // Current period: determine if an edition is stale
      const now = new Date();
      const dayOfWeek = now.getDay(); // 0=Sun
      const mondayOffset = dayOfWeek === 0 ? -6 : 1 - dayOfWeek;
      const currentMonday = new Date(now);
      currentMonday.setDate(now.getDate() + mondayOffset);
      const currentMondayStr = currentMonday.toISOString().split("T")[0];

      return counties.map((county: { id: string; fipsCode: string; name: string; state: string }) => {
        const edition = editionByCounty.get(county.id) || null;
        // Stale = the county has no published broadsheet for the current week.
        // Separate from editorial state (draft/reviewing/approved are all still "stale" publicly).
        const isStale = !edition
          || edition.periodStart < currentMondayStr
          || edition.status !== "published";
        const lastPublishedAt = edition?.publishedAt || null;

        return {
          county,
          currentEdition: edition ? {
            ...edition,
            county,
          } : null,
          lastPublishedAt,
          isStale,
        };
      });
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

    widget: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Widgets", "get_widget", { id: args.id });
    },

    widgets: async (
      _parent: unknown,
      args: {
        widgetType?: string;
        countyId?: string;
        search?: string;
        limit?: number;
        offset?: number;
      },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.server.callService<{
        widgets: WidgetData[];
      }>("Widgets", "list_widgets", {
        widget_type: args.widgetType ?? null,
        county_id: args.countyId ?? null,
        search: args.search ?? null,
        limit: args.limit ?? null,
        offset: args.offset ?? null,
      });
      return result.widgets;
    },

    editionWidgets: async (
      _parent: unknown,
      args: {
        editionId: string;
        slottedFilter?: string;
        limit?: number;
        offset?: number;
      },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.server.callService<{
        widgets: WidgetData[];
      }>("Widgets", "list_widgets_for_edition", {
        edition_id: args.editionId,
        slotted_filter: args.slottedFilter ?? null,
        limit: args.limit ?? null,
        offset: args.offset ?? null,
      });
      return result.widgets;
    },
  },

  Mutation: {
    updateCountyTargetContentWeight: async (
      _parent: unknown,
      args: { id: string; targetContentWeight: number },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "update_county_target_weight", {
        id: args.id,
        target_content_weight: args.targetContentWeight,
      });
    },

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

    unpublishEdition: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "unpublish_edition", {
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
      args: { slotId: string; targetRowId: string; slotIndex: number; sortOrder?: number | null },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "move_slot", {
        slot_id: args.slotId,
        target_row_id: args.targetRowId,
        slot_index: args.slotIndex,
        sort_order: args.sortOrder ?? null,
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

    createWidget: async (
      _parent: unknown,
      args: {
        widgetType: string;
        data: string;
        authoringMode?: string;
        zipCode?: string;
        city?: string;
        countyId?: string;
        startDate?: string;
        endDate?: string;
      },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Widgets", "create_widget", {
        widget_type: args.widgetType,
        authoring_mode: args.authoringMode ?? null,
        data: JSON.parse(args.data),
        zip_code: args.zipCode ?? null,
        city: args.city ?? null,
        county_id: args.countyId ?? null,
        start_date: args.startDate ?? null,
        end_date: args.endDate ?? null,
      });
    },

    updateWidget: async (
      _parent: unknown,
      args: {
        id: string;
        data?: string;
        zipCode?: string;
        city?: string;
        countyId?: string;
        startDate?: string;
        endDate?: string;
      },
      ctx: GraphQLContext
    ) => {
      // Build payload — only include fields that were explicitly passed.
      // Empty strings are kept as-is so the Rust handler can distinguish
      // "clear this field" (Some("")) from "don't touch" (absent/None).
      const payload: Record<string, unknown> = { id: args.id };
      if (args.data != null) payload.data = JSON.parse(args.data);
      if (args.zipCode !== undefined) payload.zip_code = args.zipCode ?? "";
      if (args.city !== undefined) payload.city = args.city ?? "";
      if (args.countyId !== undefined) payload.county_id = args.countyId || null;
      if (args.startDate !== undefined) payload.start_date = args.startDate || null;
      if (args.endDate !== undefined) payload.end_date = args.endDate || null;
      return ctx.server.callService("Widgets", "update_widget", payload);
    },

    // Backward compat alias — updates data only
    updateWidgetData: async (
      _parent: unknown,
      args: { id: string; data: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Widgets", "update_widget", {
        id: args.id,
        data: JSON.parse(args.data),
      });
    },

    deleteWidget: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Widgets", "delete_widget", {
        id: args.id,
      });
    },

    addWidgetToEdition: async (
      _parent: unknown,
      args: { editionRowId: string; widgetId: string; slotIndex: number },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Editions", "add_widget_to_edition", {
        edition_row_id: args.editionRowId,
        widget_id: args.widgetId,
        slot_index: args.slotIndex,
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
