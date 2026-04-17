"use client";

import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { useCallback, useMemo, useState } from "react";

import { AdminLoader } from "@/components/admin/AdminLoader";
import {
  PostDetailFullQuery,
  UpdatePostMutation,
  ApprovePostMutation,
  RejectPostMutation,
  ArchivePostMutation,
  DeletePostMutation,
  ReactivatePostMutation,
  AddPostTagMutation,
  RemovePostTagMutation,
  AddPostContactMutation,
  RemovePostContactMutation,
  AddPostScheduleMutation,
  DeletePostScheduleMutation,
  UpsertPostLinkMutation,
  UpsertPostSourceAttrMutation,
  UpsertPostDatetimeMutation,
  UpsertPostStatusMutation,
  UpsertPostMediaMutation,
  UpsertPostPersonMutation,
  UpsertPostItemsMutation,
} from "@/lib/graphql/posts";
import { TagKindsQuery, TagsQuery } from "@/lib/graphql/tags";

import { PostDetailHero } from "@/components/admin/post-detail/PostDetailHero";
import { PostDetailLeft } from "@/components/admin/post-detail/PostDetailLeft";
import { PostDetailRight } from "@/components/admin/post-detail/PostDetailRight";

// ---------------------------------------------------------------------------
// Main page — thin composition of hero + left + right
// ---------------------------------------------------------------------------

