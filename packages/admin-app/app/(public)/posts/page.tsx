import { Suspense } from "react";
import { PostsClient } from "./PostsClient";

export default function PostsPage() {
  return (
    <Suspense>
      <PostsClient />
    </Suspense>
  );
}
