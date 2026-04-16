"use client";

import * as React from "react";
import { Input } from "@/components/ui/input";
import { EditableRow, EditorFooter, Empty } from "../EditableRow";

export function LocationRow({
  location,
  zipCode,
  latitude,
  longitude,
  onSave,
}: {
  location: string | null;
  zipCode: string | null;
  latitude?: number | null;
  longitude?: number | null;
  onSave: (input: { location: string | null; zipCode: string | null }) => Promise<unknown>;
}) {
  const display = location || zipCode ? (
    <span>
      {location}
      {location && zipCode && <span className="text-muted-foreground"> · </span>}
      {zipCode && <span className="text-muted-foreground">{zipCode}</span>}
      {latitude != null && longitude != null && (
        <span className="text-muted-foreground/60 text-xs font-mono ml-2">
          ({latitude.toFixed(3)}, {longitude.toFixed(3)})
        </span>
      )}
    </span>
  ) : <Empty>Not set</Empty>;

  return (
    <EditableRow
      label="Location"
      value={display}
      mode="popover"
      editor={({ close }) => (
        <Editor
          location={location ?? ""}
          zipCode={zipCode ?? ""}
          onSave={async (val) => {
            await onSave(val);
            close();
          }}
          onCancel={close}
        />
      )}
    />
  );
}

function Editor({
  location,
  zipCode,
  onSave,
  onCancel,
}: {
  location: string;
  zipCode: string;
  onSave: (input: { location: string | null; zipCode: string | null }) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [loc, setLoc] = React.useState(location);
  const [zip, setZip] = React.useState(zipCode);
  const [saving, setSaving] = React.useState(false);

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave({ location: loc.trim() || null, zipCode: zip.trim() || null });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-2">
      <label className="block text-xs text-muted-foreground">Location</label>
      <Input
        value={loc}
        onChange={(e) => setLoc(e.target.value)}
        placeholder="e.g. Mountain Lake, MN"
        className="text-sm"
      />
      <label className="block text-xs text-muted-foreground">Zip Code</label>
      <Input
        value={zip}
        onChange={(e) => setZip(e.target.value)}
        placeholder="e.g. 56159"
        className="text-sm"
      />
      <EditorFooter onSave={handleSave} onCancel={onCancel} saving={saving} />
    </div>
  );
}
