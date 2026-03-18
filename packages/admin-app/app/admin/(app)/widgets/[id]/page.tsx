"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import Link from "next/link";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import {
  WidgetDetailQuery,
  UpdateWidgetMutation,
  DeleteWidgetMutation,
} from "@/lib/graphql/widgets";
import { CountiesQuery } from "@/lib/graphql/editions";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogClose,
  DialogHeader,
  DialogFooter,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import { ArrowLeft, Check, Plus, Trash2, X } from "lucide-react";

// --- Types ------------------------------------------------------------------

interface FieldSpec {
  key: string;
  label: string;
  min: number;
  max: number;
  multiline?: boolean;
  placeholder?: string;
}

// --- Constants --------------------------------------------------------------

const TYPE_LABELS: Record<string, string> = {
  stat_card: "Stat Card",
  number_block: "Number Block",
  pull_quote: "Pull Quote",
  resource_bar: "Resource Bar",
  weather: "Weather",
  section_sep: "Section Sep",
};

const TYPE_COLORS: Record<string, string> = {
  stat_card: "bg-amber-100 text-amber-800",
  number_block: "bg-violet-100 text-violet-800",
  pull_quote: "bg-rose-100 text-rose-800",
  resource_bar: "bg-teal-100 text-teal-800",
  weather: "bg-sky-100 text-sky-800",
  section_sep: "bg-gray-100 text-gray-700",
};

const AUTHORING_COLORS: Record<string, string> = {
  human: "bg-emerald-100 text-emerald-800",
  automated: "bg-blue-100 text-blue-800",
  layout: "bg-gray-100 text-gray-700",
};

const NUMBER_BLOCK_COLORS = [
  { value: "teal", label: "Teal", swatch: "bg-teal-500" },
  { value: "rust", label: "Rust", swatch: "bg-orange-700" },
  { value: "forest", label: "Forest", swatch: "bg-green-800" },
  { value: "plum", label: "Plum", swatch: "bg-purple-700" },
  { value: "blue", label: "Blue", swatch: "bg-blue-600" },
];

const WEATHER_VARIANTS = [
  { value: "forecast", label: "Forecast (card grid)" },
  { value: "line", label: "Line (SVG chart)" },
  { value: "almanac", label: "Almanac (historical table)" },
  { value: "thermo", label: "Thermo (thermometer bars)" },
];

// --- Section label (matches post detail) ------------------------------------

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-3">
      {children}
    </h3>
  );
}

// --- Page -------------------------------------------------------------------

