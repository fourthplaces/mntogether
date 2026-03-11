"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { useOffsetPagination } from "@/lib/hooks/useOffsetPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  EditorialPostsQuery,
  ArchivePostMutation,
  DeletePostMutation,
} from "@/lib/graphql/posts";

// ─── Types & config ─────────────────────────────────────────────────────────

type StatusTab = "draft" | "active" | "archived";
type PostTypeFilter = "" | "story" | "notice" | "exchange" | "event" | "spotlight" | "reference";

const STATUS_TABS: { key: StatusTab; label: string }[] = [
  { key: "draft", label: "Drafts" },
  { key: "active", label: "Active" },
  { key: "archived", label: "Archived" },
];

const POST_TYPE_OPTIONS: { value: PostTypeFilter; label: string }[] = [
  { value: "", label: "All Types" },
  { value: "story", label: "Story" },
  { value: "notice", label: "Notice" },
  { value: "exchange", label: "Exchange" },
  { value: "event", label: "Event" },
  { value: "spotlight", label: "Spotlight" },
  { value: "reference", label: "Reference" },
];

const TYPE_BADGE_STYLES: Record<string, string> = {
  story: "bg-indigo-100 text-indigo-800",
  notice: "bg-amber-100 text-amber-800",
  exchange: "bg-blue-100 text-blue-800",
  event: "bg-green-100 text-green-800",
  spotlight: "bg-purple-100 text-purple-800",
  reference: "bg-stone-100 text-stone-700",
};

const WEIGHT_BADGE_STYLES: Record<string, string> = {
  heavy: "bg-red-100 text-red-700",
  medium: "bg-yellow-100 text-yellow-700",
  light: "bg-sky-100 text-sky-700",
};

// ─── Helpers ────────────────────────────────────────────────────────────────

