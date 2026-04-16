"use client";

import * as React from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { EditableRow, EditorFooter, Empty } from "../EditableRow";

type Person = {
  name?: string | null;
  role?: string | null;
  bio?: string | null;
  photoUrl?: string | null;
  quote?: string | null;
} | null;

export function PersonRow({
  person,
  onSave,
}: {
  person: Person;
  onSave: (input: {
    name: string | null;
    role: string | null;
    bio: string | null;
    photoUrl: string | null;
    quote: string | null;
  }) => Promise<unknown>;
}) {
  const hasContent = !!(person?.name || person?.role || person?.bio);
  const display = hasContent ? (
    <div className="flex flex-col min-w-0">
      <span className="font-medium truncate">{person?.name}</span>
      {person?.role && <span className="text-muted-foreground text-xs truncate">{person.role}</span>}
    </div>
  ) : <Empty>No person</Empty>;

  return (
    <EditableRow
      label="Person"
      value={display}
      mode="sheet"
      sheetTitle="Person profile"
      editor={({ close }) => (
        <Editor
          person={person}
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
  person,
  onSave,
  onCancel,
}: {
  person: Person;
  onSave: (input: {
    name: string | null;
    role: string | null;
    bio: string | null;
    photoUrl: string | null;
    quote: string | null;
  }) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [name, setName] = React.useState(person?.name || "");
  const [role, setRole] = React.useState(person?.role || "");
  const [bio, setBio] = React.useState(person?.bio || "");
  const [photoUrl, setPhotoUrl] = React.useState(person?.photoUrl || "");
  const [quote, setQuote] = React.useState(person?.quote || "");
  const [saving, setSaving] = React.useState(false);

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave({
        name: name.trim() || null,
        role: role.trim() || null,
        bio: bio.trim() || null,
        photoUrl: photoUrl.trim() || null,
        quote: quote.trim() || null,
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-2 py-2">
      <label className="block text-xs text-muted-foreground">Name</label>
      <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="Full name" className="text-sm" />

      <label className="block text-xs text-muted-foreground">Role / title</label>
      <Input value={role} onChange={(e) => setRole(e.target.value)} placeholder="e.g. Director" className="text-sm" />

      <label className="block text-xs text-muted-foreground">Bio</label>
      <textarea
        value={bio}
        onChange={(e) => setBio(e.target.value)}
        placeholder="Short bio…"
        rows={4}
        className="w-full rounded border border-border bg-card px-2 py-1.5 text-sm"
      />

      <label className="block text-xs text-muted-foreground">Photo URL</label>
      <Input value={photoUrl} onChange={(e) => setPhotoUrl(e.target.value)} placeholder="https://…" className="text-sm" />
      {photoUrl && (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={photoUrl} alt="Person" className="h-24 w-24 rounded object-cover border border-border" />
      )}

      <label className="block text-xs text-muted-foreground">Quote (optional)</label>
      <textarea
        value={quote}
        onChange={(e) => setQuote(e.target.value)}
        placeholder="Pull quote in their voice…"
        rows={3}
        className="w-full rounded border border-border bg-card px-2 py-1.5 text-sm"
      />

      <EditorFooter onSave={handleSave} onCancel={onCancel} saving={saving} />
    </div>
  );
}
