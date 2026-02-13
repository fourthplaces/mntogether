import DataLoader from "dataloader";
import type { RestateClient } from "../restate-client";

interface PostResult {
  id: string;
  [key: string]: unknown;
}

interface PostList {
  posts: PostResult[];
  totalCount: number;
  hasNextPage: boolean;
  hasPreviousPage: boolean;
}

interface CommentResult {
  id: string;
  [key: string]: unknown;
}

interface CommentList {
  messages: CommentResult[];
}

export interface PostLoaders {
  postById: DataLoader<string, PostResult | null>;
  postsByOrgId: DataLoader<string, PostResult[]>;
  commentsByPostId: DataLoader<string, CommentResult[]>;
}

export function createPostLoaders(restate: RestateClient): PostLoaders {
  return {
    postById: new DataLoader(async (ids) => {
      const results = await Promise.all(
        ids.map((id) =>
          restate
            .callObject<PostResult>("Post", id as string, "get", {})
            .catch(() => null)
        )
      );
      return results;
    }),

    postsByOrgId: new DataLoader(async (orgIds) => {
      const results = await Promise.all(
        orgIds.map((orgId) =>
          restate
            .callService<PostList>("Posts", "list", {
              organization_id: orgId,
            })
            .then((r) => r.posts)
            .catch(() => [] as PostResult[])
        )
      );
      return results;
    }),

    commentsByPostId: new DataLoader(async (postIds) => {
      const results = await Promise.all(
        postIds.map((postId) =>
          restate
            .callObject<CommentList>(
              "Post",
              postId as string,
              "get_comments",
              {}
            )
            .then((r) => r.messages)
            .catch(() => [] as CommentResult[])
        )
      );
      return results;
    }),
  };
}
