"use client";

import * as React from "react";
import { useQuery } from "urql";
import { OrganizationsListQuery } from "@/lib/graphql/organizations";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import { EditableRow, EditorFooter, Empty } from "../EditableRow";

export function OrganizationRow({
  organizationId,
  organizationName,
  onSave,
}: {
  organizationId: string | null;
  organizationName: string | null;
  onSave: (organizationId: string | null) => Promise<unknown>;
}) {
  return (
    <EditableRow
      label="Organization"
      value={organizationName ? organizationName : <Empty>Not set</Empty>}
      mode="popover"
      editor={({ close }) => (
        <Editor
          organizationId={organizationId}
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
  organizationId,
  onSave,
  onCancel,
}: {
  organizationId: string | null;
  onSave: (val: string | null) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [{ data }] = useQuery({ query: OrganizationsListQuery });
  const organizations = data?.organizations ?? [];
  const [value, setValue] = React.useState<string>(organizationId ?? "__none__");
  const [saving, setSaving] = React.useState(false);

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave(value === "__none__" ? null : value);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div>
      <Select value={value} onValueChange={(v) => v && setValue(v)}>
        <SelectTrigger className="text-sm w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="__none__">None</SelectItem>
          {organizations.map((org) => (
            <SelectItem key={org.id} value={org.id}>{org.name}</SelectItem>
          ))}
        </SelectContent>
      </Select>
      <EditorFooter onSave={handleSave} onCancel={onCancel} saving={saving} />
    </div>
  );
}
