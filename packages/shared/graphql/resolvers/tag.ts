import type { GraphQLContext } from "../context";

export const tagResolvers = {
  Query: {
    tagKinds: async (
      _parent: unknown,
      _args: unknown,
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{ kinds: unknown[] }>(
        "Tags",
        "list_kinds",
        {}
      );
      return result.kinds;
    },

    tags: async (
      _parent: unknown,
      args: { kind?: string },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{ tags: unknown[] }>(
        "Tags",
        "list_tags",
        { kind: args.kind }
      );
      return result.tags;
    },
  },

  Mutation: {
    createTagKind: async (
      _parent: unknown,
      args: {
        slug: string;
        displayName: string;
        description?: string;
        required?: boolean;
        isPublic?: boolean;
        allowedResourceTypes?: string[];
      },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Tags", "create_kind", {
        slug: args.slug,
        display_name: args.displayName,
        description: args.description ?? null,
        allowed_resource_types: args.allowedResourceTypes ?? [],
        required: args.required ?? false,
        is_public: args.isPublic ?? false,
      });
    },

    updateTagKind: async (
      _parent: unknown,
      args: {
        id: string;
        displayName?: string;
        description?: string;
        required?: boolean;
        isPublic?: boolean;
        allowedResourceTypes?: string[];
      },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Tags", "update_kind", {
        id: args.id,
        display_name: args.displayName,
        description: args.description,
        allowed_resource_types: args.allowedResourceTypes,
        required: args.required,
        is_public: args.isPublic,
      });
    },

    deleteTagKind: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Tags", "delete_kind", { id: args.id });
      return true;
    },

    createTag: async (
      _parent: unknown,
      args: {
        kind: string;
        value: string;
        displayName?: string;
        color?: string;
        description?: string;
        emoji?: string;
      },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Tags", "create_tag", {
        kind: args.kind,
        value: args.value,
        display_name: args.displayName ?? null,
        color: args.color ?? null,
        description: args.description ?? null,
        emoji: args.emoji ?? null,
      });
    },

    updateTag: async (
      _parent: unknown,
      args: {
        id: string;
        displayName?: string;
        color?: string;
        description?: string;
        emoji?: string;
      },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Tags", "update_tag", {
        id: args.id,
        display_name: args.displayName,
        color: args.color,
        description: args.description,
        emoji: args.emoji,
      });
    },

    deleteTag: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.restate.callService("Tags", "delete_tag", { id: args.id });
      return true;
    },
  },
};
