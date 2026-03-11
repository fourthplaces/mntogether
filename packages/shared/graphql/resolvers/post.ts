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
          descriptionMarkdown: string;
          summary?: string;
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
        description_markdown: args.input.descriptionMarkdown,
        summary: args.input.summary,
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
          descriptionMarkdown?: string;
          summary?: string;
          postType?: string;
          category?: string;
          weight?: string;
          priority?: number;
          urgency?: string;
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
        description_markdown: args.input.descriptionMarkdown,
        summary: args.input.summary,
        post_type: args.input.postType,
        category: args.input.category,
        weight: args.input.weight,
        priority: args.input.priority,
        urgency: args.input.urgency,
        location: args.input.location,
        zip_code: args.input.zipCode,
        source_url: args.input.sourceUrl,
        organization_id: args.input.organizationId,
      });
      ctx.loaders.postById.clear(args.id);
      return ctx.loaders.postById.load(args.id);
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
  },
};
