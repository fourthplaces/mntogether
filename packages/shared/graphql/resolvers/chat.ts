import type { GraphQLContext } from "../context";

export const chatResolvers = {
  Query: {
    recentChats: async (
      _parent: unknown,
      args: { limit?: number },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callService<{
        chats: unknown[];
      }>("Chats", "list_recent", {
        limit: args.limit || 1,
      });
      return result.chats;
    },

    chatMessages: async (
      _parent: unknown,
      args: { chatroomId: string },
      ctx: GraphQLContext
    ) => {
      const result = await ctx.restate.callObject<{
        messages: unknown[];
      }>("Chat", args.chatroomId, "get_messages", {});
      return result.messages;
    },
  },

  Mutation: {
    createChat: async (
      _parent: unknown,
      args: { language?: string; withAgent?: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Chats", "create", {
        language: args.language || "en",
        with_agent: args.withAgent || undefined,
      });
    },

    sendChatMessage: async (
      _parent: unknown,
      args: { chatroomId: string; content: string },
      ctx: GraphQLContext
    ) => {
      return ctx.restate.callService("Chat", "send_message", {
        chatroom_id: args.chatroomId,
        content: args.content,
      });
    },
  },
};
