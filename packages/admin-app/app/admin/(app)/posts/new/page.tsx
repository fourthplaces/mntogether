"use client";

import { useRouter } from "next/navigation";
import { useMutation } from "urql";
import { BackLink } from "@/components/ui/BackLink";
import { PostForm, type PostFormValues } from "@/components/admin/PostForm";
import { CreatePostMutation } from "@/lib/graphql/posts";

export default function NewPostPage() {
  const router = useRouter();
  const [{ fetching }, createPost] = useMutation(CreatePostMutation);

  async function handleSubmit(values: PostFormValues) {
    const result = await createPost(
      {
        input: {
          title: values.title,
          descriptionMarkdown: values.descriptionMarkdown,
          summary: values.summary || undefined,
          postType: values.postType,
          weight: values.weight,
          priority: values.priority,
          urgency: values.urgency || undefined,
          location: values.location || undefined,
          organizationId: values.organizationId || undefined,
        },
      },
      { additionalTypenames: ["Post", "PostConnection", "PostStats"] }
    );

    if (result.data?.createPost?.id) {
      router.push(`/admin/posts/${result.data.createPost.id}`);
    }
  }

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <BackLink href="/admin/posts">Back to Posts</BackLink>

      <div className="mt-4 mb-6">
        <h1 className="text-2xl font-semibold text-stone-900">New Post</h1>
        <p className="mt-1 text-sm text-stone-500">
          Create a new draft post. It won't be visible publicly until published.
        </p>
      </div>

      <div className="bg-white border border-stone-200 rounded-lg p-6">
        <PostForm onSubmit={handleSubmit} loading={fetching} />
      </div>
    </div>
  );
}
