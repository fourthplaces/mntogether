"use client";

import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationPrevious,
  PaginationNext,
} from "@/components/ui/pagination";

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

  const prevDisabled = !pageInfo.hasPreviousPage || loading;
  const nextDisabled = !pageInfo.hasNextPage || loading;

  return (
    <div className="flex items-center justify-between bg-card border border-border rounded-lg p-4">
      <div className="text-sm text-muted-foreground">
        Showing {startItem}–{endItem} of {totalCount}
      </div>
      <Pagination className="mx-0 w-auto">
        <PaginationContent>
          <PaginationItem>
            <PaginationPrevious
              onClick={prevDisabled ? undefined : onPreviousPage}
              className={prevDisabled ? "pointer-events-none opacity-50" : "cursor-pointer"}
              aria-disabled={prevDisabled}
              tabIndex={prevDisabled ? -1 : undefined}
            />
          </PaginationItem>
          <PaginationItem>
            <PaginationNext
              onClick={nextDisabled ? undefined : onNextPage}
              className={nextDisabled ? "pointer-events-none opacity-50" : "cursor-pointer"}
              aria-disabled={nextDisabled}
              tabIndex={nextDisabled ? -1 : undefined}
            />
          </PaginationItem>
        </PaginationContent>
      </Pagination>
    </div>
  );
}

export default PaginationControls;
