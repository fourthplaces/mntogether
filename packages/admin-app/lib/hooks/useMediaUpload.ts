"use client";

import { useState, useCallback } from "react";
import { useClient } from "urql";
import { PresignedUploadQuery, ConfirmUploadMutation } from "@/lib/graphql/media";

export interface UploadingFile {
  file: File;
  progress: "requesting" | "uploading" | "confirming" | "done" | "error";
  error?: string;
  mediaId?: string;
  url?: string;
}

/**
 * Hook for the 3-step presigned upload flow:
 * 1. Request presigned URL from server (via Restate)
 * 2. PUT file directly to S3/MinIO
 * 3. Confirm upload — creates the Media record in the DB
 */
export function useMediaUpload() {
  const client = useClient();
  const [uploads, setUploads] = useState<UploadingFile[]>([]);

  const updateUpload = useCallback(
    (file: File, patch: Partial<UploadingFile>) => {
      setUploads((prev) =>
        prev.map((u) => (u.file === file ? { ...u, ...patch } : u))
      );
    },
    []
  );

  const uploadFile = useCallback(
    async (file: File) => {
      const entry: UploadingFile = { file, progress: "requesting" };
      setUploads((prev) => [...prev, entry]);

      try {
        // Step 1: Get presigned URL
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
        updateUpload(file, { progress: "uploading" });

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
        updateUpload(file, { progress: "confirming" });

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
        updateUpload(file, {
          progress: "done",
          mediaId: media.id,
          url: media.url,
        });

        return media;
      } catch (err) {
        const message =
          err instanceof Error ? err.message : "Unknown upload error";
        updateUpload(file, { progress: "error", error: message });
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
