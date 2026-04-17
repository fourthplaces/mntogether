"use client";

import { useState, useCallback } from "react";
import { useClient } from "urql";
import { PresignedUploadQuery, ConfirmUploadMutation } from "@/lib/graphql/media";
import { processImageForUpload } from "@/lib/image-processing";

export interface UploadingFile {
  /** The file actually being uploaded — may be a resized/recompressed
   *  version of the one the user picked. */
  file: File;
  /** The original file the editor selected, for UI display / telemetry. */
  originalFile?: File;
  progress: "processing" | "requesting" | "uploading" | "confirming" | "done" | "error";
  error?: string;
  mediaId?: string;
  url?: string;
}

/**
 * Hook for the 3-step presigned upload flow:
 * 1. Request presigned URL from server
 * 2. PUT file directly to S3/MinIO
 * 3. Confirm upload — creates the Media record in the DB
 */
export function useMediaUpload() {
  const client = useClient();
  const [uploads, setUploads] = useState<UploadingFile[]>([]);

  const updateUpload = useCallback(
    (file: File, patch: Partial<UploadingFile>) => {
      // Match on originalFile identity (stable across processing) OR current
      // file reference (for legacy entries without an originalFile).
      setUploads((prev) =>
        prev.map((u) => ((u.originalFile ?? u.file) === file ? { ...u, ...patch } : u))
      );
    },
    []
  );

  const uploadFile = useCallback(
    async (originalFile: File) => {
      const entry: UploadingFile = { file: originalFile, originalFile, progress: "processing" };
      setUploads((prev) => [...prev, entry]);

      // Step 0: Resize + recompress in-browser for images we handle. Silently
      // passes through files we don't touch (PDFs, videos, svg, webp, etc.).
      let file = originalFile;
      try {
        const result = await processImageForUpload(originalFile);
        if (result.processed) {
          file = result.file;
          // Swap the file on the tracked upload entry so progress UIs that
          // show size/filename reflect the processed version.
          updateUpload(originalFile, { file });
        }
      } catch {
        // If processing throws unexpectedly, fall through with the original.
      }

      try {
        // Step 1: Get presigned URL
        updateUpload(originalFile, { progress: "requesting" });
        const presignResult = await client.query(PresignedUploadQuery, {
          filename: file.name,
          contentType: file.type || "application/octet-stream",
          sizeBytes: file.size,
        }).toPromise();

        if (presignResult.error || !presignResult.data?.presignedUpload) {
          throw new Error(
            presignResult.error?.message || "Failed to get upload URL"
          );
        }

        const { uploadUrl, storageKey, publicUrl } =
          presignResult.data.presignedUpload;

        // Step 2: PUT file to S3
        updateUpload(originalFile, { progress: "uploading" });

        const putResponse = await fetch(uploadUrl, {
          method: "PUT",
          body: file,
          headers: {
            "Content-Type": file.type || "application/octet-stream",
          },
        });

        if (!putResponse.ok) {
          throw new Error(`Upload failed: ${putResponse.statusText}`);
        }

        // Step 3: Confirm upload
        updateUpload(originalFile, { progress: "confirming" });

        // Try to get image dimensions for images
        let width: number | undefined;
        let height: number | undefined;
        if (file.type.startsWith("image/")) {
          try {
            const dims = await getImageDimensions(file);
            width = dims.width;
            height = dims.height;
          } catch {
            // Not critical — continue without dimensions
          }
        }

        const confirmResult = await client
          .mutation(ConfirmUploadMutation, {
            storageKey,
            publicUrl,
            filename: file.name,
            contentType: file.type || "application/octet-stream",
            sizeBytes: file.size,
            width,
            height,
          })
          .toPromise();

        if (confirmResult.error || !confirmResult.data?.confirmUpload) {
          throw new Error(
            confirmResult.error?.message || "Failed to confirm upload"
          );
        }

        const media = confirmResult.data.confirmUpload;
        updateUpload(originalFile, {
          progress: "done",
          mediaId: media.id,
          url: media.url,
        });

        return media;
      } catch (err) {
        const message =
          err instanceof Error ? err.message : "Unknown upload error";
        updateUpload(originalFile, { progress: "error", error: message });
        return null;
      }
    },
    [client, updateUpload]
  );

  const uploadFiles = useCallback(
    async (files: File[]) => {
      return Promise.all(files.map(uploadFile));
    },
    [uploadFile]
  );

  const clearUploads = useCallback(() => {
    setUploads([]);
  }, []);

  return { uploads, uploadFile, uploadFiles, clearUploads };
}

/** Read image dimensions from a File using the browser's Image API. */
function getImageDimensions(
  file: File
): Promise<{ width: number; height: number }> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      resolve({ width: img.naturalWidth, height: img.naturalHeight });
      URL.revokeObjectURL(url);
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("Failed to load image"));
    };
    img.src = url;
  });
}
