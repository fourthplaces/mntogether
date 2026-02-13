import { graphql } from "@/gql";

export const RecentChatsQuery = graphql(`
  query RecentChats($limit: Int) {
    recentChats(limit: $limit) {
      id
      title
      createdAt
      messageCount
    }
  }
`);

export const ChatMessagesQuery = graphql(`
  query ChatMessages($chatroomId: ID!) {
    chatMessages(chatroomId: $chatroomId) {
      id
      chatroomId
      senderType
      content
      createdAt
    }
  }
`);

export const CreateChatMutation = graphql(`
  mutation CreateChat($language: String, $withAgent: String) {
    createChat(language: $language, withAgent: $withAgent) {
      id
      title
      createdAt
      messageCount
    }
  }
`);

export const SendChatMessageMutation = graphql(`
  mutation SendChatMessage($chatroomId: ID!, $content: String!) {
    sendChatMessage(chatroomId: $chatroomId, content: $content) {
      id
      chatroomId
      senderType
      content
      createdAt
    }
  }
`);
