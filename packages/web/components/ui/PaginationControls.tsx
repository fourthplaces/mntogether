"use client";

interface PageInfo {
  hasNextPage: boolean;
  hasPreviousPage: boolean;
}

interface PaginationControlsProps {
  pageInfo: PageInfo;
  totalCount: number;
  currentPage: number;
  pageSize: number;
  onNextPage: () => void;
  onPreviousPage: () => void;
  loading?: boolean;
}

export function PaginationControls({
  pageInfo,
  totalCount,
  currentPage,
  pageSize,
  onNextPage,
  onPreviousPage,
  loading = false,
}: PaginationControlsProps) {
  const startItem = currentPage * pageSize + 1;
  const endItem = Math.min((currentPage + 1) * pageSize, totalCount);

  if (totalCount === 0) {
    return null;
  }

  return (
    <div className="flex items-center justify-between bg-surface-raised border border-border rounded-lg p-4">
      <div className="text-sm text-text-secondary">
        Showing {startItem}-{endItem} of {totalCount}
      </div>
      <div className="flex gap-2">
        <button
          onClick={onPreviousPage}
          disabled={!pageInfo.hasPreviousPage || loading}
          className="px-4 py-2 bg-surface-muted text-text-secondary rounded hover:bg-surface disabled:opacity-50 disabled:cursor-not-allowed"
        >
          &larr; Previous
        </button>
        <button
          onClick={onNextPage}
          disabled={!pageInfo.hasNextPage || loading}
          className="px-4 py-2 bg-surface-muted text-text-secondary rounded hover:bg-surface disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Next &rarr;
        </button>
      </div>
    </div>
  );
}

export default PaginationControls;