export default function PostDetailPage() {
  const params = useParams();
  const router = useRouter();
  const postId = params.id as string;
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [isTagsBusy, setIsTagsBusy] = useState(false);

  // GraphQL
  const [{ data: postData, fetching: isLoading, error }] = useQuery({
    query: PostDetailFullQuery,
    variables: { id: postId },
  });
  const post = postData?.post;
  const notes = postData?.entityNotes ?? [];

  const [, updatePost] = useMutation(UpdatePostMutation);
  const [, approvePost] = useMutation(ApprovePostMutation);
  const [, rejectPost] = useMutation(RejectPostMutation);
  const [, archivePost] = useMutation(ArchivePostMutation);
  const [, deletePost] = useMutation(DeletePostMutation);
  const [, reactivatePost] = useMutation(ReactivatePostMutation);
  const [, addPostTag] = useMutation(AddPostTagMutation);
  const [, removePostTag] = useMutation(RemovePostTagMutation);
  const [, addPostContact] = useMutation(AddPostContactMutation);
  const [, removePostContact] = useMutation(RemovePostContactMutation);
  const [, addPostSchedule] = useMutation(AddPostScheduleMutation);
  const [, deletePostSchedule] = useMutation(DeletePostScheduleMutation);
  const [, upsertLink] = useMutation(UpsertPostLinkMutation);
  const [, upsertSourceAttr] = useMutation(UpsertPostSourceAttrMutation);
  const [, upsertDatetime] = useMutation(UpsertPostDatetimeMutation);
  const [, upsertPostStatus] = useMutation(UpsertPostStatusMutation);
  const [, upsertMedia] = useMutation(UpsertPostMediaMutation);
  const [, upsertPerson] = useMutation(UpsertPostPersonMutation);
  const [, upsertItems] = useMutation(UpsertPostItemsMutation);

  // Tag data
  const [{ data: kindsData }] = useQuery({ query: TagKindsQuery });
  const [{ data: allTagsData }] = useQuery({ query: TagsQuery });

  const postTagKinds = useMemo(
    () =>
      (kindsData?.tagKinds || [])
        .filter((k) => k.allowedResourceTypes.includes("post"))
        .map((k) => ({ slug: k.slug, displayName: k.displayName, locked: k.locked ?? false })),
    [kindsData]
  );

  const allTagsByKind = useMemo(() => {
    const map: Record<string, Array<{ id: string; value: string; displayName?: string | null; color?: string | null }>> = {};
    for (const tag of allTagsData?.tags || []) {
      if (!map[tag.kind]) map[tag.kind] = [];
      map[tag.kind].push(tag);
    }
    return map;
  }, [allTagsData]);

  const mutationContext = useMemo(
    () => ({ additionalTypenames: ["Post", "PostConnection", "PostStats"] }),
    []
  );

  // ---------------------------------------------------------------------------
  // Mutation wrappers
  // ---------------------------------------------------------------------------

  const inlineUpdate = useCallback(
    async (input: Record<string, unknown>) => {
      return await updatePost({ id: postId, input }, mutationContext);
    },
    [postId, updatePost, mutationContext]
  );

  const withAction = (name: string, fn: () => Promise<unknown>) => async () => {
    setActionInProgress(name);
    try { await fn(); } catch (err) { console.error(`Failed to ${name}:`, err); } finally { setActionInProgress(null); }
  };

  const handleArchive = () => withAction("archive", () => archivePost({ id: postId }, mutationContext))();
  const handleReactivate = () => withAction("reactivate", () => reactivatePost({ id: postId }, mutationContext))();
  const handleApprove = () => withAction("approve", () => approvePost({ id: postId }, mutationContext))();
  const handleReject = () => withAction("reject", () => rejectPost({ id: postId, reason: "Rejected by admin" }, mutationContext))();
  const handleDelete = withAction("delete", async () => {
    await deletePost({ id: postId }, mutationContext);
    router.push("/admin/posts");
  });

  const onStatusChange = (newStatus: string) => {
    if (newStatus === "active") handleApprove();
    else if (newStatus === "rejected") handleReject();
    else if (newStatus === "archived") handleArchive();
    else if (newStatus === "draft" || newStatus === "pending_approval") handleReactivate();
  };

  // Tag handlers
  const handleAddTags = useCallback(
    async (kindSlug: string, newTags: Array<{ value: string; displayName: string }>) => {
      setIsTagsBusy(true);
      try {
        await Promise.all(
          newTags.map((t) =>
            addPostTag({ postId, tagKind: kindSlug, tagValue: t.value, displayName: t.displayName }, mutationContext)
          )
        );
      } catch (err) {
        console.error("Failed to add tags:", err);
      } finally {
        setIsTagsBusy(false);
      }
    },
    [postId, addPostTag, mutationContext]
  );

  const handleRemoveTag = useCallback(
    async (tagId: string) => {
      setIsTagsBusy(true);
      try {
        await removePostTag({ postId, tagId }, mutationContext);
      } catch (err) {
        console.error("Failed to remove tag:", err);
      } finally {
        setIsTagsBusy(false);
      }
    },
    [postId, removePostTag, mutationContext]
  );

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  if (isLoading) return <AdminLoader label="Loading post..." />;

  if (error) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="max-w-4xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-danger-text mb-4">Error Loading Post</h1>
          <p className="text-muted-foreground mb-4">{error.message}</p>
          <Link href="/admin/posts" className="text-link hover:text-link-hover">Back to Posts</Link>
        </div>
      </div>
    );
  }

  if (!post) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="max-w-4xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-foreground mb-4">Post Not Found</h1>
          <Link href="/admin/posts" className="text-link hover:text-link-hover">Back to Posts</Link>
        </div>
      </div>
    );
  }

  const actions = {
    inlineUpdate,
    addContact: async (input: { contactType: string; contactValue: string; contactLabel?: string | null }) =>
      addPostContact({ postId, ...input }, mutationContext),
    removeContact: async (contactId: string) =>
      removePostContact({ postId, contactId }, mutationContext),
    addSchedule: async (input: { dayOfWeek: number; opensAt: string; closesAt: string }) =>
      addPostSchedule({ postId, input: { ...input, timezone: "America/Chicago" } }, mutationContext),
    deleteSchedule: async (scheduleId: string) =>
      deletePostSchedule({ postId, scheduleId }, mutationContext),
    upsertLink: async (input: { label: string | null; url: string | null; deadline: string | null }) =>
      upsertLink({ postId, ...input }, mutationContext),
    upsertDatetime: async (input: { start: string | null; end: string | null; cost: string | null; recurring: boolean }) =>
      upsertDatetime({ postId, startAt: input.start, endAt: input.end, cost: input.cost, recurring: input.recurring }, mutationContext),
    upsertPerson: async (input: { name: string | null; role: string | null; bio: string | null; photoUrl: string | null; quote: string | null; photoMediaId: string | null }) =>
      upsertPerson({ postId, ...input }, mutationContext),
    upsertItems: async (items: Array<{ name: string; detail?: string | null }>) =>
      upsertItems({ postId, items: items.map(i => ({ name: i.name, detail: i.detail ?? null })) }, mutationContext),
    upsertSourceAttr: async (input: { sourceName: string | null; attribution: string | null }) =>
      upsertSourceAttr({ postId, ...input }, mutationContext),
    upsertStatus: async (input: { state: string | null; verified: string | null }) =>
      upsertPostStatus({ postId, ...input }, mutationContext),
  };

  const onSaveMedia = async (input: { imageUrl: string | null; caption: string | null; credit: string | null; mediaId: string | null }) =>
    upsertMedia({ postId, ...input }, mutationContext);

  return (
    <div className="min-h-screen bg-background">
      {/* Full-width hero header */}
      <PostDetailHero
        post={post}
        actionInProgress={actionInProgress}
        onStatusChange={onStatusChange}
        onDelete={handleDelete}
        inlineUpdate={inlineUpdate}
      />

      {/* Two-column body */}
      <main className="max-w-7xl mx-auto px-4 py-6">
        <div className="grid grid-cols-1 lg:grid-cols-[6fr_4fr] gap-8">
          <PostDetailLeft
            post={post}
            postId={postId}
            onSaveMedia={onSaveMedia}
          />
          <PostDetailRight
            post={post}
            notes={notes}
            actions={actions}
            tagsData={{
              applicableKinds: postTagKinds,
              allTagsByKind,
              onAddTags: handleAddTags,
              onRemoveTag: handleRemoveTag,
              disabled: isTagsBusy,
            }}
          />
        </div>
      </main>
    </div>
  );
}
