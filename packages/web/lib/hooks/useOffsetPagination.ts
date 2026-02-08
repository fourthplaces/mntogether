"use client";

import { useState, useCallback, useMemo } from "react";

interface UseOffsetPaginationOptions {
  pageSize?: number;
}

export interface OffsetPageInfo {
  hasNextPage: boolean;
  hasPreviousPage: boolean;
}

interface UseOffsetPaginationResult {
  variables: {
    first: number;
    offset: number;
  };
  currentPage: number;
  pageSize: number;
  goToNextPage: () => void;
  goToPreviousPage: () => void;
  reset: () => void;
  buildPageInfo: (hasNextPage: boolean) => OffsetPageInfo;
}

export function useOffsetPagination(
  options: UseOffsetPaginationOptions = {}
): UseOffsetPaginationResult {
  const { pageSize = 20 } = options;
  const [currentPage, setCurrentPage] = useState(0);

  const variables = useMemo(
    () => ({
      first: pageSize,
      offset: currentPage * pageSize,
    }),
    [pageSize, currentPage]
  );

  const goToNextPage = useCallback(() => {
    setCurrentPage((p) => p + 1);
  }, []);

  const goToPreviousPage = useCallback(() => {
    setCurrentPage((p) => Math.max(0, p - 1));
  }, []);

  const reset = useCallback(() => {
    setCurrentPage(0);
  }, []);

  const buildPageInfo = useCallback(
    (hasNextPage: boolean): OffsetPageInfo => ({
      hasNextPage,
      hasPreviousPage: currentPage > 0,
    }),
    [currentPage]
  );

  return {
    variables,
    currentPage,
    pageSize,
    goToNextPage,
    goToPreviousPage,
    reset,
    buildPageInfo,
  };
}
