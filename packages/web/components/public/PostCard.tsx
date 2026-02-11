import Link from "next/link";
import type { PublicPostResult, PostTypeOption } from "@/lib/restate/types";

function formatCategory(value: string): string {
  return value
    .split("-")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

export function PostCard({ post }: { post: PublicPostResult; postTypes?: PostTypeOption[] }) {
  const postTypeTag = post.tags.find((t) => t.kind === "post_type");
  const displayTags = post.tags.filter((t) => t.kind !== "post_type");

  return (
    <Link
      href={`/posts/${post.id}`}
      className="bg-white p-6 rounded-lg border border-[#E8DED2] hover:shadow-md transition-shadow block"
    >
      {post.has_urgent_notes && (
        <span className="px-2.5 py-0.5 text-xs font-medium rounded-full bg-red-100 text-red-800 mb-2 inline-block">
          Urgent
        </span>
      )}
      <h3 className="text-xl font-bold text-[#3D3D3D] mb-1">{post.title}</h3>
      {post.location && (
        <p className="text-sm text-[#7D7D7D] mb-1">{post.location}</p>
      )}
      <p className="text-[#5D5D5D] text-[0.95rem] leading-relaxed mb-3">
        {post.summary || post.description}
      </p>
      <div className="flex flex-wrap gap-2">
        {postTypeTag && (
          <span
            title={`${postTypeTag.kind}: ${postTypeTag.value}`}
            className={`px-3 py-1 rounded-full text-xs font-medium ${!postTypeTag.color ? "bg-[#F5F1E8] text-[#5D5D5D]" : ""}`}
            style={postTypeTag.color ? { backgroundColor: postTypeTag.color + "20", color: postTypeTag.color } : undefined}
          >
            {postTypeTag.display_name || formatCategory(postTypeTag.value)}
          </span>
        )}
        {displayTags.map((tag) => (
          <span
            key={tag.value}
            title={`${tag.kind}: ${tag.value}`}
            className={`px-3 py-1 rounded-full text-xs font-medium ${!tag.color ? "bg-[#F5F1E8] text-[#5D5D5D]" : ""}`}
            style={tag.color ? { backgroundColor: tag.color + "20", color: tag.color } : undefined}
          >
            {tag.display_name || formatCategory(tag.value)}
          </span>
        ))}
      </div>
    </Link>
  );
}

export function PostCardSkeleton() {
  return (
    <div className="bg-white p-6 rounded-lg border border-[#E8DED2] animate-pulse">
      <div className="h-6 w-3/4 bg-gray-200 rounded mb-2" />
      <div className="h-4 w-1/3 bg-gray-200 rounded mb-2" />
      <div className="h-4 w-full bg-gray-200 rounded mb-1" />
      <div className="h-4 w-5/6 bg-gray-200 rounded mb-3" />
      <div className="h-6 w-20 bg-gray-200 rounded-full" />
    </div>
  );
}
