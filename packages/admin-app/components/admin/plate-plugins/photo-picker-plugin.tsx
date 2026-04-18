"use client";

/**
 * PhotoPickerPlugin — one shared <MediaPicker> hosted at the editor root,
 * opened by individual photo plugins via a context.
 *
 * Shape-wise this matches the built-in DndPlugin: a plugin with
 * `render.aboveSlate` that mounts a provider once at the Plate tree's
 * root, so every block inside can call `usePhotoPicker().openPicker(...)`.
 * Registering the plugin in the editor's plugin list is how Plate wants
 * "one thing per editor, N consumers per block" wired — instead of
 * wrapping PlateEditor's JSX by hand.
 *
 * Why lift at all: rendering the MediaPicker Dialog *inside* each void
 * photo element (the previous shape) meant three separate Dialog
 * instances, three separate useState hooks, and a portal-inside-void
 * layout that Slate doesn't love. One shared instance is strictly
 * simpler regardless of whether it also papers over a rendering bug.
 */

import * as React from "react";
import { createPlatePlugin } from "platejs/react";
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
    throw new Error(
      "usePhotoPicker must be used inside an editor that includes PhotoPickerPlugin",
    );
  }
  return ctx;
}

function PhotoPickerHost({ children }: { children: React.ReactNode }) {
  const [open, setOpen] = React.useState(false);
  const [title, setTitle] = React.useState<string | undefined>(undefined);
  // Keep the callback in a ref so the host component doesn't re-render
  // every time a photo plugin re-captures its onSelect closure.
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

  const ctx = React.useMemo<Ctx>(() => ({ openPicker }), [openPicker]);

  return (
    <PhotoPickerCtx.Provider value={ctx}>
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

/**
 * Register this alongside the photo plugins in the editor's plugin list.
 * The `aboveSlate` render wraps the entire editor tree once, so every
 * block (and every slash-command-inserted new photo block) can call
 * `usePhotoPicker()`.
 */
export const PhotoPickerPlugin = createPlatePlugin({
  key: "photo_picker",
  render: {
    aboveSlate: ({ children }) => <PhotoPickerHost>{children}</PhotoPickerHost>,
  },
});
