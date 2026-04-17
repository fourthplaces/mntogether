"use client";

/**
 * Client-side image resize + compress for upload.
 *
 * Runs before the presigned-upload PUT in `useMediaUpload`, keeping editors
 * from ever having to think about dimensions or file size. Zero dependencies:
 * uses the browser's Canvas API + createImageBitmap, which strips EXIF and
 * normalizes orientation as a side effect.
 *
 * Defaults are newspaper-hero-sized: 1240px on the longest edge, 85% JPEG.
 * Tweakable via the options arg; the constants below are the single source.
 *
 * Behavior by file type:
 *   - image/jpeg, image/png, image/heic, image/heif → re-encoded as JPEG
 *   - image/webp, image/avif, image/gif, image/svg+xml → passed through
 *     unchanged (these already compress well, or have reasons not to touch
 *     them — animated GIFs become still frames under canvas, SVGs lose
 *     vector info)
 *   - anything else (PDF, video, etc.) → passed through unchanged
 *
 * If the source image is already smaller than the max on both axes, we
 * only re-encode (to apply JPEG quality); we never upscale.
 */

export const DEFAULT_PROCESSING = {
  /** Max pixel size on the longest edge. */
  maxSize: 1240,
  /** JPEG quality 0..1. 0.85 is the standard web sweet spot. */
  quality: 0.85,
  /** MIME type for re-encoded output. */
  outputType: "image/jpeg" as const,
};

export type ProcessingOptions = Partial<typeof DEFAULT_PROCESSING>;

const PROCESSABLE_TYPES = new Set([
  "image/jpeg",
  "image/pjpeg",
  "image/png",
  "image/heic",
  "image/heif",
]);

export interface ProcessingResult {
  file: File;
  /** true if we actually re-encoded, false if we passed through. */
  processed: boolean;
  /** If processed, stats for telemetry / editor feedback. */
  stats?: {
    originalBytes: number;
    processedBytes: number;
    originalWidth: number;
    originalHeight: number;
    processedWidth: number;
    processedHeight: number;
  };
}

/**
 * Resize + recompress an image file if it's one of the types we handle.
 * Returns the original file unchanged otherwise.
 */
export async function processImageForUpload(
  file: File,
  opts: ProcessingOptions = {},
): Promise<ProcessingResult> {
  const { maxSize, quality, outputType } = { ...DEFAULT_PROCESSING, ...opts };

  if (!PROCESSABLE_TYPES.has(file.type)) {
    return { file, processed: false };
  }

  // Decode with browsers' orientation-aware path so EXIF rotation is applied
  // (and stripped) automatically.
  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(file, { imageOrientation: "from-image" });
  } catch {
    // Older browsers or corrupt image — pass through rather than block upload.
    return { file, processed: false };
  }

  const { width: origW, height: origH } = bitmap;
  const scale = Math.min(1, maxSize / Math.max(origW, origH));
  const targetW = Math.round(origW * scale);
  const targetH = Math.round(origH * scale);

  // Step-down resizing: Canvas's single-shot drawImage with large scale
  // ratios (say, 4000→1240) produces jagged edges and moire even with
  // imageSmoothingQuality=high. Halving iteratively until we're within 2×
  // of the target and then doing the final draw gives much smoother
  // results — each halving step has plenty of source pixels to average.
  //
  // Each temporary canvas is painted with imageSmoothingQuality=high.
  const canvas = await stepDownResize(bitmap, targetW, targetH, {
    pngWhiteBackdrop: file.type === "image/png" && outputType === "image/jpeg",
  });
  bitmap.close?.();

  const blob: Blob | null = await new Promise((resolve) =>
    canvas.toBlob(resolve, outputType, quality),
  );
  if (!blob) {
    return { file, processed: false };
  }

  const newName = renameForOutput(file.name, outputType);
  const processedFile = new File([blob], newName, {
    type: outputType,
    lastModified: Date.now(),
  });

  return {
    file: processedFile,
    processed: true,
    stats: {
      originalBytes: file.size,
      processedBytes: processedFile.size,
      originalWidth: origW,
      originalHeight: origH,
      processedWidth: targetW,
      processedHeight: targetH,
    },
  };
}

/** Swap the extension to match the output MIME type. */
function renameForOutput(filename: string, outputType: string): string {
  const targetExt = outputType === "image/jpeg" ? "jpg" : outputType.split("/")[1] ?? "bin";
  const dot = filename.lastIndexOf(".");
  const base = dot > 0 ? filename.slice(0, dot) : filename;
  return `${base}.${targetExt}`;
}

/**
 * Resize an ImageBitmap down to (targetW, targetH) via successive halvings.
 *
 * When the scale ratio is large (origin is >2× the target on the longest
 * axis), halve → halve → ... until the current size is within 2× of the
 * target, then do the final draw. This drastically cleans up aliasing /
 * jagged edges that single-shot drawImage produces at large scale deltas.
 *
 * Returns a canvas ready to be encoded via toBlob.
 */
async function stepDownResize(
  source: ImageBitmap,
  targetW: number,
  targetH: number,
  opts: { pngWhiteBackdrop?: boolean } = {},
): Promise<HTMLCanvasElement> {
  let srcW = source.width;
  let srcH = source.height;

  // Draw source into a canvas we can reuse as the next iteration's source.
  let currentCanvas: HTMLCanvasElement | null = null;
  let currentSource: CanvasImageSource = source;

  // Halve as long as doing so wouldn't undershoot the target.
  while (srcW >= targetW * 2 && srcH >= targetH * 2) {
    const halfW = Math.max(1, Math.floor(srcW / 2));
    const halfH = Math.max(1, Math.floor(srcH / 2));
    const next = document.createElement("canvas");
    next.width = halfW;
    next.height = halfH;
    const nctx = next.getContext("2d");
    if (!nctx) throw new Error("2d context unavailable");
    nctx.imageSmoothingEnabled = true;
    nctx.imageSmoothingQuality = "high";
    nctx.drawImage(currentSource, 0, 0, halfW, halfH);
    srcW = halfW;
    srcH = halfH;
    currentCanvas = next;
    currentSource = next;
  }

  // Final draw to exact target size.
  const final = document.createElement("canvas");
  final.width = targetW;
  final.height = targetH;
  const fctx = final.getContext("2d");
  if (!fctx) throw new Error("2d context unavailable");
  if (opts.pngWhiteBackdrop) {
    fctx.fillStyle = "#ffffff";
    fctx.fillRect(0, 0, targetW, targetH);
  }
  fctx.imageSmoothingEnabled = true;
  fctx.imageSmoothingQuality = "high";
  fctx.drawImage(currentSource, 0, 0, targetW, targetH);

  // Help GC release intermediate canvases.
  if (currentCanvas) currentCanvas.width = currentCanvas.height = 0;

  return final;
}
