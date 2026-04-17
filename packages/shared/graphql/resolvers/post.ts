import type { GraphQLContext } from "../context";

export const postResolvers = {
  Query: {
    publicPosts: async (
      _parent: unknown,
      args: {
        postType?: string;
        category?: string;
        limit?: number;
        offset?: number;
        zipCode?: string;
        radiusMiles?: number;
      },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Posts", "public_list", {
        post_type: args.postType,
        category: args.category,
        limit: args.limit,
        offset: args.offset,
        zip_code: args.zipCode,
        radius_miles: args.radiusMiles,
      });
    },

    publicFilters: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Posts", "public_filters", {});
    },

    post: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.loaders.postById.load(args.id);
    },

    posts: async (
      _parent: unknown,
      args: {
        status?: string;
        search?: string;
        postType?: string;
        submissionType?: string;
        excludeSubmissionType?: string;
        countyId?: string;
        statewideOnly?: boolean;
        zipCode?: string;
        radiusMiles?: number;
        limit?: number;
        offset?: number;
      },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Posts", "list", {
        status: args.status,
        search: args.search,
        post_type: args.postType,
        submission_type: args.submissionType,
        exclude_submission_type: args.excludeSubmissionType,
        county_id: args.countyId,
        statewide_only: args.statewideOnly,
        zip_code: args.zipCode,
        radius_miles: args.radiusMiles,
        first: args.limit,
        offset: args.offset,
      });
    },

    postStats: async (
      _parent: unknown,
      args: { status?: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Posts", "stats", {
        status: args.status,
      });
    },

    editionPosts: async (
      _parent: unknown,
      args: {
        editionId: string;
        slottedFilter?: string;
        limit?: number;
        offset?: number;
      },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Posts", "list_for_edition", {
        edition_id: args.editionId,
        slotted_filter: args.slottedFilter ?? null,
        limit: args.limit ?? null,
        offset: args.offset ?? null,
      });
    },
  },

  Mutation: {
    trackPostView: async (
      _parent: unknown,
      args: { postId: string },
      ctx: GraphQLContext
    ) => {
      try {
        await ctx.server.callObject(
          "Post",
          args.postId,
          "track_view",
          {}
        );
        return true;
      } catch {
        return false;
      }
    },

    trackPostClick: async (
      _parent: unknown,
      args: { postId: string },
      ctx: GraphQLContext
    ) => {
      try {
        await ctx.server.callObject(
          "Post",
          args.postId,
          "track_click",
          {}
        );
        return true;
      } catch {
        return false;
      }
    },

    approvePost: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callObject("Post", args.id, "approve", {});
      ctx.loaders.postById.clear(args.id);
      return ctx.loaders.postById.load(args.id);
    },

    rejectPost: async (
      _parent: unknown,
      args: { id: string; reason?: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callObject("Post", args.id, "reject", {
        reason: args.reason ?? "Rejected by admin",
      });
      ctx.loaders.postById.clear(args.id);
      return ctx.loaders.postById.load(args.id);
    },

    archivePost: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callObject("Post", args.id, "archive", {});
      ctx.loaders.postById.clear(args.id);
      return ctx.loaders.postById.load(args.id);
    },

    deletePost: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callObject("Post", args.id, "delete", {});
      return true;
    },

    reactivatePost: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callObject("Post", args.id, "reactivate", {});
      ctx.loaders.postById.clear(args.id);
      return ctx.loaders.postById.load(args.id);
    },

    addPostTag: async (
      _parent: unknown,
      args: {
        postId: string;
        tagKind: string;
        tagValue: string;
        displayName?: string;
      },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callObject("Post", args.postId, "add_tag", {
        tag_kind: args.tagKind,
        tag_value: args.tagValue,
        display_name: args.displayName ?? args.tagValue,
      });
      ctx.loaders.postById.clear(args.postId);
      return ctx.loaders.postById.load(args.postId);
    },

    removePostTag: async (
      _parent: unknown,
      args: { postId: string; tagId: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callObject("Post", args.postId, "remove_tag", {
        tag_id: args.tagId,
      });
      ctx.loaders.postById.clear(args.postId);
      return ctx.loaders.postById.load(args.postId);
    },

    addPostContact: async (
      _parent: unknown,
      args: {
        postId: string;
        contactType: string;
        contactValue: string;
        contactLabel?: string;
      },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callObject("Post", args.postId, "add_contact", {
        contact_type: args.contactType,
        contact_value: args.contactValue,
        contact_label: args.contactLabel,
      });
      ctx.loaders.postById.clear(args.postId);
      return ctx.loaders.postById.load(args.postId);
    },

    removePostContact: async (
      _parent: unknown,
      args: { postId: string; contactId: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callObject("Post", args.postId, "remove_contact", {
        contact_id: args.contactId,
      });
      ctx.loaders.postById.clear(args.postId);
      return ctx.loaders.postById.load(args.postId);
    },

    addPostSchedule: async (
      _parent: unknown,
      args: {
        postId: string;
        input: {
          dayOfWeek?: number;
          opensAt?: string;
          closesAt?: string;
          timezone?: string;
          notes?: string;
          rrule?: string;
          dtstart?: string;
          dtend?: string;
          isAllDay?: boolean;
          durationMinutes?: number;
        };
      },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callObject("Post", args.postId, "add_schedule", {
        day_of_week: args.input.dayOfWeek,
        opens_at: args.input.opensAt,
        closes_at: args.input.closesAt,
        timezone: args.input.timezone,
        notes: args.input.notes,
        rrule: args.input.rrule,
        dtstart: args.input.dtstart,
        dtend: args.input.dtend,
        is_all_day: args.input.isAllDay,
        duration_minutes: args.input.durationMinutes,
      });
      ctx.loaders.postById.clear(args.postId);
      return ctx.loaders.postById.load(args.postId);
    },

    deletePostSchedule: async (
      _parent: unknown,
      args: { postId: string; scheduleId: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callObject("Post", args.postId, "delete_schedule", {
        schedule_id: args.scheduleId,
      });
      ctx.loaders.postById.clear(args.postId);
      return ctx.loaders.postById.load(args.postId);
    },

    regeneratePost: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callObject("Post", args.id, "regenerate", {});
      ctx.loaders.postById.clear(args.id);
      return ctx.loaders.postById.load(args.id);
    },

    createPost: async (
      _parent: unknown,
      args: {
        input: {
          title: string;
          bodyRaw: string;
          postType?: string;
          weight?: string;
          priority?: number;
          urgency?: string;
          location?: string;
          organizationId?: string;
        };
      },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.server.callService("Posts", "create_post", {
        title: args.input.title,
        body_raw: args.input.bodyRaw,
        post_type: args.input.postType,
        weight: args.input.weight,
        priority: args.input.priority,
        urgency: args.input.urgency,
        location: args.input.location,
        organization_id: args.input.organizationId,
      });
      return result;
    },

    updatePost: async (
      _parent: unknown,
      args: {
        id: string;
        input: {
          title?: string;
          bodyRaw?: string;
          bodyAst?: string;
          postType?: string;
          category?: string;
          weight?: string;
          priority?: number;
          urgency?: string;
          isUrgent?: boolean;
          pencilMark?: string;
          location?: string;
          zipCode?: string;
          sourceUrl?: string;
          organizationId?: string;
        };
      },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callObject("Post", args.id, "update_content", {
        title: args.input.title,
        body_raw: args.input.bodyRaw,
        body_ast: args.input.bodyAst ? JSON.parse(args.input.bodyAst) : undefined,
        post_type: args.input.postType,
        category: args.input.category,
        weight: args.input.weight,
        priority: args.input.priority,
        urgency: args.input.urgency,
        is_urgent: args.input.isUrgent,
        pencil_mark: args.input.pencilMark,
        location: args.input.location,
        zip_code: args.input.zipCode,
        source_url: args.input.sourceUrl,
        organization_id: args.input.organizationId,
      });
      ctx.loaders.postById.clear(args.id);
      return ctx.loaders.postById.load(args.id);
    },

    // Field group upserts
    upsertPostMedia: async (
      _parent: unknown,
      args: { postId: string; imageUrl?: string; caption?: string; credit?: string; mediaId?: string | null },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callService("Post", `${args.postId}/upsert_media`, {
        image_url: args.imageUrl,
        caption: args.caption,
        credit: args.credit,
        media_id: args.mediaId ?? null,
      });
      return true;
    },

    upsertPostMeta: async (
      _parent: unknown,
      args: { postId: string; kicker?: string; byline?: string; deck?: string; updated?: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callService("Post", `${args.postId}/upsert_meta`, {
        kicker: args.kicker,
        byline: args.byline,
        deck: args.deck,
        updated: args.updated,
      });
      return true;
    },

    upsertPostPerson: async (
      _parent: unknown,
      args: { postId: string; name?: string; role?: string; bio?: string; photoUrl?: string; quote?: string; photoMediaId?: string | null },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callService("Post", `${args.postId}/upsert_person`, {
        name: args.name,
        role: args.role,
        bio: args.bio,
        photo_url: args.photoUrl,
        quote: args.quote,
        photo_media_id: args.photoMediaId ?? null,
      });
      return true;
    },

    upsertPostLink: async (
      _parent: unknown,
      args: { postId: string; label?: string; url?: string; deadline?: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callService("Post", `${args.postId}/upsert_link`, {
        label: args.label,
        url: args.url,
        deadline: args.deadline,
      });
      return true;
    },

    upsertPostSourceAttr: async (
      _parent: unknown,
      args: { postId: string; sourceName?: string; attribution?: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callService("Post", `${args.postId}/upsert_source_attr`, {
        source_name: args.sourceName,
        attribution: args.attribution,
      });
      return true;
    },

    upsertPostDatetime: async (
      _parent: unknown,
      args: { postId: string; startAt?: string; endAt?: string; cost?: string; recurring?: boolean },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callService("Post", `${args.postId}/upsert_datetime`, {
        start_at: args.startAt,
        end_at: args.endAt,
        cost: args.cost,
        recurring: args.recurring,
      });
      return true;
    },

    upsertPostStatus: async (
      _parent: unknown,
      args: { postId: string; state?: string; verified?: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callService("Post", `${args.postId}/upsert_status`, {
        state: args.state,
        verified: args.verified,
      });
      return true;
    },

    upsertPostItems: async (
      _parent: unknown,
      args: { postId: string; items: Array<{ name: string; detail?: string | null }> },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callService("Post", `${args.postId}/upsert_items`, {
        items: args.items.map((i) => ({ name: i.name, detail: i.detail ?? null })),
      });
      ctx.loaders.postById.clear(args.postId);
      return true;
    },

  },

  PublicPost: {
    urgentNotes: (parent: { urgentNotes?: unknown[] }) => {
      return parent.urgentNotes ?? [];
    },
  },

  Post: {
    urgentNotes: (parent: { urgentNotes?: unknown[] }) => {
      return parent.urgentNotes ?? [];
    },

    bodyAst: (parent: { body_ast?: unknown; bodyAst?: unknown }) => {
      const ast = parent.body_ast ?? parent.bodyAst;
      if (!ast) return null;
      return typeof ast === "string" ? ast : JSON.stringify(ast);
    },

    organization: async (
      parent: { organizationId?: string },
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      if (!parent.organizationId) return null;
      return ctx.server.callService("Organizations", "get", {
        id: parent.organizationId,
      });
    },

    // Field group resolvers — lazy-load from the server
    media: async (parent: { id: string; media?: unknown[] }, _args: unknown, ctx: GraphQLContext) => {
      if (parent.media) return parent.media;
      const fg = await ctx.server.callService<{ media: unknown[] }>("Post", `${parent.id}/field_groups`, {});
      return (fg.media ?? []).map((m: unknown) => {
        const rec = m as Record<string, unknown>;
        return {
          imageUrl: rec.imageUrl ?? rec.image_url,
          caption: rec.caption,
          credit: rec.credit,
          mediaId: rec.mediaId ?? rec.media_id ?? null,
        };
      });
    },
    items: async (parent: { id: string; items?: unknown[] }, _args: unknown, ctx: GraphQLContext) => {
      if (parent.items) return parent.items;
      const fg = await ctx.server.callService<{ items: unknown[] }>("Post", `${parent.id}/field_groups`, {});
      return fg.items ?? [];
    },
    person: async (parent: { id: string; person?: unknown }, _args: unknown, ctx: GraphQLContext) => {
      if (parent.person !== undefined) return parent.person;
      const fg = await ctx.server.callService<{ person?: Record<string, unknown> }>("Post", `${parent.id}/field_groups`, {});
      if (!fg.person) return null;
      return {
        ...fg.person,
        photoUrl: fg.person.photoUrl ?? fg.person.photo_url,
        photoMediaId: fg.person.photoMediaId ?? fg.person.photo_media_id ?? null,
      };
    },
    link: async (parent: { id: string; link?: unknown }, _args: unknown, ctx: GraphQLContext) => {
      if (parent.link !== undefined) return parent.link;
      const fg = await ctx.server.callService<{ link?: unknown }>("Post", `${parent.id}/field_groups`, {});
      return fg.link ?? null;
    },
    sourceAttribution: async (parent: { id: string; sourceAttribution?: unknown; source_attribution?: unknown }, _args: unknown, ctx: GraphQLContext) => {
      if (parent.sourceAttribution !== undefined || parent.source_attribution !== undefined) {
        const sa = parent.sourceAttribution ?? parent.source_attribution;
        if (!sa) return null;
        const obj = sa as Record<string, unknown>;
        return { sourceName: obj.sourceName ?? obj.source_name, attribution: obj.attribution };
      }
      const fg = await ctx.server.callService<{ sourceAttribution?: Record<string, unknown> }>("Post", `${parent.id}/field_groups`, {});
      if (!fg.sourceAttribution) return null;
      return { sourceName: fg.sourceAttribution.sourceName, attribution: fg.sourceAttribution.attribution };
    },
    meta: async (parent: { id: string; meta?: unknown }, _args: unknown, ctx: GraphQLContext) => {
      if (parent.meta !== undefined) return parent.meta;
      const fg = await ctx.server.callService<{ meta?: unknown }>("Post", `${parent.id}/field_groups`, {});
      return fg.meta ?? null;
    },
    datetime: async (parent: { id: string; datetime?: unknown }, _args: unknown, ctx: GraphQLContext) => {
      if (parent.datetime !== undefined) return parent.datetime;
      const fg = await ctx.server.callService<{ datetime?: Record<string, unknown> }>("Post", `${parent.id}/field_groups`, {});
      if (!fg.datetime) return null;
      return { start: fg.datetime.startAt, end: fg.datetime.endAt, cost: fg.datetime.cost, recurring: fg.datetime.recurring };
    },
    postStatus: async (parent: { id: string; postStatus?: unknown; post_status?: unknown }, _args: unknown, ctx: GraphQLContext) => {
      if (parent.postStatus !== undefined || parent.post_status !== undefined) return parent.postStatus ?? parent.post_status ?? null;
      const fg = await ctx.server.callService<{ postStatus?: unknown }>("Post", `${parent.id}/field_groups`, {});
      return fg.postStatus ?? null;
    },
    schedule: async (parent: { id: string; schedule?: unknown[] }, _args: unknown, ctx: GraphQLContext) => {
      if (parent.schedule) return parent.schedule;
      const fg = await ctx.server.callService<{ schedule: unknown[] }>("Post", `${parent.id}/field_groups`, {});
      return fg.schedule ?? [];
    },
    relatedPosts: async (parent: { id: string; relatedPosts?: unknown[] }, _args: unknown, ctx: GraphQLContext) => {
      if (parent.relatedPosts) return parent.relatedPosts;
      return ctx.server.callService("Post", `${parent.id}/related`, {});
    },
  },
};
