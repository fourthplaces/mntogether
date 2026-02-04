import { graphqlFetch } from "@/lib/graphql/server";
import { GET_PUBLISHED_POSTS } from "@/lib/graphql/queries";
import type { GetPublishedPostsResult } from "@/lib/types";
import { HomeClient } from "./HomeClient";

export const revalidate = 60; // Revalidate every 60 seconds

export default async function HomePage() {
  let posts: GetPublishedPostsResult["publishedPosts"] = [];
  let error: string | null = null;

  try {
    const data = await graphqlFetch<GetPublishedPostsResult>(
      GET_PUBLISHED_POSTS,
      { limit: 100 },
      { revalidate: 60 }
    );
    posts = data.publishedPosts || [];
  } catch (e) {
    error = e instanceof Error ? e.message : "Failed to load posts";
    console.error("Failed to fetch posts:", e);
  }

  return <HomeClient initialPosts={posts} error={error ?? undefined} />;
}
