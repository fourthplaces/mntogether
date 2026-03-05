import type { GraphQLContext } from "../context";

export const mediaResolvers = {
  Query: {
    mediaLibrary: async (
      _parent: unknown,
      args: { limit?: number; offset?: number; contentType?: string },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("MediaService", "list", {
        limit: args.limit,
        offset: args.offset,
        content_type: args.contentType,
      });
    },

    presignedUpload: async (
      _parent: unknown,
      args: { filename: string; contentType: string; sizeBytes: number },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("MediaService", "presigned_upload", {
        filename: args.filename,
        content_type: args.contentType,
        size_bytes: args.sizeBytes,
      });
    },
  },

  Mutation: {
    confirmUpload: async (
      _parent: unknown,
      args: {
        storageKey: string;
        publicUrl: string;
        filename: string;
        contentType: string;
        sizeBytes: number;
        altText?: string;
        width?: number;
        height?: number;
      },
      ctx: GraphQLContext
    ) => {
      return ctx.server.callService("MediaService", "confirm_upload", {
        storage_key: args.storageKey,
        public_url: args.publicUrl,
        filename: args.filename,
        content_type: args.contentType,
        size_bytes: args.sizeBytes,
        alt_text: args.altText,
        width: args.width,
        height: args.height,
      });
    },

    deleteMedia: async (
      _parent: unknown,
      args: { id: string },
      ctx: GraphQLContext
    ) => {
      await ctx.server.callService("MediaService", "delete", {
        id: args.id,
      });
      return true;
    },
  },
};
