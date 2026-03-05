import DataLoader from "dataloader";
import type { ServerClient } from "../server-client";

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

export interface PostLoaders {
  postById: DataLoader<string, PostResult | null>;
  postsByOrgId: DataLoader<string, PostResult[]>;
}

export function createPostLoaders(server: ServerClient): PostLoaders {
  return {
    postById: new DataLoader(async (ids) => {
      const results = await Promise.all(
        ids.map((id) =>
          server
            .callObject<PostResult>("Post", id as string, "get", {})
            .catch(() => null)
        )
      );
      return results;
    }),

    postsByOrgId: new DataLoader(async (orgIds) => {
      const results = await Promise.all(
        orgIds.map((orgId) =>
          server
            .callService<PostList>("Posts", "list", {
              organization_id: orgId,
            })
            .then((r) => r.posts)
            .catch(() => [] as PostResult[])
        )
      );
      return results;
    }),
  };
}
