import type { GraphQLContext } from "../context";

export const noteResolvers = {
  Query: {
    notes: async (
      _parent: unknown,
      args: { severity?: string; isPublic?: boolean; limit?: number; offset?: number },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.server.callService<{
        notes: unknown[];
        totalCount: number;
      }>("Notes", "list", {
        severity: args.severity ?? null,
        is_public: args.isPublic ?? null,
        limit: args.limit ?? 50,
        offset: args.offset ?? 0,
      });
      return {
        notes: result.notes,
        totalCount: result.totalCount,
      };
    },

    note: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("Notes", "get", { id: args.id });
    },

    entityNotes: async (
      _parent: unknown,
      args: { noteableType: string; noteableId: string },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.server.callService<{
        notes: unknown[];
      }>("Notes", "list_for_entity", {
        noteable_type: args.noteableType,
        noteable_id: args.noteableId,
      });
      return result.notes;
    },

    organizationPosts: async (
      _parent: unknown,
      args: { organizationId: string; limit?: number },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService(
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
      return ctx.server.callService("Notes", "create", {
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
      return ctx.server.callService("Notes", "update", {
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
      await ctx.server.callService("Notes", "delete", {
        id: args.id,
      });
      return true;
    },

    unlinkNote: async (
      _parent: unknown,
      args: { noteId: string; postId: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callService("Notes", "unlink", {
        note_id: args.noteId,
        noteable_type: "post",
        noteable_id: args.postId,
      });
      return true;
    },

    autoAttachNotes: async (
      _parent: unknown,
      args: { organizationId: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService(
        "Notes",
        "auto_attach_notes",
        { organization_id: args.organizationId }
      );
    },
  },
};
