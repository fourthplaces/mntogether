import type { GraphQLContext } from "../context";
import { requireAuth } from "../auth";

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
        limit?: number;
        offset?: number;
      },
      ctx: GraphQLContext
    ) => {
      requireAuth(ctx);
      return ctx.restate.callService("Posts", "list", {
        status: args.status,
        search: args.search,
        first: args.limit,
        offset: args.offset,
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
  },

  Post: {
    comments: (parent: { id: string }, _args: unknown, ctx: GraphQLContext) => {
      return ctx.loaders.commentsByPostId.load(parent.id);
    },
  },
};