export default function WidgetDetailPage() {
  const params = useParams();
  const router = useRouter();
  const id = params.id as string;

  const [{ data, fetching }] = useQuery({
    query: WidgetDetailQuery,
    variables: { id },
  });

  const [, updateWidgetMut] = useMutation(UpdateWidgetMutation);
  const [, deleteWidget] = useMutation(DeleteWidgetMutation);
  const [{ data: countiesData }] = useQuery({ query: CountiesQuery });

  const [widgetData, setWidgetData] = useState<Record<string, unknown>>({});
  const [zipCode, setZipCode] = useState("");
  const [city, setCity] = useState("");
  const [countyId, setCountyId] = useState("");
  const [startDate, setStartDate] = useState("");
  const [endDate, setEndDate] = useState("");
  const [saved, setSaved] = useState(false);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout>>(undefined);
  const metaSaveTimerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  // Load widget data from query
  useEffect(() => {
    if (data?.widget) {
      try {
        const parsed =
          typeof data.widget.data === "string"
            ? JSON.parse(data.widget.data)
            : data.widget.data;
        setWidgetData(parsed);
      } catch {
        setWidgetData({});
      }
      setZipCode(data.widget.zipCode ?? "");
      setCity(data.widget.city ?? "");
      setCountyId(data.widget.countyId ?? "");
      setStartDate(data.widget.startDate ?? "");
      setEndDate(data.widget.endDate ?? "");
    }
  }, [data?.widget]);

  // Auto-save data (debounced)
  const save = useCallback(
    (newData: Record<string, unknown>) => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
      saveTimerRef.current = setTimeout(async () => {
        await updateWidgetMut({ id, data: JSON.stringify(newData) });
        setSaved(true);
        setTimeout(() => setSaved(false), 1500);
      }, 800);
    },
    [id, updateWidgetMut],
  );

  // Auto-save meta fields (location/dates) -- debounced
  const saveMeta = useCallback(
    (fields: { zipCode?: string; city?: string; countyId?: string; startDate?: string; endDate?: string }) => {
      if (metaSaveTimerRef.current) clearTimeout(metaSaveTimerRef.current);
      metaSaveTimerRef.current = setTimeout(async () => {
        await updateWidgetMut({
          id,
          zipCode: fields.zipCode ?? null,
          city: fields.city ?? null,
          countyId: fields.countyId || null,
          startDate: fields.startDate || null,
          endDate: fields.endDate || null,
        });
        setSaved(true);
        setTimeout(() => setSaved(false), 1500);
      }, 800);
    },
    [id, updateWidgetMut],
  );

  const updateField = useCallback(
    (key: string, value: unknown) => {
      setWidgetData((prev) => {
        const next = { ...prev, [key]: value };
        save(next);
        return next;
      });
    },
    [save],
  );

  const handleDelete = async () => {
    await deleteWidget({ id });
    router.push("/admin/widgets");
  };

  if (fetching) return <AdminLoader label="Loading widget..." />;

  if (!data?.widget) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="max-w-4xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-foreground mb-4">Widget Not Found</h1>
          <Link href="/admin/widgets" className="text-link hover:text-link-hover">
            Back to Widgets
          </Link>
        </div>
      </div>
    );
  }

  const widget = data.widget;
  const widgetType = widget.widgetType;

  return (
    <div className="min-h-screen bg-background px-4 py-4">
      <div className="max-w-7xl mx-auto">

        {/* Header bar */}
        <div className="flex items-center justify-between mb-4">
          <Link
            href="/admin/widgets"
            className="inline-flex items-center text-muted-foreground hover:text-foreground text-sm"
          >
            <ArrowLeft className="w-4 h-4 mr-1" /> Back to Widgets
          </Link>

          <div className="flex items-center gap-2">
            {saved && (
              <span className="text-xs text-emerald-600 flex items-center gap-1">
                <Check className="h-3 w-3" /> Saved
              </span>
            )}
            <DeleteConfirmButton onDelete={handleDelete} />
          </div>
        </div>

        {/* Two-column layout */}
        <div className="grid grid-cols-1 lg:grid-cols-[6fr_4fr] gap-6">

          {/* LEFT COLUMN */}
          <div className="space-y-6">

            {/* Title */}
            <h1 className="text-2xl font-bold text-foreground">
              {TYPE_LABELS[widgetType] ?? widgetType}
            </h1>

            {/* Location & Timing */}
            <div className="border-t border-border pt-4">
              <SectionLabel>Location & Timing</SectionLabel>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs text-muted-foreground uppercase mb-1">Zip Code</label>
                  <Input
                    value={zipCode}
                    placeholder="55401"
                    className="text-sm"
                    onChange={(e) => {
                      setZipCode(e.target.value);
                      saveMeta({ zipCode: e.target.value, city, countyId, startDate, endDate });
                    }}
                  />
                </div>
                <div>
                  <label className="block text-xs text-muted-foreground uppercase mb-1">City</label>
                  <Input
                    value={city}
                    placeholder="Minneapolis"
                    className="text-sm"
                    onChange={(e) => {
                      setCity(e.target.value);
                      saveMeta({ zipCode, city: e.target.value, countyId, startDate, endDate });
                    }}
                  />
                </div>
              </div>

              <div className="mt-3">
                <label className="block text-xs text-muted-foreground uppercase mb-1">County</label>
                <Select
                  value={countyId || "__none__"}
                  onValueChange={(v) => {
                    const val = v === "__none__" ? "" : (v ?? "");
                    setCountyId(val);
                    saveMeta({ zipCode, city, countyId: val, startDate, endDate });
                  }}
                >
                  <SelectTrigger className="text-sm w-full max-w-sm">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="__none__">No county (auto-derived from zip/city)</SelectItem>
                    {countiesData?.counties?.map((c: { id: string; name: string }) => (
                      <SelectItem key={c.id} value={c.id}>
                        {c.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                {!zipCode && !city && !countyId && (
                  <p className="text-xs text-amber-600 mt-1.5">
                    Set a zip, city, or county for this widget to appear in edition editors.
                  </p>
                )}
              </div>

              <div className="grid grid-cols-2 gap-3 mt-3">
                <div>
                  <label className="block text-xs text-muted-foreground uppercase mb-1">Start Date</label>
                  <Input
                    type="date"
                    value={startDate}
                    className="text-sm"
                    onChange={(e) => {
                      setStartDate(e.target.value);
                      saveMeta({ zipCode, city, countyId, startDate: e.target.value, endDate });
                    }}
                  />
                </div>
                <div>
                  <label className="block text-xs text-muted-foreground uppercase mb-1">End Date</label>
                  <Input
                    type="date"
                    value={endDate}
                    className="text-sm"
                    onChange={(e) => {
                      setEndDate(e.target.value);
                      saveMeta({ zipCode, city, countyId, startDate, endDate: e.target.value });
                    }}
                  />
                </div>
              </div>
              {!startDate && !endDate && (
                <p className="text-xs text-muted-foreground mt-1.5">No date range set -- widget is evergreen.</p>
              )}
            </div>

            {/* Widget Content */}
            <div className="border-t border-border pt-4">
              <SectionLabel>Widget Content</SectionLabel>
              <WidgetEditor
                widgetType={widgetType}
                data={widgetData}
                updateField={updateField}
                setWidgetData={(fn) => {
                  setWidgetData((prev) => {
                    const next = fn(prev);
                    save(next);
                    return next;
                  });
                }}
              />
            </div>
          </div>

          {/* RIGHT COLUMN */}
          <div className="space-y-6">

            {/* Widget Type */}
            <div>
              <SectionLabel>Widget Type</SectionLabel>
              <div className="flex items-center gap-2">
                <Badge
                  variant="secondary"
                  className={TYPE_COLORS[widgetType] ?? ""}
                >
                  {TYPE_LABELS[widgetType] ?? widgetType}
                </Badge>
                <Badge
                  variant="outline"
                  className={AUTHORING_COLORS[widget.authoringMode] ?? ""}
                >
                  {widget.authoringMode}
                </Badge>
              </div>
            </div>

            {/* Derived County */}
            {widget.county && (
              <div className="border-t border-border pt-4">
                <SectionLabel>Derived County</SectionLabel>
                <Badge variant="secondary">{widget.county.name}</Badge>
              </div>
            )}

            {/* System */}
            <div className="border-t border-border pt-4">
              <SectionLabel>System</SectionLabel>
              <div className="space-y-1.5 text-sm">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">ID</span>
                  <span className="text-foreground font-mono text-xs truncate max-w-[180px]">{widget.id}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Type</span>
                  <span className="text-foreground">{widgetType}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Authoring</span>
                  <span className="text-foreground">{widget.authoringMode}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Created</span>
                  <span className="text-foreground">{new Date(widget.createdAt).toLocaleString()}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Last edited</span>
                  <span className="text-foreground">{new Date(widget.updatedAt).toLocaleString()}</span>
                </div>
                {widget.countyId && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">County ID</span>
                    <span className="text-foreground font-mono text-xs truncate max-w-[180px]">{widget.countyId}</span>
                  </div>
                )}
              </div>
            </div>

            {/* JSON Preview */}
            <div className="border-t border-border pt-4">
              <SectionLabel>Data Preview</SectionLabel>
              <pre className="text-xs text-muted-foreground bg-muted/50 rounded-md p-3 overflow-auto max-h-64 font-mono">
                {JSON.stringify(widgetData, null, 2)}
              </pre>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// --- Type-specific editor ---------------------------------------------------

interface EditorProps {
  widgetType: string;
  data: Record<string, unknown>;
  updateField: (key: string, value: unknown) => void;
  setWidgetData: (fn: (prev: Record<string, unknown>) => Record<string, unknown>) => void;
}

function WidgetEditor({ widgetType, data, updateField, setWidgetData }: EditorProps) {
  switch (widgetType) {
    case "number":
    case "stat_card":
    case "number_block":
      return (
        <>
          <FieldsEditor
            data={data}
            updateField={updateField}
            fields={[
              { key: "number", label: "Number", min: 1, max: 6, placeholder: "2,847" },
              { key: "title", label: "Title", min: 1, max: 55, placeholder: "volunteer hours this month" },
              { key: "label", label: "Label (alt)", min: 0, max: 55, placeholder: "meals served across all partner locations" },
              { key: "body", label: "Body (optional)", min: 0, max: 100, multiline: true, placeholder: "Longer description providing context" },
              { key: "detail", label: "Detail (optional)", min: 0, max: 100, multiline: true, placeholder: "Additional context for the number" },
            ]}
          />
          <div className="space-y-2 mt-4">
            <label className="block text-xs text-muted-foreground uppercase mb-1">Color</label>
            <div className="flex gap-2">
              {NUMBER_BLOCK_COLORS.map((c) => (
                <button
                  key={c.value}
                  className={`w-8 h-8 rounded-full ${c.swatch} border-2 transition-all ${
                    (data.color || "teal") === c.value
                      ? "border-foreground scale-110"
                      : "border-transparent opacity-70 hover:opacity-100"
                  }`}
                  title={c.label}
                  onClick={() => updateField("color", c.value)}
                />
              ))}
            </div>
          </div>
        </>
      );

    case "pull_quote":
      return (
        <FieldsEditor
          data={data}
          updateField={updateField}
          fields={[
            { key: "quote", label: "Quote", min: 40, max: 140, multiline: true, placeholder: "Enter the quote text (curly quotes added by renderer)" },
            { key: "attribution", label: "Attribution", min: 10, max: 40, placeholder: "Resident, Phillips neighborhood" },
          ]}
        />
      );

    case "resource_bar":
      return <ResourceBarEditor data={data} updateField={updateField} setWidgetData={setWidgetData} />;

    case "weather":
      return <WeatherEditor data={data} updateField={updateField} setWidgetData={setWidgetData} />;

    case "section_sep":
      return (
        <FieldsEditor
          data={data}
          updateField={updateField}
          fields={[
            { key: "title", label: "Title", min: 8, max: 35, placeholder: "Community & Events" },
            { key: "sub", label: "Subtitle (optional)", min: 0, max: 60, placeholder: "What's happening near you" },
          ]}
        />
      );

    default:
      return (
        <div className="text-muted-foreground italic">
          Unknown widget type: {widgetType}
        </div>
      );
  }
}

// --- Shared text fields editor ----------------------------------------------

function FieldsEditor({
  data,
  updateField,
  fields,
}: {
  data: Record<string, unknown>;
  updateField: (key: string, value: unknown) => void;
  fields: FieldSpec[];
}) {
  return (
    <div className="space-y-4">
      {fields.map((f) => {
        const value = (data[f.key] as string) ?? "";
        const len = value.length;
        const overMax = f.max > 0 && len > f.max;
        const underMin = f.min > 0 && len > 0 && len < f.min;

        return (
          <div key={f.key}>
            <div className="flex items-center justify-between mb-1">
              <label className="block text-xs text-muted-foreground uppercase">{f.label}</label>
              {f.max > 0 && (
                <span
                  className={`text-xs tabular-nums ${
                    overMax
                      ? "text-red-500 font-medium"
                      : underMin
                        ? "text-amber-500"
                        : "text-muted-foreground"
                  }`}
                >
                  {len}/{f.max}
                </span>
              )}
            </div>
            {f.multiline ? (
              <Textarea
                id={f.key}
                value={value}
                placeholder={f.placeholder}
                onChange={(e) => updateField(f.key, e.target.value)}
                rows={3}
                className="text-sm"
              />
            ) : (
              <Input
                id={f.key}
                value={value}
                placeholder={f.placeholder}
                onChange={(e) => updateField(f.key, e.target.value)}
                className="text-sm"
              />
            )}
          </div>
        );
      })}
    </div>
  );
}

// --- Resource Bar editor ----------------------------------------------------

function ResourceBarEditor({
  data,
  updateField,
  setWidgetData,
}: {
  data: Record<string, unknown>;
  updateField: (key: string, value: unknown) => void;
  setWidgetData: (fn: (prev: Record<string, unknown>) => Record<string, unknown>) => void;
}) {
  const items = Array.isArray(data.items)
    ? (data.items as Array<{ number: string; text: string }>)
    : [];

  const updateItem = (idx: number, field: "number" | "text", value: string) => {
    setWidgetData((prev) => {
      const prevItems = Array.isArray(prev.items)
        ? [...(prev.items as Array<{ number: string; text: string }>)]
        : [];
      prevItems[idx] = { ...prevItems[idx], [field]: value };
      return { ...prev, items: prevItems };
    });
  };

  const addItem = () => {
    setWidgetData((prev) => {
      const prevItems = Array.isArray(prev.items)
        ? [...(prev.items as Array<{ number: string; text: string }>)]
        : [];
      return { ...prev, items: [...prevItems, { number: "", text: "" }] };
    });
  };

  const removeItem = (idx: number) => {
    setWidgetData((prev) => {
      const prevItems = Array.isArray(prev.items)
        ? [...(prev.items as Array<{ number: string; text: string }>)]
        : [];
      prevItems.splice(idx, 1);
      return { ...prev, items: prevItems };
    });
  };

  return (
    <div className="space-y-4">
      <div>
        <div className="flex items-center justify-between mb-1">
          <label className="block text-xs text-muted-foreground uppercase">Label</label>
          <span className="text-xs text-muted-foreground tabular-nums">
            {((data.label as string) ?? "").length}/25
          </span>
        </div>
        <Input
          value={(data.label as string) ?? ""}
          placeholder="Hotlines & Help:"
          className="text-sm"
          onChange={(e) => updateField("label", e.target.value)}
        />
      </div>

      <div>
        <div className="flex items-center justify-between mb-2">
          <label className="block text-xs text-muted-foreground uppercase">Items ({items.length}/8)</label>
          {items.length < 8 && (
            <Button variant="ghost" size="sm" onClick={addItem}>
              <Plus className="h-3 w-3 mr-1" />
              Add
            </Button>
          )}
        </div>
        <div className="space-y-2">
          {items.map((item, idx) => (
            <div key={idx} className="flex items-start gap-2">
              <div className="flex-1">
                <Input
                  value={item.number ?? ""}
                  placeholder="612-555-0100"
                  className="text-sm"
                  onChange={(e) => updateItem(idx, "number", e.target.value)}
                />
              </div>
              <div className="flex-[2]">
                <Input
                  value={item.text ?? ""}
                  placeholder="Crisis line"
                  className="text-sm"
                  onChange={(e) => updateItem(idx, "text", e.target.value)}
                />
              </div>
              {items.length > 1 && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="mt-0.5"
                  onClick={() => removeItem(idx)}
                >
                  <X className="h-3 w-3" />
                </Button>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// --- Weather editor ---------------------------------------------------------

function WeatherEditor({
  data,
  updateField,
  setWidgetData,
}: {
  data: Record<string, unknown>;
  updateField: (key: string, value: unknown) => void;
  setWidgetData: (fn: (prev: Record<string, unknown>) => Record<string, unknown>) => void;
}) {
  const config = (data.config as { location?: string }) ?? {};

  return (
    <div className="space-y-4">
      <div>
        <label className="block text-xs text-muted-foreground uppercase mb-1">Variant</label>
        <Select
          value={(data.variant as string) ?? "forecast"}
          onValueChange={(v) => updateField("variant", v)}
        >
          <SelectTrigger className="text-sm w-full max-w-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {WEATHER_VARIANTS.map((v) => (
              <SelectItem key={v.value} value={v.value}>
                {v.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div>
        <div className="flex items-center justify-between mb-1">
          <label className="block text-xs text-muted-foreground uppercase">Location</label>
          <span className="text-xs text-muted-foreground tabular-nums">
            {(config.location ?? "").length}/30
          </span>
        </div>
        <Input
          value={config.location ?? ""}
          placeholder="Minneapolis, MN"
          className="text-sm"
          onChange={(e) =>
            setWidgetData((prev) => ({
              ...prev,
              config: { ...(prev.config as object ?? {}), location: e.target.value },
            }))
          }
        />
        <p className="text-xs text-muted-foreground mt-1.5">
          Weather data is fetched automatically at build time. This field controls the data source.
        </p>
      </div>
    </div>
  );
}

// --- Delete confirm button --------------------------------------------------

function DeleteConfirmButton({ onDelete }: { onDelete: () => void }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button variant="destructive" size="sm" onClick={() => setOpen(true)}>
        Delete
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete widget?</DialogTitle>
            <DialogDescription>
              This will permanently remove this widget and any edition slots referencing it.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose render={<Button variant="outline" />}>Cancel</DialogClose>
            <Button
              variant="destructive"
              onClick={() => {
                onDelete();
                setOpen(false);
              }}
            >
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