function timeAgo(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffMs = now - then;
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  if (diffDay < 7) return `${diffDay}d ago`;
  return new Date(dateStr).toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

// ─── Component ──────────────────────────────────────────────────────────────

export default function EditorialPage() {
  const router = useRouter();
  const [statusTab, setStatusTab] = useState<StatusTab>("draft");
  const [postType, setPostType] = useState<PostTypeFilter>("");
  const [searchInput, setSearchInput] = useState("");
  const [searchQuery, setSearchQuery] = useState("");

  const pagination = useOffsetPagination({ pageSize: 20 });

  // Reset pagination when filters change
  useEffect(() => {
    pagination.reset();
  }, [statusTab, postType, searchQuery]);

  // ─── Queries ──────────────────────────────────────────────────────

  const [{ data, fetching, error }] = useQuery({
    query: EditorialPostsQuery,
    variables: {
      status: statusTab,
      postType: postType || null,
      search: searchQuery || null,
      limit: pagination.variables.first,
      offset: pagination.variables.offset,
    },
  });

  const [, archivePost] = useMutation(ArchivePostMutation);
  const [, deletePost] = useMutation(DeletePostMutation);

  const posts = data?.posts?.posts || [];
  const totalCount = data?.posts?.totalCount || 0;
  const hasNextPage = data?.posts?.hasNextPage || false;
  const pageInfo = pagination.buildPageInfo(hasNextPage);

  // ─── Actions ──────────────────────────────────────────────────────

  const handleArchive = async (postId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    await archivePost(
      { id: postId },
      { additionalTypenames: ["Post", "PostConnection"] }
    );
  };

  const handleDelete = async (postId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!confirm("Delete this post permanently?")) return;
    await deletePost(
      { id: postId },
      { additionalTypenames: ["Post", "PostConnection"] }
    );
  };

  // ─── Render ───────────────────────────────────────────────────────

  return (
    <div className="min-h-screen bg-background p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <div>
            <h1 className="text-2xl font-bold text-foreground">Editorial</h1>
            <p className="text-muted-foreground text-sm mt-0.5">
              Human-authored posts &middot; {totalCount.toLocaleString()} {statusTab} posts
            </p>
          </div>
          <a
            href="/admin/posts/new"
            className="inline-flex items-center gap-1.5 px-4 py-2 text-sm font-medium text-white bg-admin-accent hover:bg-admin-accent-hover rounded-lg transition-colors"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4">
              <path d="M10.75 4.75a.75.75 0 0 0-1.5 0v4.5h-4.5a.75.75 0 0 0 0 1.5h4.5v4.5a.75.75 0 0 0 1.5 0v-4.5h4.5a.75.75 0 0 0 0-1.5h-4.5v-4.5Z" />
            </svg>
            New Post
          </a>
        </div>

        {/* Status tabs + filters row */}
        <div className="flex items-center gap-3 mb-4">
          <Tabs value={statusTab} onValueChange={(v) => setStatusTab(v as StatusTab)}>
            <TabsList>
              {STATUS_TABS.map((tab) => (
                <TabsTrigger key={tab.key} value={tab.key}>
                  {tab.label}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>

          {/* Type dropdown */}
          <select
            value={postType}
            onChange={(e) => setPostType(e.target.value as PostTypeFilter)}
            className="h-9 px-3 border border-border rounded-lg text-sm bg-background focus:outline-none focus:ring-2 focus:ring-ring w-36"
          >
            {POST_TYPE_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>

          {/* Search */}
          <form
            onSubmit={(e) => {
              e.preventDefault();
              setSearchQuery(searchInput);
            }}
            className="flex gap-2 flex-1 min-w-[200px]"
          >
            <input
              type="text"
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
              placeholder="Search posts..."
              className="h-9 flex-1 px-3 border border-border rounded-lg text-sm bg-background focus:outline-none focus:ring-2 focus:ring-ring"
            />
            {searchQuery && (
              <button
                type="button"
                onClick={() => {
                  setSearchInput("");
                  setSearchQuery("");
                }}
                className="px-3 py-2 text-sm text-muted-foreground hover:text-foreground"
              >
                Clear
              </button>
            )}
          </form>
        </div>

        {/* Error */}
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg mb-4 text-sm">
            Error: {error.message}
          </div>
        )}

        {/* Loading */}
        {fetching && posts.length === 0 && (
          <AdminLoader label="Loading editorial posts..." />
        )}

        {/* Table or empty */}
        {!fetching && !error && posts.length === 0 ? (
          <div className="bg-card border border-border rounded-lg p-12 text-center">
            <h3 className="text-lg font-semibold text-foreground mb-1">
              {statusTab === "draft" ? "No drafts yet" : "No posts found"}
            </h3>
            <p className="text-muted-foreground text-sm">
              {statusTab === "draft"
                ? "Create a new post to get started."
                : `No ${statusTab} editorial posts${postType ? ` of type "${postType}"` : ""}.`}
            </p>
            {statusTab === "draft" && (
              <a
                href="/admin/posts/new"
                className="inline-flex items-center gap-1.5 mt-4 px-4 py-2 text-sm font-medium text-white bg-admin-accent hover:bg-admin-accent-hover rounded-lg transition-colors"
              >
                New Post
              </a>
            )}
          </div>
        ) : posts.length > 0 && (
          <>
            <div className="rounded-lg border border-border overflow-hidden bg-card">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="pl-6">Title</TableHead>
                    <TableHead className="w-24">Type</TableHead>
                    <TableHead className="w-24">Weight</TableHead>
                    <TableHead className="w-28">Updated</TableHead>
                    <TableHead className="w-20" />
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {posts.map((post) => (
                    <TableRow
                      key={post.id}
                      onClick={() => router.push(`/admin/posts/${post.id}`)}
                      className="cursor-pointer"
                    >
                      <TableCell className="pl-6">
                        <div className="font-medium text-foreground truncate max-w-lg">
                          {post.title}
                        </div>
                        {post.organizationName && (
                          <div className="text-xs text-muted-foreground mt-0.5">
                            {post.organizationName}
                          </div>
                        )}
                      </TableCell>
                      <TableCell className="whitespace-nowrap">
                        {post.postType && (
                          <span
                            className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                              TYPE_BADGE_STYLES[post.postType] || "bg-secondary text-muted-foreground"
                            }`}
                          >
                            {post.postType}
                          </span>
                        )}
                      </TableCell>
                      <TableCell className="whitespace-nowrap">
                        {post.weight && (
                          <span
                            className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                              WEIGHT_BADGE_STYLES[post.weight] || "bg-secondary text-muted-foreground"
                            }`}
                          >
                            {post.weight}
                          </span>
                        )}
                      </TableCell>
                      <TableCell className="whitespace-nowrap text-muted-foreground">
                        {timeAgo(post.createdAt)}
                      </TableCell>
                      <TableCell className="whitespace-nowrap">
                        <div className="flex gap-1">
                          {statusTab !== "archived" && (
                            <button
                              onClick={(e) => handleArchive(post.id, e)}
                              className="p-1 text-muted-foreground hover:text-amber-600 rounded"
                              title="Archive"
                            >
                              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M20.25 7.5l-.625 10.632a2.25 2.25 0 01-2.247 2.118H6.622a2.25 2.25 0 01-2.247-2.118L3.75 7.5M10 11.25h4M3.375 7.5h17.25c.621 0 1.125-.504 1.125-1.125v-1.5c0-.621-.504-1.125-1.125-1.125H3.375c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125z" />
                              </svg>
                            </button>
                          )}
                          <button
                            onClick={(e) => handleDelete(post.id, e)}
                            className="p-1 text-muted-foreground hover:text-red-600 rounded"
                            title="Delete"
                          >
                            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" />
                            </svg>
                          </button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>

            {/* Pagination */}
            <div className="mt-4">
              <PaginationControls
                pageInfo={pageInfo}
                totalCount={totalCount}
                currentPage={pagination.currentPage}
                pageSize={pagination.pageSize}
                onNextPage={pagination.goToNextPage}
                onPreviousPage={pagination.goToPreviousPage}
                loading={fetching}
              />
            </div>
          </>
        )}
      </div>
    </div>
  );
}
