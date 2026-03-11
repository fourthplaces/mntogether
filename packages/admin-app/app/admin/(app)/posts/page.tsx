"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { useOffsetPagination } from "@/lib/hooks/useOffsetPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Archive, Plus, Trash2 } from "lucide-react";
import {
  EditorialPostsQuery,
  ArchivePostMutation,
  DeletePostMutation,
} from "@/lib/graphql/posts";

// ─── Types & config ─────────────────────────────────────────────────────────

type StatusTab = "draft" | "active" | "archived";
type PostTypeFilter = "__all__" | "story" | "notice" | "exchange" | "event" | "spotlight" | "reference";

const STATUS_TABS: { key: StatusTab; label: string }[] = [
  { key: "draft", label: "Drafts" },
  { key: "active", label: "Active" },
  { key: "archived", label: "Archived" },
];

const POST_TYPE_OPTIONS: { value: PostTypeFilter; label: string }[] = [
  { value: "__all__", label: "All Types" },
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
  reference: "bg-muted text-muted-foreground",
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
  const [postType, setPostType] = useState<PostTypeFilter>("__all__");
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
      postType: postType === "__all__" ? null : postType,
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
          <Button render={<a href="/admin/posts/new" />} variant="admin" size="sm">
              <Plus className="w-4 h-4" />
              New Post
          </Button>
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
          <Select value={postType} onValueChange={(v) => setPostType(v as PostTypeFilter)}>
            <SelectTrigger className="w-36">
              <SelectValue placeholder="All Types" />
            </SelectTrigger>
            <SelectContent>
              {POST_TYPE_OPTIONS.map((o) => (
                <SelectItem key={o.value} value={o.value}>
                  {o.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          {/* Search */}
          <form
            onSubmit={(e) => {
              e.preventDefault();
              setSearchQuery(searchInput);
            }}
            className="flex gap-2 flex-1 min-w-[200px]"
          >
            <Input
              type="text"
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
              placeholder="Search posts..."
              className="flex-1"
            />
            {searchQuery && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  setSearchInput("");
                  setSearchQuery("");
                }}
              >
                Clear
              </Button>
            )}
          </form>
        </div>

        {/* Error */}
        {error && (
          <Alert variant="error" className="mb-4">
            Error: {error.message}
          </Alert>
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
                : `No ${statusTab} editorial posts${postType !== "__all__" ? ` of type "${postType}"` : ""}.`}
            </p>
            {statusTab === "draft" && (
              <Button render={<a href="/admin/posts/new" />} variant="admin" size="sm" className="mt-4">
                New Post
              </Button>
            )}
          </div>
        ) : posts.length > 0 && (
          <>
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
                            <Tooltip>
                              <TooltipTrigger render={<Button
                                  variant="ghost"
                                  size="icon-xs"
                                  onClick={(e) => handleArchive(post.id, e)}
                                  className="text-muted-foreground hover:text-amber-600"
                                />}>
                                  <Archive className="w-4 h-4" />
                              </TooltipTrigger>
                              <TooltipContent>Archive</TooltipContent>
                            </Tooltip>
                          )}
                          <Tooltip>
                            <TooltipTrigger render={<Button
                                variant="ghost"
                                size="icon-xs"
                                onClick={(e) => handleDelete(post.id, e)}
                                className="text-muted-foreground hover:text-red-600"
                              />}>
                                <Trash2 className="w-4 h-4" />
                            </TooltipTrigger>
                            <TooltipContent>Delete</TooltipContent>
                          </Tooltip>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>

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
