"use client";

import { Button } from "@/components/ui/button";

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
    <div className="flex items-center justify-between bg-card border border-border rounded-lg p-4">
      <div className="text-sm text-muted-foreground">
        Showing {startItem}-{endItem} of {totalCount}
      </div>
      <div className="flex gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={onPreviousPage}
          disabled={!pageInfo.hasPreviousPage || loading}
        >
          &larr; Previous
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={onNextPage}
          disabled={!pageInfo.hasNextPage || loading}
        >
          Next &rarr;
        </Button>
      </div>
    </div>
  );
}

export default PaginationControls;
