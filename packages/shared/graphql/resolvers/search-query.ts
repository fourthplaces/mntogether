import type { GraphQLContext } from "../context";

export const searchQueryResolvers = {
  Query: {
    searchQueries: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
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
      await ctx.restate.callService(
        "Websites",
        "run_scheduled_discovery",
        {}
      );
      return true;
    },
  },
};
