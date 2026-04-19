"use client";

/**
 * PostDetailLinkContext
 * ---------------------------------------------------------------------
 * Carries the href for the "click the title to read this post" link
 * from BroadsheetRenderer / SlotRenderer down into MTitle without
 * requiring each of the 40+ post card components to thread it
 * through props.
 *
 * Previously the card click target was a full-card absolute-positioned
 * overlay anchor. That broke text selection, clobbered hover states on
 * inner elements, and suppressed standard link underlines. The title-as-
 * link approach is cleaner: the title is a natural affordance, and
 * everything else on the card stays its normal interactive self.
 */

import { createContext, useContext } from "react";

const PostDetailLinkContext = createContext<string | null>(null);

export const PostDetailLinkProvider = PostDetailLinkContext.Provider;

export function usePostDetailLink(): string | null {
  return useContext(PostDetailLinkContext);
}
