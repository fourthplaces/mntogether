"use client";

import * as React from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import { Plus, X, Phone, Mail, Globe, MapPin } from "lucide-react";
import { EditableRow, Empty } from "../EditableRow";

type Contact = { id: string; contactType: string; contactValue: string; contactLabel?: string | null };

const CONTACT_TYPES = [
  { value: "phone", label: "Phone", icon: Phone },
  { value: "email", label: "Email", icon: Mail },
  { value: "website", label: "Website", icon: Globe },
  { value: "address", label: "Address", icon: MapPin },
  { value: "booking_url", label: "Booking URL", icon: Globe },
  { value: "social", label: "Social", icon: Globe },
];

function contactIcon(type: string) {
  const t = CONTACT_TYPES.find((c) => c.value === type);
  const Icon = t?.icon ?? Globe;
  return <Icon className="w-3.5 h-3.5 text-muted-foreground" />;
}

function renderValue(c: Contact) {
  const href =
    c.contactType === "email"
      ? `mailto:${c.contactValue}`
      : c.contactType === "phone"
        ? `tel:${c.contactValue}`
        : c.contactType === "website" || c.contactType === "booking_url" || c.contactType === "social"
          ? c.contactValue.startsWith("http") ? c.contactValue : `https://${c.contactValue}`
          : null;
  if (href) {
    return (
      <a
        href={href}
        target="_blank"
        rel="noopener noreferrer"
        className="text-link hover:text-link-hover truncate"
        onClick={(e) => e.stopPropagation()}
      >
        {c.contactValue}
      </a>
    );
  }
  return c.contactValue;
}

export function ContactsRow({
  contacts,
  onAdd,
  onRemove,
}: {
  contacts: Contact[];
  onAdd: (input: { contactType: string; contactValue: string; contactLabel?: string | null }) => Promise<unknown>;
  onRemove: (contactId: string) => Promise<unknown>;
}) {
  const display = contacts.length > 0 ? (
    <div className="flex flex-col gap-1 min-w-0">
      {contacts.slice(0, 3).map((c) => (
        <div key={c.id} className="flex items-center gap-1.5 text-sm min-w-0">
          {contactIcon(c.contactType)}
          <span className="truncate">{renderValue(c)}</span>
        </div>
      ))}
      {contacts.length > 3 && (
        <div className="text-xs text-muted-foreground">+ {contacts.length - 3} more</div>
      )}
    </div>
  ) : <Empty>No contacts</Empty>;

  return (
    <EditableRow
      label="Contact"
      value={display}
      mode="sheet"
      sheetTitle="Contacts"
      editor={({ close }) => (
        <ContactsEditor
          contacts={contacts}
          onAdd={onAdd}
          onRemove={onRemove}
          onDone={close}
        />
      )}
    />
  );
}

function ContactsEditor({
  contacts,
  onAdd,
  onRemove,
  onDone,
}: {
  contacts: Contact[];
  onAdd: (input: { contactType: string; contactValue: string; contactLabel?: string | null }) => Promise<unknown>;
  onRemove: (contactId: string) => Promise<unknown>;
  onDone: () => void;
}) {
  const [newType, setNewType] = React.useState("phone");
  const [newValue, setNewValue] = React.useState("");
  const [newLabel, setNewLabel] = React.useState("");
  const [busy, setBusy] = React.useState(false);

  const handleAdd = async () => {
    if (!newValue.trim()) return;
    setBusy(true);
    try {
      await onAdd({
        contactType: newType,
        contactValue: newValue.trim(),
        contactLabel: newLabel.trim() || null,
      });
      setNewValue("");
      setNewLabel("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-4 py-2">
      {/* List existing */}
      <div className="space-y-1.5">
        {contacts.length === 0 ? (
          <p className="text-sm text-muted-foreground italic">No contacts yet.</p>
        ) : contacts.map((c) => (
          <div key={c.id} className="flex items-center gap-2 group py-1.5">
            <Badge variant="secondary" className="text-[10px] uppercase flex-shrink-0 w-20 justify-center">
              {c.contactType.replace("_", " ")}
            </Badge>
            <span className="text-sm text-foreground break-all flex-1 min-w-0 truncate">
              {renderValue(c)}
            </span>
            <Button
              variant="ghost"
              size="icon"
              onClick={() => onRemove(c.id)}
              className="h-6 w-6 text-muted-foreground hover:text-danger-text"
              title="Remove"
            >
              <X className="h-3.5 w-3.5" />
            </Button>
          </div>
        ))}
      </div>

      {/* Add new */}
      <div className="border-t border-border pt-3">
        <div className="text-xs uppercase tracking-wide text-muted-foreground mb-2">Add contact</div>
        <div className="space-y-2">
          <Select value={newType} onValueChange={(v) => v && setNewType(v)}>
            <SelectTrigger className="text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {CONTACT_TYPES.map((t) => (
                <SelectItem key={t.value} value={t.value}>{t.label}</SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Input
            value={newValue}
            onChange={(e) => setNewValue(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") handleAdd(); }}
            placeholder="Value…"
            className="text-sm"
          />
          <Input
            value={newLabel}
            onChange={(e) => setNewLabel(e.target.value)}
            placeholder="Optional label (e.g. 'After hours')"
            className="text-sm"
          />
          <Button onClick={handleAdd} disabled={busy || !newValue.trim()} size="sm" className="w-full">
            <Plus className="h-4 w-4 mr-1" />
            Add contact
          </Button>
        </div>
      </div>

      <div className="flex justify-end pt-2">
        <Button variant="outline" size="sm" onClick={onDone}>Done</Button>
      </div>
    </div>
  );
}
