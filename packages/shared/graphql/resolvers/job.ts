import type { GraphQLContext } from "../context";

export const jobResolvers = {
  Query: {
    jobs: async (
      _parent: unknown,
      args: { status?: string; limit?: number },
      ctx: GraphQLContext
    ) => {
      const body: Record<string, unknown> = {};
      if (args.status) body.status = args.status;
      if (args.limit) body.limit = args.limit;
      const result = await ctx.restate.callService<{
        jobs: unknown[];
      }>("Jobs", "list", body);
      return result.jobs;
    },
  },
};
