"use client";

/**
 * PhotoPickerContext — one shared <MediaPicker> hosted at the PlateEditor
 * level, opened by individual photo plugins via a context.
 *
 * Why: rendering the MediaPicker Dialog *inside* each Plate void element
 * was corrupting the editor's gutter state. Base UI's focus-trap + portal
 * cleanup doesn't play nicely when the host element can be unmounted
 * (e.g. the user deletes a photo block). Hoisting the picker to a single
 * instance outside the Slate tree avoids the entire class of problems.
 */

import * as React from "react";
import { MediaPicker, type PickedMedia } from "@/components/admin/MediaPicker";

type OpenArgs = {
  title?: string;
  onSelect: (media: PickedMedia) => void;
};

type Ctx = {
  openPicker: (args: OpenArgs) => void;
};

const PhotoPickerCtx = React.createContext<Ctx | null>(null);

export function usePhotoPicker(): Ctx {
  const ctx = React.useContext(PhotoPickerCtx);
  if (!ctx) {
    throw new Error("usePhotoPicker must be used inside <PhotoPickerProvider>");
  }
  return ctx;
}

export function PhotoPickerProvider({ children }: { children: React.ReactNode }) {
  const [open, setOpen] = React.useState(false);
  const [title, setTitle] = React.useState<string | undefined>(undefined);
  // Hold onSelect in a ref so the shared Dialog doesn't re-render every
  // time a photo plugin re-mounts / re-captures its callback.
  const onSelectRef = React.useRef<((m: PickedMedia) => void) | null>(null);

  const openPicker = React.useCallback((args: OpenArgs) => {
    onSelectRef.current = args.onSelect;
    setTitle(args.title);
    setOpen(true);
  }, []);

  const handleSelect = React.useCallback((m: PickedMedia) => {
    const cb = onSelectRef.current;
    onSelectRef.current = null;
    setOpen(false);
    cb?.(m);
  }, []);

  const handleOpenChange = React.useCallback((next: boolean) => {
    if (!next) onSelectRef.current = null;
    setOpen(next);
  }, []);

  return (
    <PhotoPickerCtx.Provider value={{ openPicker }}>
      {children}
      <MediaPicker
        open={open}
        onOpenChange={handleOpenChange}
        onSelect={handleSelect}
        title={title ?? "Choose photo"}
      />
    </PhotoPickerCtx.Provider>
  );
}
