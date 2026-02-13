import type { GraphQLContext } from "../context";
import { requireAdmin } from "../auth";

export const websiteResolvers = {
  Query: {
    websites: async (
      _parent: unknown,
      args: {
        status?: string;
        search?: string;
        limit?: number;
        offset?: number;
      },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService("Websites", "list", {
        status: args.status ?? null,
        search: args.search ?? null,
        limit: args.limit ?? 20,
        offset: args.offset ?? 0,
      });
    },

    website: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callObject("Website", args.id, "get", {});
    },

    websitePages: async (
      _parent: unknown,
      args: { domain: string; limit?: number },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callService<{ pages: unknown[] }>(
        "Extraction",
        "list_pages",
        { domain: args.domain, limit: args.limit ?? 50 }
      );
      return result.pages;
    },

    websitePageCount: async (
      _parent: unknown,
      args: { domain: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callService<{ count: number }>(
        "Extraction",
        "count_pages",
        { domain: args.domain }
      );
      return result.count;
    },

    websiteAssessment: async (
      _parent: unknown,
      args: { websiteId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callObject<{
        assessment: unknown | null;
      }>("Website", args.websiteId, "get_assessment", {});
      return result.assessment;
    },

    websitePosts: async (
      _parent: unknown,
      args: { websiteId: string; limit?: number },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService("Posts", "list", {
        source_type: "website",
        source_id: args.websiteId,
        first: args.limit ?? 100,
      });
    },
  },

  Mutation: {
    submitNewWebsite: async (
      _parent: unknown,
      args: { url: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService("Websites", "submit", {
        url: args.url,
      });
    },

    approveWebsite: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Website", args.id, "approve", {});
      return ctx.restate.callObject("Website", args.id, "get", {});
    },

    rejectWebsite: async (
      _parent: unknown,
      args: { id: string; reason: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Website", args.id, "reject", {
        reason: args.reason,
      });
      return ctx.restate.callObject("Website", args.id, "get", {});
    },

    crawlWebsite: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const workflowId = `crawl-${args.id}-${Date.now()}`;
      await ctx.restate.callObject("CrawlWebsiteWorkflow", workflowId, "run", {
        website_id: args.id,
      });
      return true;
    },

    generateWebsiteAssessment: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject(
        "Website",
        args.id,
        "generate_assessment",
        {}
      );
      return true;
    },

    regenerateWebsitePosts: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callObject<{ status: string }>(
        "Website",
        args.id,
        "regenerate_posts",
        {}
      );
      const workflowId = result.status.replace("started:", "");
      return { workflowId, status: result.status };
    },

    deduplicateWebsitePosts: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callObject<{ status: string }>(
        "Website",
        args.id,
        "deduplicate_posts",
        {}
      );
      const workflowId = result.status.replace("started:", "");
      return { workflowId, status: result.status };
    },

    extractWebsiteOrganization: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject(
        "Website",
        args.id,
        "extract_organization",
        {}
      );
      return ctx.restate.callObject("Website", args.id, "get", {});
    },

    assignWebsiteOrganization: async (
      _parent: unknown,
      args: { id: string; organizationId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject(
        "Website",
        args.id,
        "assign_organization",
        { organization_id: args.organizationId }
      );
      return ctx.restate.callObject("Website", args.id, "get", {});
    },

    unassignWebsiteOrganization: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject(
        "Website",
        args.id,
        "unassign_organization",
        {}
      );
      return ctx.restate.callObject("Website", args.id, "get", {});
    },

    approvePostInline: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Post", args.id, "approve", {});
      return ctx.loaders.postById.load(args.id);
    },

    rejectPostInline: async (
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
  },
};
