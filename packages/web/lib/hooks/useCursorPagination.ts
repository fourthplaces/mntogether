"use client";

import { useState, useCallback, useMemo } from "react";

export interface PageInfo {
  hasNextPage: boolean;
  hasPreviousPage: boolean;
  startCursor?: string | null;
  endCursor?: string | null;
}

interface UseCursorPaginationOptions {
  pageSize?: number;
}

interface CursorPaginationState {
  currentPage: number;
  pageSize: number;
  after: string | null;
  // Stack of cursors for backwards navigation (previous pages' startCursors)
  cursorHistory: string[];
}

interface UseCursorPaginationResult {
  // Query variables to pass to GraphQL
  variables: {
    first: number;
    after: string | null;
  };
  // Current page number (0-indexed)
  currentPage: number;
  pageSize: number;
  // Navigation functions
  goToNextPage: (endCursor: string | null) => void;
  goToPreviousPage: () => void;
  reset: () => void;
  // For building PageInfo when query only returns hasNextPage
  buildPageInfo: (
    hasNextPage: boolean,
    startCursor?: string | null,
    endCursor?: string | null
  ) => PageInfo;
}

export function useCursorPagination(
  options: UseCursorPaginationOptions = {}
): UseCursorPaginationResult {
  const { pageSize = 20 } = options;

  const [state, setState] = useState<CursorPaginationState>({
    currentPage: 0,
    pageSize,
    after: null,
    cursorHistory: [],
  });

  const variables = useMemo(
    () => ({
      first: state.pageSize,
      after: state.after,
    }),
    [state.pageSize, state.after]
  );

  const goToNextPage = useCallback((endCursor: string | null) => {
    if (!endCursor) return;
    setState((prev) => ({
      ...prev,
      currentPage: prev.currentPage + 1,
      after: endCursor,
      // Store the current cursor so we can go back
      cursorHistory: [...prev.cursorHistory, prev.after || ""],
    }));
  }, []);

  const goToPreviousPage = useCallback(() => {
    setState((prev) => {
      if (prev.currentPage === 0) return prev;
      const newHistory = [...prev.cursorHistory];
      const previousCursor = newHistory.pop();
      return {
        ...prev,
        currentPage: prev.currentPage - 1,
        after: previousCursor === "" ? null : previousCursor || null,
        cursorHistory: newHistory,
      };
    });
  }, []);

  const reset = useCallback(() => {
    setState({
      currentPage: 0,
      pageSize,
      after: null,
      cursorHistory: [],
    });
  }, [pageSize]);

  const buildPageInfo = useCallback(
    (
      hasNextPage: boolean,
      startCursor?: string | null,
      endCursor?: string | null
    ): PageInfo => ({
      hasNextPage,
      hasPreviousPage: state.currentPage > 0,
      startCursor,
      endCursor,
    }),
    [state.currentPage]
  );

  return {
    variables,
    currentPage: state.currentPage,
    pageSize: state.pageSize,
    goToNextPage,
    goToPreviousPage,
    reset,
    buildPageInfo,
  };
}

export default useCursorPagination;
