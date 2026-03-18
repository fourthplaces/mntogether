"use client";

import { useState } from "react";
import { useQuery, useMutation } from "urql";
import { useRouter } from "next/navigation";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { WidgetListQuery, CreateWidgetMutation } from "@/lib/graphql/widgets";
import { CountiesQuery } from "@/lib/graphql/editions";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import { Plus } from "lucide-react";

// ─── Widget type definitions ────────────────────────────────────────────────

const WIDGET_TYPES = [
  { type: "number", label: "Number", description: "Big number with title/body — variants: stat card or colored tile", authoring: "human" },
  { type: "pull_quote", label: "Pull Quote", description: "Editorial quotation", authoring: "human" },
  { type: "resource_bar", label: "Resource Bar", description: "Horizontal strip of resources", authoring: "human" },
  { type: "weather", label: "Weather", description: "Automated weather display", authoring: "automated" },
  { type: "section_sep", label: "Section Sep", description: "Section divider", authoring: "layout" },
] as const;

const AUTHORING_COLORS: Record<string, string> = {
  human: "bg-emerald-100 text-emerald-800",
  automated: "bg-blue-100 text-blue-800",
  layout: "bg-gray-100 text-gray-700",
};

const TYPE_COLORS: Record<string, string> = {
  number: "bg-amber-100 text-amber-800",
  pull_quote: "bg-rose-100 text-rose-800",
  resource_bar: "bg-teal-100 text-teal-800",
  weather: "bg-sky-100 text-sky-800",
  section_sep: "bg-gray-100 text-gray-700",
  // Backward compat for old type names in DB
  stat_card: "bg-amber-100 text-amber-800",
  number_block: "bg-violet-100 text-violet-800",
};

function defaultData(widgetType: string): Record<string, unknown> {
  switch (widgetType) {
    case "number":
    case "stat_card":
    case "number_block":
      return { number: "", title: "", label: "", body: "", detail: "", color: "teal" };
    case "pull_quote":
      return { quote: "", attribution: "" };
    case "resource_bar":
      return { label: "", items: [{ number: "", text: "" }] };
    case "weather":
      return { variant: "forecast", config: { location: "" } };
    case "section_sep":
      return { title: "", sub: "" };
    default:
      return {};
  }
}

// ─── Page ───────────────────────────────────────────────────────────────────

