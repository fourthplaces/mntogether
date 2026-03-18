/**
 * Detail Variants — selects which broadsheet component variant to use
 * based on post type and available data.
 *
 * The broadsheet prototype has A/B variants for most components:
 * - TitleA (standard with deck) vs TitleB (with summary)
 * - BodyA (editorial justified) vs BodyB (narrative single-column)
 * - KickerA (dot-separated tags) vs KickerB (primary + secondary pills)
 */

import type { TitleSize } from "./detail-types";

export type PostType = "story" | "notice" | "exchange" | "event" | "spotlight" | "reference";

export interface DetailVariants {
  titleVariant: "A" | "B";
  titleSize: TitleSize;
  bodyVariant: "A" | "B";
  kickerVariant: "A" | "B";
}

/**
 * Resolve which component variants to render based on post type.
 *
 * - Stories and references → TitleA (deck), BodyA (editorial), KickerA
 * - Spotlights → TitleA, BodyB (narrative), KickerB (featured primary tag)
 * - Exchanges → TitleB (summary), BodyA, KickerA
 * - Events/Notices → TitleA, BodyA, KickerA
 */
export function resolveDetailVariants(postType?: string | null): DetailVariants {
  const type = (postType || "story") as PostType;

  switch (type) {
    case "spotlight":
      return {
        titleVariant: "A",
        titleSize: "spotlight",
        bodyVariant: "B",
        kickerVariant: "B",
      };
    case "exchange":
      return {
        titleVariant: "B",
        titleSize: "exchange",
        bodyVariant: "A",
        kickerVariant: "A",
      };
    case "event":
      return {
        titleVariant: "A",
        titleSize: "event",
        bodyVariant: "A",
        kickerVariant: "A",
      };
    case "notice":
      return {
        titleVariant: "A",
        titleSize: "notice",
        bodyVariant: "A",
        kickerVariant: "A",
      };
    case "reference":
      return {
        titleVariant: "A",
        titleSize: "reference",
        bodyVariant: "A",
        kickerVariant: "A",
      };
    case "story":
    default:
      return {
        titleVariant: "A",
        titleSize: "story",
        bodyVariant: "A",
        kickerVariant: "A",
      };
  }
}
