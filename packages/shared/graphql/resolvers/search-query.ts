import type { GraphQLContext } from "../context";

function requireAdmin(ctx: GraphQLContext) {
  if (!ctx.user?.isAdmin) {
    throw new Error("Unauthorized: admin access required");
  }
}

export const searchQueryResolvers = {
  Query: {
    searchQueries: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callService<{
        queries: unknown[];
      }>("Websites", "list_search_queries", {});
      return result.queries;
    },
  },

  Mutation: {
    createSearchQuery: async (
      _parent: unknown,
      args: { queryText: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService(
        "Websites",
        "create_search_query",
        { query_text: args.queryText }
      );
    },

    updateSearchQuery: async (
      _parent: unknown,
      args: { id: string; queryText: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService(
        "Websites",
        "update_search_query",
        { id: args.id, query_text: args.queryText }
      );
    },

    toggleSearchQuery: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService(
        "Websites",
        "toggle_search_query",
        { id: args.id }
      );
    },

    deleteSearchQuery: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callService(
        "Websites",
        "delete_search_query",
        { id: args.id }
      );
      return true;
    },

    runScheduledDiscovery: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callService(
        "Websites",
        "run_scheduled_discovery",
        {}
      );
      return true;
    },
  },
};
