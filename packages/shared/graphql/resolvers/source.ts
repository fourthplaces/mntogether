import type { GraphQLContext } from "../context";
import { requireAdmin } from "../auth";

export const sourceResolvers = {
  Query: {
    sources: async (
      _parent: unknown,
      args: {
        status?: string;
        sourceType?: string;
        search?: string;
        limit?: number;
        offset?: number;
      },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callService<{
        sources: unknown[];
        totalCount: number;
        hasNextPage: boolean;
        hasPreviousPage: boolean;
      }>("Sources", "list", {
        status: args.status ?? null,
        source_type: args.sourceType ?? null,
        search: args.search ?? null,
        limit: args.limit ?? 20,
        offset: args.offset ?? 0,
      });
      return result;
    },

    source: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callObject("Source", args.id, "get", {});
    },

    sourcePages: async (
      _parent: unknown,
      args: { sourceId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callObject<{ pages: unknown[] }>(
        "Source",
        args.sourceId,
        "list_pages",
        {}
      );
      return result.pages;
    },

    sourcePageCount: async (
      _parent: unknown,
      args: { sourceId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callObject<{ count: number }>(
        "Source",
        args.sourceId,
        "count_pages",
        {}
      );
      return result.count;
    },

    sourceAssessment: async (
      _parent: unknown,
      args: { sourceId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callObject<{
        assessment: unknown | null;
      }>("Source", args.sourceId, "get_assessment", {});
      return result.assessment;
    },

    searchSourcesByContent: async (
      _parent: unknown,
      args: { query: string; limit?: number },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService("Sources", "search_by_content", {
        query: args.query,
        limit: args.limit ?? 100,
      });
    },

    extractionPage: async (
      _parent: unknown,
      args: { url: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callService<{
        page: unknown | null;
      }>("Extraction", "get_page", { url: args.url });
      return result.page;
    },

    workflowStatus: async (
      _parent: unknown,
      args: { workflowName: string; workflowId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callObject<string>(
        args.workflowName,
        args.workflowId,
        "get_status",
        {}
      );
    },
  },

  Mutation: {
    submitWebsite: async (
      _parent: unknown,
      args: { url: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService("Sources", "submit_website", {
        url: args.url,
      });
    },

    lightCrawlAll: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      return ctx.restate.callService("Sources", "light_crawl_all", {});
    },

    approveSource: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Source", args.id, "approve", {});
      return ctx.restate.callObject("Source", args.id, "get", {});
    },

    rejectSource: async (
      _parent: unknown,
      args: { id: string; reason: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Source", args.id, "reject", {
        reason: args.reason,
      });
      return ctx.restate.callObject("Source", args.id, "get", {});
    },

    crawlSource: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const source = await ctx.restate.callObject<{ sourceType: string }>(
        "Source",
        args.id,
        "get",
        {}
      );
      const workflowId = `crawl-${args.id}-${Date.now()}`;
      if (source.sourceType === "website") {
        await ctx.restate.callObject(
          "CrawlWebsiteWorkflow",
          workflowId,
          "run",
          { website_id: args.id }
        );
      } else {
        await ctx.restate.callObject(
          "CrawlSocialSourceWorkflow",
          workflowId,
          "run",
          { source_id: args.id }
        );
      }
      return true;
    },

    generateSourceAssessment: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject(
        "Source",
        args.id,
        "generate_assessment",
        {}
      );
      return true;
    },

    regenerateSourcePosts: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callObject<{ status: string }>(
        "Source",
        args.id,
        "regenerate_posts",
        {}
      );
      const workflowId = result.status.replace("started:", "");
      return { workflowId, status: result.status };
    },

    deduplicateSourcePosts: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      const result = await ctx.restate.callObject<{ status: string }>(
        "Source",
        args.id,
        "deduplicate_posts",
        {}
      );
      const workflowId = result.status.replace("started:", "");
      return { workflowId, status: result.status };
    },

    extractSourceOrganization: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject(
        "Source",
        args.id,
        "extract_organization",
        {}
      );
      return ctx.restate.callObject("Source", args.id, "get", {});
    },

    assignSourceOrganization: async (
      _parent: unknown,
      args: { id: string; organizationId: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject("Source", args.id, "assign_organization", {
        organization_id: args.organizationId,
      });
      return ctx.restate.callObject("Source", args.id, "get", {});
    },

    unassignSourceOrganization: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      requireAdmin(ctx);
      await ctx.restate.callObject(
        "Source",
        args.id,
        "unassign_organization",
        {}
      );
      return ctx.restate.callObject("Source", args.id, "get", {});
    },
  },
};
