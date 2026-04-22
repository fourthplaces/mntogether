"use client";

import { useState, useMemo } from "react";
import { useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { Building2, Plus, User } from "lucide-react";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import {
  OrganizationsListQuery,
  CreateOrganizationMutation,
} from "@/lib/graphql/organizations";
import { SeedBadgeIf } from "@/components/admin/SeedBadge";

function statusBadgeVariant(status: string): "success" | "warning" | "danger" | "secondary" {
  switch (status) {
    case "approved": return "success";
    case "pending_review": return "warning";
    case "rejected": return "danger";
    default: return "secondary";
  }
}

export default function OrganizationsPage() {
  return <OrganizationsContent />;
}

function OrganizationsContent() {
  const router = useRouter();
  const [showAddForm, setShowAddForm] = useState(false);
  const [addName, setAddName] = useState("");
  const [addDescription, setAddDescription] = useState("");
  const [addSourceType, setAddSourceType] = useState<"organization" | "individual">("organization");
  const [addError, setAddError] = useState<string | null>(null);

  const [{ data, fetching: isLoading, error }] = useQuery({
    query: OrganizationsListQuery,
  });

  const [{ fetching: addLoading }, createOrg] = useMutation(CreateOrganizationMutation);

  const organizations = data?.organizations || [];

  const counts = useMemo(() => ({
    all: organizations.length,
    organization: organizations.filter((o) => o.sourceType === "organization").length,
    individual: organizations.filter((o) => o.sourceType === "individual").length,
  }), [organizations]);

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!addName.trim()) return;

    setAddError(null);
    try {
      const result = await createOrg(
        {
          name: addName.trim(),
          description: addDescription.trim() || null,
          sourceType: addSourceType,
        },
        { additionalTypenames: ["Organization"] }
      );
      if (result.error) throw result.error;
      setAddName("");
      setAddDescription("");
      setAddSourceType("organization");
      setShowAddForm(false);
      if (result.data?.createOrganization?.id) {
        router.push(`/admin/organizations/${result.data.createOrganization.id}`);
      }
    } catch (err: any) {
      setAddError(err.message || "Failed to create source");
    }
  };

  const resetAddForm = () => {
    setShowAddForm(false);
    setAddName("");
    setAddDescription("");
    setAddSourceType("organization");
    setAddError(null);
  };

  if (isLoading && organizations.length === 0) {
    return <AdminLoader label="Loading sources..." />;
  }

  return (
    <div className="min-h-screen bg-background p-6">
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center justify-between mb-6">
          <h1 className="text-3xl font-bold text-foreground">Sources</h1>
          <Button variant="admin" size="sm" onClick={() => setShowAddForm(!showAddForm)}>
            <Plus className="h-4 w-4" />
            Add Source
          </Button>
        </div>

        {showAddForm && (
          <form
            onSubmit={handleAdd}
            className="rounded-lg border border-border bg-card p-4 mb-6 space-y-3"
          >
            <div className="flex items-center gap-3">
              {/* Source type toggle */}
              <Tabs value={addSourceType} onValueChange={(v) => setAddSourceType(v as "organization" | "individual")}>
                <TabsList className="shrink-0">
                  <TabsTrigger value="organization">
                    <Building2 className="h-3.5 w-3.5" />
                    Organization
                  </TabsTrigger>
                  <TabsTrigger value="individual">
                    <User className="h-3.5 w-3.5" />
                    Individual
                  </TabsTrigger>
                </TabsList>
              </Tabs>

              <Input
                value={addName}
                onChange={(e) => setAddName(e.target.value)}
                placeholder={addSourceType === "individual" ? "Person name" : "Organization name"}
                className="flex-1"
                autoFocus
                disabled={addLoading}
              />
              <Input
                value={addDescription}
                onChange={(e) => setAddDescription(e.target.value)}
                placeholder="Description (optional)"
                className="flex-1"
                disabled={addLoading}
              />
            </div>
            <div className="flex items-center gap-3">
              <Button
                type="submit"
                variant="admin"
                size="sm"
                disabled={addLoading || !addName.trim()}
                loading={addLoading}
              >
                Add
              </Button>
              <Button type="button" variant="ghost" size="sm" onClick={resetAddForm}>
                Cancel
              </Button>
              {addError && (
                <Alert variant="error" className="py-2 px-3">{addError}</Alert>
              )}
            </div>
          </form>
        )}

        {error && (
          <Alert variant="error" className="mb-6">
            Error: {error.message}
          </Alert>
        )}

        <Tabs defaultValue="all">
          <TabsList>
            <TabsTrigger value="all">
              All
              <span className="text-xs opacity-60 tabular-nums">{counts.all}</span>
            </TabsTrigger>
            <TabsTrigger value="organization">
              <Building2 className="h-3.5 w-3.5" />
              Organizations
              <span className="text-xs opacity-60 tabular-nums">{counts.organization}</span>
            </TabsTrigger>
            <TabsTrigger value="individual">
              <User className="h-3.5 w-3.5" />
              Individuals
              <span className="text-xs opacity-60 tabular-nums">{counts.individual}</span>
            </TabsTrigger>
          </TabsList>

          <TabsContent value="all">
            <SourcesTable organizations={organizations} router={router} />
          </TabsContent>
          <TabsContent value="organization">
            <SourcesTable
              organizations={organizations.filter((o) => o.sourceType === "organization")}
              router={router}
            />
          </TabsContent>
          <TabsContent value="individual">
            <SourcesTable
              organizations={organizations.filter((o) => o.sourceType === "individual")}
              router={router}
            />
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Sources table
// ---------------------------------------------------------------------------

type OrgRow = {
  id: string;
  name: string;
  description?: string | null;
  status: string;
  sourceType: string;
  createdAt: string;
  isSeed?: boolean | null;
};

function SourcesTable({
  organizations,
  router,
}: {
  organizations: readonly OrgRow[];
  router: ReturnType<typeof useRouter>;
}) {
  if (organizations.length === 0) {
    return (
      <div className="text-muted-foreground text-center py-12">
        No sources found
      </div>
    );
  }

  return (
    <Table className="table-fixed">
        <TableHeader>
          <TableRow>
            <TableHead className="pl-6">Name</TableHead>
            <TableHead className="w-36">Type</TableHead>
            <TableHead className="w-36">Status</TableHead>
            <TableHead className="w-32">Created</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {organizations.map((org) => (
            <TableRow
              key={org.id}
              onClick={() => router.push(`/admin/organizations/${org.id}`)}
              className="cursor-pointer"
            >
              <TableCell className="pl-6">
                <div className="flex items-center gap-2">
                  <div className="font-medium text-foreground truncate">{org.name}</div>
                  <SeedBadgeIf isSeed={org.isSeed} size="sm" />
                </div>
                {org.description && (
                  <div className="text-sm text-muted-foreground truncate">
                    {org.description}
                  </div>
                )}
              </TableCell>
              <TableCell>
                <Badge variant="outline" className="text-[11px]">
                  {org.sourceType === "individual" ? (
                    <><User className="h-3 w-3" /> Individual</>
                  ) : (
                    <><Building2 className="h-3 w-3" /> Organization</>
                  )}
                </Badge>
              </TableCell>
              <TableCell>
                <Badge variant={statusBadgeVariant(org.status)}>
                  {org.status.replace(/_/g, " ")}
                </Badge>
              </TableCell>
              <TableCell className="text-muted-foreground">
                {new Date(org.createdAt).toLocaleDateString()}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
  );
}
