"use client";

/**
 * CountyPicker — select a Minnesota county and navigate to its broadsheet.
 *
 * For now this sits on the public home page as a simple native <select>
 * that updates the `?county=<id>` URL param. When the param changes,
 * HomeClient re-queries `publicBroadsheet(countyId)` and renders that
 * county's current published edition.
 *
 * Future work:
 *   - Replace with a styled command-palette-style picker (search +
 *     map affordances) once we have published editions across more
 *     counties to browse.
 *   - Auto-select a county based on IP geolocation — with a
 *     "Statewide" fallback for out-of-state visitors. Tracked in the
 *     layout-engine roadmap.
 */

import { useRouter, useSearchParams } from "next/navigation";
import { useQuery } from "urql";
import { CountiesQuery } from "@/lib/graphql/public";

interface CountyPickerProps {
  /** Current county id (from URL param). Empty string = none selected. */
  selectedId: string;
}

export function CountyPicker({ selectedId }: CountyPickerProps) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [{ data }] = useQuery({ query: CountiesQuery });
  const counties = data?.counties ?? [];

  // Split pseudo counties (Statewide) from the real 87 so we can group
  // them in the dropdown with a visual separator. Pseudo rows sit at
  // the top since they're the sensible default for visitors who don't
  // know their county or are out of state.
  const pseudo = counties.filter((c) => c.isPseudo);
  const real = counties.filter((c) => !c.isPseudo);

  function onChange(e: React.ChangeEvent<HTMLSelectElement>) {
    const next = e.target.value;
    const params = new URLSearchParams(searchParams?.toString() ?? "");
    if (next) params.set("county", next);
    else params.delete("county");
    const qs = params.toString();
    router.push(qs ? `/?${qs}` : "/");
  }

  return (
    <label className="county-picker" aria-label="Choose a county">
      <span className="county-picker__label mono-sm">Viewing:</span>
      <select
        className="county-picker__select"
        value={selectedId}
        onChange={onChange}
      >
        {pseudo.length > 0 && (
          <optgroup label="All of Minnesota">
            {pseudo.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </optgroup>
        )}
        {real.length > 0 && (
          <optgroup label="By county">
            {real.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name} County
              </option>
            ))}
          </optgroup>
        )}
      </select>
    </label>
  );
}
