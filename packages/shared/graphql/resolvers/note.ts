import type { GraphQLContext } from "../context";

function requireAdmin(ctx: GraphQLContext) {
  if (!ctx.user?.isAdmin) {
    throw new Error("Unauthorized: admin access required");
  }
}

export const noteResolvers = {
  Query: {
    entityProposals: async (
      _parent: unknown,
      args: { entityId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callService<{
        proposals: unknown[];
      }>("Sync", "list_entity_proposals", {
        entity_id: args.entityId,
      });
      return result.proposals;
    },

    entityNotes: async (
      _parent: unknown,
      args: { noteableType: string; noteableId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callService<{
        notes: unknown[];
      }>("Notes", "list_for_entity", {
        noteable_type: args.noteableType,
        noteable_id: args.noteableId,
      });
      return result.notes;
    },

    organizationSources: async (
      _parent: unknown,
      args: { organizationId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callService<{
        sources: unknown[];
      }>("Sources", "list_by_organization", {
        organization_id: args.organizationId,
      });
      return result.sources;
    },

    organizationPosts: async (
      _parent: unknown,
      args: { organizationId: string; limit?: number },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService(
        "Posts",
        "list_by_organization",
        {
          organization_id: args.organizationId,
          ...(args.limit ? { limit: args.limit } : {}),
        }
      );
    },
  },

  Mutation: {
    createNote: async (
      _parent: unknown,
      args: {
        noteableType: string;
        noteableId: string;
        content: string;
        severity?: string;
        isPublic?: boolean;
        ctaText?: string;
        sourceUrl?: string;
      },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService("Notes", "create", {
        noteable_type: args.noteableType,
        noteable_id: args.noteableId,
        content: args.content,
        severity: args.severity || "info",
        is_public: args.isPublic || false,
        cta_text: args.ctaText || null,
        source_url: args.sourceUrl || null,
      });
    },

    updateNote: async (
      _parent: unknown,
      args: {
        id: string;
        content: string;
        severity?: string;
        isPublic?: boolean;
        ctaText?: string;
        sourceUrl?: string;
        expiredAt?: string;
      },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService("Notes", "update", {
        id: args.id,
        content: args.content,
        severity: args.severity,
        is_public: args.isPublic,
        cta_text: args.ctaText,
        source_url: args.sourceUrl,
        expired_at: args.expiredAt,
      });
    },

    deleteNote: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callService("Notes", "delete", {
        id: args.id,
      });
      return true;
    },

    unlinkNote: async (
      _parent: unknown,
      args: { noteId: string; postId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callService("Notes", "unlink", {
        note_id: args.noteId,
        noteable_type: "post",
        noteable_id: args.postId,
      });
      return true;
    },

    generateNotesFromSources: async (
      _parent: unknown,
      args: { organizationId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService(
        "Notes",
        "generate_from_sources",
        { organization_id: args.organizationId }
      );
    },

    autoAttachNotes: async (
      _parent: unknown,
      args: { organizationId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService(
        "Notes",
        "auto_attach_notes",
        { organization_id: args.organizationId }
      );
    },

    createSocialSource: async (
      _parent: unknown,
      args: {
        organizationId: string;
        platform: string;
        identifier: string;
      },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService("Sources", "create_social", {
        organization_id: args.organizationId,
        platform: args.platform,
        identifier: args.identifier,
      });
    },

    crawlAllOrgSources: async (
      _parent: unknown,
      args: { organizationId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      // Get org sources first
      const sourcesResult = await ctx.restate.callService<{
        sources: Array<{ id: string; sourceType: string }>;
      }>("Sources", "list_by_organization", {
        organization_id: args.organizationId,
      });

      // Crawl each source
      await Promise.all(
        sourcesResult.sources.map((source) => {
          const workflowId = `crawl-${source.id}-${Date.now()}`;
          if (source.sourceType === "website") {
            return ctx.restate.callObject(
              "CrawlWebsiteWorkflow",
              workflowId,
              "run",
              { website_id: source.id }
            );
          } else {
            return ctx.restate.callObject(
              "CrawlSocialSourceWorkflow",
              workflowId,
              "run",
              { source_id: source.id }
            );
          }
        })
      );
      return true;
    },
  },
};
