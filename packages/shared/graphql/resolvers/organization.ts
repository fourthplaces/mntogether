import type { GraphQLContext } from "../context";

export const organizationResolvers = {
  Query: {
    organizations: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{
        organizations: unknown[];
      }>("Organizations", "list", {});
      return result.organizations;
    },

    organization: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Organizations", "get", {
        id: args.id,
      });
    },

    publicOrganizations: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{
        organizations: unknown[];
      }>("Organizations", "public_list", {});
      return result.organizations;
    },

    publicOrganization: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Organizations", "public_get", {
        id: args.id,
      });
    },

    organizationChecklist: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Organizations", "get_checklist", {
        id: args.id,
      });
    },
  },

  Mutation: {
    createOrganization: async (
      _parent: unknown,
      args: { name: string; description?: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Organizations", "create", {
        name: args.name,
        description: args.description ?? null,
      });
    },

    updateOrganization: async (
      _parent: unknown,
      args: { id: string; name: string; description?: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Organizations", "update", {
        id: args.id,
        name: args.name,
        description: args.description ?? null,
      });
      return ctx.restate.callService("Organizations", "get", {
        id: args.id,
      });
    },

    deleteOrganization: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Organizations", "delete", {
        id: args.id,
      });
      return true;
    },

    approveOrganization: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Organizations", "approve", {
        id: args.id,
      });
      return ctx.restate.callService("Organizations", "get", {
        id: args.id,
      });
    },

    rejectOrganization: async (
      _parent: unknown,
      args: { id: string; reason: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Organizations", "reject", {
        id: args.id,
        reason: args.reason,
      });
      return ctx.restate.callService("Organizations", "get", {
        id: args.id,
      });
    },

    suspendOrganization: async (
      _parent: unknown,
      args: { id: string; reason: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Organizations", "suspend", {
        id: args.id,
        reason: args.reason,
      });
      return ctx.restate.callService("Organizations", "get", {
        id: args.id,
      });
    },

    setOrganizationStatus: async (
      _parent: unknown,
      args: { id: string; status: string; reason?: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Organizations", "set_status", {
        id: args.id,
        status: args.status,
        reason: args.reason ?? null,
      });
      return ctx.restate.callService("Organizations", "get", {
        id: args.id,
      });
    },

    toggleChecklistItem: async (
      _parent: unknown,
      args: {
        organizationId: string;
        checklistKey: string;
        checked: boolean;
      },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService(
        "Organizations",
        "toggle_checklist_item",
        {
          organization_id: args.organizationId,
          checklist_key: args.checklistKey,
          checked: args.checked,
        }
      );
      return ctx.restate.callService("Organizations", "get_checklist", {
        id: args.organizationId,
      });
    },

    regenerateOrganization: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Organizations", "regenerate", {
        id: args.id,
      });
    },

    extractOrgPosts: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Organizations", "extract_org_posts", {
        id: args.id,
      });
      return true;
    },

    cleanUpOrgPosts: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Organizations", "clean_up_org_posts", {
        id: args.id,
      });
      return true;
    },

    runCurator: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Organizations", "run_curator", {
        id: args.id,
      });
      return true;
    },

    removeAllOrgPosts: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Organizations", "remove_all_posts", {
        id: args.id,
      });
      return true;
    },

    removeAllOrgNotes: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Organizations", "remove_all_notes", {
        id: args.id,
      });
      return true;
    },

    rewriteNarratives: async (
      _parent: unknown,
      args: { organizationId: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Posts", "rewrite_narratives", {
        organization_id: args.organizationId,
      });
    },
  },

  Organization: {
    sources: async (
      parent: { id: string },
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{ sources: unknown[] }>(
        "Sources",
        "list_by_organization",
        { organization_id: parent.id }
      );
      return result.sources;
    },

    posts: async (
      parent: { id: string },
      args: { limit?: number },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService(
        "Posts",
        "list_by_organization",
        {
          organization_id: parent.id,
          ...(args.limit ? { limit: args.limit } : {}),
        }
      );
    },

    notes: async (
      parent: { id: string },
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{ notes: unknown[] }>(
        "Notes",
        "list_for_entity",
        { noteable_type: "organization", noteable_id: parent.id }
      );
      return result.notes;
    },

    checklist: async (
      parent: { id: string },
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService(
        "Organizations",
        "get_checklist",
        { id: parent.id }
      );
    },
  },
};
