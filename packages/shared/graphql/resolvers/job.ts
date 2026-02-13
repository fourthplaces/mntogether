import type { GraphQLContext } from "../context";

function requireAdmin(ctx: GraphQLContext) {
  if (!ctx.user?.isAdmin) {
    throw new Error("Unauthorized: admin access required");
  }
}

export const jobResolvers = {
  Query: {
    jobs: async (
      _parent: unknown,
      args: { status?: string; limit?: number },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
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
