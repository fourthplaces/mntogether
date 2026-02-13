import type { GraphQLContext } from "../context";

export const syncResolvers = {
  Query: {
    syncBatches: async (
      _parent: unknown,
      args: { status?: string; limit?: number },
      ctx: GraphQLContext
    ) => {
      const body: Record<string, unknown> = {};
      if (args.status) body.status = args.status;
      if (args.limit) body.limit = args.limit;
      return ctx.restate.callService<{ batches: unknown[] }>(
        "Sync",
        "list_batches",
        body
      );
    },

    syncProposals: async (
      _parent: unknown,
      args: { batchId: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService<{ proposals: unknown[] }>(
        "Sync",
        "list_proposals",
        { batch_id: args.batchId }
      );
    },
  },

  Mutation: {
    approveProposal: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Sync", "approve_proposal", {
        proposal_id: args.id,
      });
      return true;
    },

    rejectProposal: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Sync", "reject_proposal", {
        proposal_id: args.id,
      });
      return true;
    },

    approveBatch: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Sync", "approve_batch", {
        batch_id: args.id,
      });
      return true;
    },

    rejectBatch: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Sync", "reject_batch", {
        batch_id: args.id,
      });
      return true;
    },

    refineProposal: async (
      _parent: unknown,
      args: { proposalId: string; comment: string },
      ctx: GraphQLContext
    ) => {
      const workflowId = `refine-${args.proposalId}-${Date.now()}`;
      await ctx.restate.callObject(
        "RefineProposalWorkflow",
        workflowId,
        "run",
        {
          proposal_id: args.proposalId,
          comment: args.comment,
        }
      );
      return true;
    },
  },
};