export default function WidgetsPage() {
  const router = useRouter();
  const [typeFilter, setTypeFilter] = useState<string>("all");
  const [countyFilter, setCountyFilter] = useState<string>("all");
  const [searchInput, setSearchInput] = useState("");
  const [search, setSearch] = useState<string | undefined>(undefined);
  const [showNewDialog, setShowNewDialog] = useState(false);

  const [{ data: countiesData }] = useQuery({ query: CountiesQuery });

  const [{ data, fetching }] = useQuery({
    query: WidgetListQuery,
    variables: {
      widgetType: typeFilter === "all" ? undefined : typeFilter,
      countyId: countyFilter === "all" ? undefined : countyFilter,
      search: search || undefined,
      limit: 100,
    },
  });

  const [, createWidget] = useMutation(CreateWidgetMutation);

  const handleCreate = async (widgetType: string) => {
    const typeDef = WIDGET_TYPES.find((t) => t.type === widgetType);
    const result = await createWidget(
      {
        widgetType,
        data: JSON.stringify(defaultData(widgetType)),
        authoringMode: typeDef?.authoring ?? "human",
      },
      { additionalTypenames: ["Widget"] },
    );
    if (result.data?.createWidget?.id) {
      setShowNewDialog(false);
      router.push(`/admin/widgets/${result.data.createWidget.id}`);
    }
  };

  if (fetching) return <AdminLoader />;

  const widgets = data?.widgets ?? [];

  return (
    <div className="p-6 space-y-6 max-w-5xl">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Widgets</h1>
          <p className="text-muted-foreground text-sm mt-1">
            Standalone content blocks placed in broadsheet rows
          </p>
        </div>
        <Button onClick={() => setShowNewDialog(true)}>
          <Plus className="mr-2 h-4 w-4" />
          New Widget
        </Button>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-3 flex-wrap">
        <Select value={typeFilter} onValueChange={(v) => v && setTypeFilter(v)}>
          <SelectTrigger className="w-48">
            <SelectValue placeholder="All types" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All types</SelectItem>
            {WIDGET_TYPES.map((t) => (
              <SelectItem key={t.type} value={t.type}>
                {t.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <Select value={countyFilter} onValueChange={(v) => v && setCountyFilter(v)}>
          <SelectTrigger className="w-48">
            <SelectValue placeholder="All counties" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Counties</SelectItem>
            {countiesData?.counties?.map((c: { id: string; name: string }) => (
              <SelectItem key={c.id} value={c.id}>
                {c.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            setSearch(searchInput || undefined);
          }}
        >
          <Input
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            placeholder="Search..."
            className="w-48"
          />
          {search && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setSearchInput("");
                setSearch(undefined);
              }}
            >
              Clear
            </Button>
          )}
        </form>

        <span className="text-sm text-muted-foreground">
          {widgets.length} widget{widgets.length !== 1 ? "s" : ""}
        </span>
      </div>

      {/* Widget list */}
      {widgets.length === 0 ? (
        <div className="rounded-lg border border-dashed p-8 text-center text-muted-foreground">
          No widgets found. Create one to get started.
        </div>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Type</TableHead>
              <TableHead>Summary</TableHead>
              <TableHead>County</TableHead>
              <TableHead>Mode</TableHead>
              <TableHead>Created</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {widgets.map((w) => {
              const summary = getWidgetSummary(w.widgetType, w.data);
              return (
                <TableRow
                  key={w.id}
                  className="cursor-pointer"
                  onClick={() => router.push(`/admin/widgets/${w.id}`)}
                >
                  <TableCell>
                    <Badge
                      variant="secondary"
                      className={TYPE_COLORS[w.widgetType] ?? ""}
                    >
                      {formatType(w.widgetType)}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-muted-foreground truncate max-w-xs">
                    {summary || <span className="italic">Empty</span>}
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {w.county?.name ?? <span className="italic text-xs">—</span>}
                  </TableCell>
                  <TableCell>
                    <Badge
                      variant="outline"
                      className={AUTHORING_COLORS[w.authoringMode] ?? ""}
                    >
                      {w.authoringMode}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {new Date(w.createdAt).toLocaleDateString()}
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      )}

      {/* New Widget Dialog */}
      <Dialog open={showNewDialog} onOpenChange={setShowNewDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create Widget</DialogTitle>
            <DialogDescription>Choose a widget type to create.</DialogDescription>
          </DialogHeader>
          <div className="grid gap-2 py-2">
            {WIDGET_TYPES.map((t) => (
              <button
                key={t.type}
                className="flex items-center gap-3 p-3 rounded-lg border hover:bg-muted/50 transition-colors text-left"
                onClick={() => handleCreate(t.type)}
              >
                <Badge
                  variant="secondary"
                  className={TYPE_COLORS[t.type] ?? ""}
                >
                  {t.label}
                </Badge>
                <span className="text-sm text-muted-foreground">
                  {t.description}
                </span>
                <Badge
                  variant="outline"
                  className={`ml-auto text-xs ${AUTHORING_COLORS[t.authoring] ?? ""}`}
                >
                  {t.authoring}
                </Badge>
              </button>
            ))}
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

// ─── Helpers ────────────────────────────────────────────────────────────────

function formatType(type: string): string {
  return type
    .split("_")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

function getWidgetSummary(type: string, dataStr: string | null): string {
  if (!dataStr) return "";
  try {
    const data = typeof dataStr === "string" ? JSON.parse(dataStr) : dataStr;
    switch (type) {
      case "number":
      case "stat_card":
      case "number_block":
        return [data.number, data.title || data.label].filter(Boolean).join(" — ");
      case "pull_quote":
        return data.quote ? `"${data.quote.slice(0, 60)}${data.quote.length > 60 ? "..." : ""}"` : "";
      case "resource_bar":
        return data.label || "";
      case "weather":
        return data.config?.location || data.variant || "";
      case "section_sep":
        return data.title || "";
      default:
        return "";
    }
  } catch {
    return "";
  }
}
