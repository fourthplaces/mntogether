"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { useOffsetPagination } from "@/lib/hooks/useOffsetPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { Alert } from "@/components/ui/alert";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { SignalPostsQuery, RejectPostMutation } from "@/lib/graphql/posts";
import { CountiesQuery } from "@/lib/graphql/editions";
import { SeedBadgeIf } from "@/components/admin/SeedBadge";

// ─── Types ──────────────────────────────────────────────────────────────────

type PostTypeFilter =
  | "__all__"
  | "story"
  | "update"
  | "action"
  | "event"
  | "need"
  | "aid"
  | "person"
  | "business"
  | "reference";

const POST_TYPE_OPTIONS: { value: PostTypeFilter; label: string }[] = [
  { value: "__all__", label: "All Types" },
  { value: "story", label: "Story" },
  { value: "update", label: "Update" },
  { value: "action", label: "Action" },
  { value: "event", label: "Event" },
  { value: "need", label: "Need" },
  { value: "aid", label: "Aid" },
  { value: "person", label: "Person" },
  { value: "business", label: "Business" },
  { value: "reference", label: "Reference" },
];

const TYPE_BADGE_STYLES: Record<string, string> = {
  story: "bg-indigo-100 text-indigo-800",
  update: "bg-amber-100 text-amber-800",
  action: "bg-orange-100 text-orange-800",
  event: "bg-green-100 text-green-800",
  need: "bg-red-100 text-red-800",
  aid: "bg-emerald-100 text-emerald-800",
  person: "bg-purple-100 text-purple-800",
  business: "bg-blue-100 text-blue-800",
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

export default function SignalPage() {
  const router = useRouter();
  const [countyId, setCountyId] = useState("__all__");
  const [postType, setPostType] = useState<PostTypeFilter>("__all__");
  const [searchQuery, setSearchQuery] = useState("");
  const [searchInput, setSearchInput] = useState("");
  const [showRejected, setShowRejected] = useState(false);
  const [rejectingId, setRejectingId] = useState<string | null>(null);

  const pagination = useOffsetPagination({ pageSize: 25 });

  // Reset pagination when filters change
  useEffect(() => {
    pagination.reset();
  }, [countyId, postType, searchQuery, showRejected]);

  // ─── Queries ──────────────────────────────────────────────────────

  const [{ data: countiesData }] = useQuery({ query: CountiesQuery });
  const counties = countiesData?.counties || [];

  const isStatewide = countyId === "__statewide__";
  const hasCountyFilter = countyId !== "__all__" && !isStatewide;
  const [{ data, fetching, error }] = useQuery({
    query: SignalPostsQuery,
    variables: {
      status: showRejected ? "rejected" : "active",
      countyId: hasCountyFilter ? countyId : null,
      statewideOnly: isStatewide || null,
      postType: postType === "__all__" ? null : postType,
      search: searchQuery || null,
      limit: pagination.variables.first,
      offset: pagination.variables.offset,
    },
  });

  const [, rejectPost] = useMutation(RejectPostMutation);

  const posts = data?.posts?.posts || [];
  const totalCount = data?.posts?.totalCount || 0;
  const hasNextPage = data?.posts?.hasNextPage || false;
  const pageInfo = pagination.buildPageInfo(hasNextPage);

  // ─── Actions ──────────────────────────────────────────────────────

  const handleReject = async (postId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!confirm("Reject this post? It will be removed from broadsheet eligibility.")) return;
    setRejectingId(postId);
    try {
      await rejectPost(
        { id: postId, reason: "Rejected by editor from Signal view" },
        { additionalTypenames: ["Post", "PostConnection"] }
      );
    } catch (err) {
      console.error("Failed to reject post:", err);
    } finally {
      setRejectingId(null);
    }
  };

  // ─── Render ───────────────────────────────────────────────────────

  return (
    <div className="min-h-screen bg-background p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-4">
          <h1 className="text-2xl font-bold text-foreground">Signal</h1>
          <p className="text-muted-foreground text-sm mt-0.5">
            Posts ingested from Root Signal &middot; {totalCount.toLocaleString()} posts
          </p>
        </div>

        {/* Filters row */}
        <div className="flex items-center gap-3 mb-4">
          <Tabs value={showRejected ? "rejected" : "active"} onValueChange={(v) => setShowRejected(v === "rejected")}>
            <TabsList>
              <TabsTrigger value="active">Active</TabsTrigger>
              <TabsTrigger value="rejected">Rejected</TabsTrigger>
            </TabsList>
          </Tabs>

          {/* County dropdown */}
          <Select value={countyId} onValueChange={(val) => val !== null && setCountyId(val)}>
            <SelectTrigger className="w-52">
              <SelectValue placeholder="All Counties" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__all__">All Counties</SelectItem>
              <SelectItem value="__statewide__">Statewide</SelectItem>
              {counties
                .slice()
                .sort((a, b) => a.name.localeCompare(b.name))
                .map((c) => (
                  <SelectItem key={c.id} value={c.id}>
                    {c.name}
                  </SelectItem>
                ))}
            </SelectContent>
          </Select>

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
                className="text-muted-foreground hover:text-foreground"
              >
                Clear
              </Button>
            )}
          </form>
        </div>

        {/* Active filter pills */}
        {(countyId !== "__all__" || postType !== "__all__" || searchQuery) && (
          <div className="flex gap-2 flex-wrap mb-4">
            {countyId !== "__all__" && (
              <span className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-accent text-accent-foreground rounded-full text-xs font-medium">
                County: {countyId === "__statewide__" ? "Statewide" : counties.find((c) => c.id === countyId)?.name || countyId}
                <Button variant="ghost" size="icon-xs" onClick={() => setCountyId("__all__")} className="hover:text-foreground size-4"><X className="w-3 h-3" /></Button>
              </span>
            )}
            {postType !== "__all__" && (
              <span className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-accent text-accent-foreground rounded-full text-xs font-medium">
                Type: <span className="capitalize">{postType}</span>
                <Button variant="ghost" size="icon-xs" onClick={() => setPostType("__all__")} className="hover:text-foreground size-4"><X className="w-3 h-3" /></Button>
              </span>
            )}
            {searchQuery && (
              <span className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-accent text-accent-foreground rounded-full text-xs font-medium">
                Search: {searchQuery}
                <Button variant="ghost" size="icon-xs" onClick={() => { setSearchInput(""); setSearchQuery(""); }} className="hover:text-foreground size-4"><X className="w-3 h-3" /></Button>
              </span>
            )}
          </div>
        )}

        {/* Error */}
        {error && (
          <Alert variant="error" className="mb-4">
            Error: {error.message}
          </Alert>
        )}

        {/* Loading */}
        {fetching && posts.length === 0 && (
          <AdminLoader label="Loading signal posts..." />
        )}

        {/* Table */}
        {!fetching && !error && posts.length === 0 ? (
          <div className="bg-card border border-border rounded-lg p-12 text-center">
            <h3 className="text-lg font-semibold text-foreground mb-1">No posts found</h3>
            <p className="text-muted-foreground text-sm">
              {searchQuery || countyId !== "__all__" || postType !== "__all__"
                ? "Try adjusting your filters."
                : showRejected
                ? "No rejected signal posts."
                : "No active signal posts in the system."}
            </p>
          </div>
        ) : posts.length > 0 && (
          <>
            <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="pl-6">Title</TableHead>
                    <TableHead className="w-24">Type</TableHead>
                    <TableHead className="w-24">Weight</TableHead>
                    <TableHead>Source</TableHead>
                    <TableHead className="w-24">Date</TableHead>
                    <TableHead className="w-12" />
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
                        <div className="flex items-center gap-2">
                          <div className="font-medium text-foreground truncate max-w-md">
                            {post.title}
                          </div>
                          <SeedBadgeIf isSeed={post.isSeed} size="sm" />
                        </div>
                        {post.location && (
                          <div className="text-xs text-muted-foreground truncate max-w-md mt-0.5">
                            {post.location}
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
                      <TableCell className="whitespace-nowrap text-muted-foreground truncate max-w-[160px]">
                        {post.organizationName || "—"}
                      </TableCell>
                      <TableCell className="whitespace-nowrap text-muted-foreground">
                        {timeAgo(post.createdAt)}
                      </TableCell>
                      <TableCell className="whitespace-nowrap">
                        {!showRejected && (
                          <Button
                            variant="ghost"
                            size="icon-xs"
                            onClick={(e) => handleReject(post.id, e)}
                            disabled={rejectingId === post.id}
                            className="text-muted-foreground hover:text-red-600"
                            title="Reject post"
                          >
                            <X className="w-4 h-4" />
                          </Button>
                        )}
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
