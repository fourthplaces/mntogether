"use client";

import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { useMutation } from "urql";
import { CreatePostMutation } from "@/lib/graphql/posts";

const mutationContext = {
  additionalTypenames: ["Post", "PostConnection", "PostStats"],
};

export default function NewPostPage() {
  const router = useRouter();
  const [, createPost] = useMutation(CreatePostMutation);
  const started = useRef(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    (async () => {
      const result = await createPost(
        {
          input: {
            title: "Untitled",
            bodyRaw: "",
            postType: "story",
            weight: "medium",
            priority: 0,
          },
        },
        mutationContext
      );
      const id = result.data?.createPost?.id;
      if (id) {
        router.replace(`/admin/posts/${id}/edit`);
      } else {
        setError(result.error?.message ?? "Could not create draft");
      }
    })();
  }, [createPost, router]);

  return (
    <div className="flex h-full items-center justify-center p-8 text-sm text-muted-foreground">
      {error ? (
        <div className="space-y-2 text-center">
          <p className="text-destructive">Couldn&rsquo;t start a new post: {error}</p>
          <button
            className="underline"
            onClick={() => {
              started.current = false;
              setError(null);
              router.refresh();
            }}
          >
            Try again
          </button>
        </div>
      ) : (
        "Creating draft\u2026"
      )}
    </div>
  );
}
