import type { GraphQLContext } from "../context";
import { requireAuth, requireAdmin } from "../auth";

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
      return ctx.restate.callService("Posts", "public_list", {
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
      return ctx.restate.callService("Posts", "public_filters", {});
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
        zipCode?: string;
        radiusMiles?: number;
        limit?: number;
        offset?: number;
      },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService("Posts", "list", {
        status: args.status,
        search: args.search,
        post_type: args.postType,
        submission_type: args.submissionType,
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
      requireAdmin(ctx);
      return ctx.restate.callService("Posts", "stats", {
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
        await ctx.restate.callObject(
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
        await ctx.restate.callObject(
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
      requireAdmin(ctx);
      await ctx.restate.callObject("Post", args.id, "approve", {});
      return ctx.loaders.postById.load(args.id);
    },

    rejectPost: async (
      _parent: unknown,
      args: { id: string; reason?: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Post", args.id, "reject", {
        reason: args.reason ?? "Rejected by admin",
      });
      return ctx.loaders.postById.load(args.id);
    },

    archivePost: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Post", args.id, "archive", {});
      return ctx.loaders.postById.load(args.id);
    },

    deletePost: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Post", args.id, "delete", {});
      return true;
    },

    reactivatePost: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Post", args.id, "reactivate", {});
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
      requireAdmin(ctx);
      await ctx.restate.callObject("Post", args.postId, "add_tag", {
        tag_kind: args.tagKind,
        tag_value: args.tagValue,
        display_name: args.displayName ?? args.tagValue,
      });
      return ctx.loaders.postById.load(args.postId);
    },

    removePostTag: async (
      _parent: unknown,
      args: { postId: string; tagId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Post", args.postId, "remove_tag", {
        tag_id: args.tagId,
      });
      return ctx.loaders.postById.load(args.postId);
    },

    regeneratePost: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Post", args.id, "regenerate", {});
      return ctx.loaders.postById.load(args.id);
    },

    regeneratePostTags: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Post", args.id, "regenerate_tags", {});
      return ctx.loaders.postById.load(args.id);
    },

    updatePostCapacity: async (
      _parent: unknown,
      args: { id: string; capacityStatus: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Post", args.id, "update_capacity", {
        capacity_status: args.capacityStatus,
      });
      return ctx.loaders.postById.load(args.id);
    },

    batchScorePosts: async (
      _parent: unknown,
      args: { limit?: number },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService("Posts", "batch_score_posts", {
        limit: args.limit,
      });
    },

    submitResourceLink: async (
      _parent: unknown,
      args: { url: string; context?: string; submitterContact?: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Posts", "submit_resource_link", {
        url: args.url,
        context: args.context ?? null,
        submitter_contact: args.submitterContact ?? null,
      });
    },

    addComment: async (
      _parent: unknown,
      args: { postId: string; content: string; parentMessageId?: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callObject("Post", args.postId, "add_comment", {
        content: args.content,
        parent_message_id: args.parentMessageId ?? null,
      });
    },
  },

  Post: {
    comments: (parent: { id: string }, _args: unknown, ctx: GraphQLContext) => {
      return ctx.loaders.commentsByPostId.load(parent.id);
    },
  },
};
