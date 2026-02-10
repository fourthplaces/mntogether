"use client";

import Link from "next/link";
import { useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { useRestate, callService, invalidateService } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type {
  OrganizationResult,
  SocialProfileListResult,
  SocialProfileResult,
  WebsiteList,
} from "@/lib/restate/types";

const PLATFORMS = ["instagram", "facebook", "tiktok"];

export default function OrganizationDetailPage() {
  const params = useParams();
  const router = useRouter();
  const orgId = params.id as string;

  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [editLoading, setEditLoading] = useState(false);
  const [editError, setEditError] = useState<string | null>(null);

  const {
    data: org,
    isLoading: orgLoading,
    error: orgError,
    mutate: refetchOrg,
  } = useRestate<OrganizationResult>("Organizations", "get", { id: orgId }, {
    revalidateOnFocus: false,
  });

  const { data: profilesData, mutate: refetchProfiles } =
    useRestate<SocialProfileListResult>("SocialProfiles", "list_by_organization", {
      organization_id: orgId,
    }, { revalidateOnFocus: false });

  const { data: websitesData } = useRestate<WebsiteList>("Websites", "list", {
    organization_id: orgId,
    first: 50,
  }, { revalidateOnFocus: false });

  const profiles = profilesData?.profiles || [];
  const websites = websitesData?.websites || [];

  const startEditing = () => {
    if (!org) return;
    setEditName(org.name);
    setEditDescription(org.description || "");
    setEditing(true);
    setEditError(null);
  };

  const handleUpdate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editName.trim()) return;

    setEditLoading(true);
    setEditError(null);
    try {
      await callService("Organizations", "update", {
        id: orgId,
        name: editName.trim(),
        description: editDescription.trim() || null,
      });
      invalidateService("Organizations");
      refetchOrg();
      setEditing(false);
    } catch (err: any) {
      setEditError(err.message || "Failed to update organization");
    } finally {
      setEditLoading(false);
    }
  };

  const handleDelete = async () => {
    if (!confirm("Are you sure you want to delete this organization?")) return;
    try {
      await callService("Organizations", "delete", { id: orgId });
      invalidateService("Organizations");
      router.push("/admin/organizations");
    } catch (err: any) {
      alert(err.message || "Failed to delete organization");
    }
  };

  if (orgLoading) {
    return <AdminLoader label="Loading organization..." />;
  }

  if (orgError) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-5xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-red-600 mb-4">Error</h1>
          <p className="text-stone-600 mb-4">{orgError.message}</p>
          <Link href="/admin/organizations" className="text-amber-600 hover:text-amber-800">
            Back to Organizations
          </Link>
        </div>
      </div>
    );
  }

  if (!org) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-5xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-stone-900 mb-4">Not Found</h1>
          <Link href="/admin/organizations" className="text-amber-600 hover:text-amber-800">
            Back to Organizations
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-5xl mx-auto">
        <Link
          href="/admin/organizations"
          className="inline-flex items-center text-stone-600 hover:text-stone-900 mb-6"
        >
          {"\u2190"} Back to Organizations
        </Link>

        {/* Header */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          {editing ? (
            <form onSubmit={handleUpdate} className="space-y-3">
              <input
                type="text"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
                className="w-full px-3 py-2 border border-stone-300 rounded-lg text-lg font-bold focus:outline-none focus:ring-2 focus:ring-amber-500"
                autoFocus
                disabled={editLoading}
              />
              <textarea
                value={editDescription}
                onChange={(e) => setEditDescription(e.target.value)}
                placeholder="Description (optional)"
                rows={3}
                className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                disabled={editLoading}
              />
              <div className="flex items-center gap-2">
                <button
                  type="submit"
                  disabled={editLoading || !editName.trim()}
                  className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
                >
                  {editLoading ? "Saving..." : "Save"}
                </button>
                <button
                  type="button"
                  onClick={() => setEditing(false)}
                  className="px-3 py-2 text-stone-500 hover:text-stone-700 text-sm"
                >
                  Cancel
                </button>
                {editError && <span className="text-red-600 text-sm">{editError}</span>}
              </div>
            </form>
          ) : (
            <div className="flex justify-between items-start">
              <div>
                <h1 className="text-2xl font-bold text-stone-900 mb-1">{org.name}</h1>
                {org.description && (
                  <p className="text-stone-600">{org.description}</p>
                )}
              </div>
              <div className="flex gap-2">
                <button
                  onClick={startEditing}
                  className="px-3 py-1.5 rounded-lg text-sm font-medium bg-stone-100 text-stone-700 hover:bg-stone-200 transition-colors"
                >
                  Edit
                </button>
                <button
                  onClick={handleDelete}
                  className="px-3 py-1.5 rounded-lg text-sm font-medium text-red-600 hover:bg-red-50 transition-colors"
                >
                  Delete
                </button>
              </div>
            </div>
          )}

          <div className="grid grid-cols-3 gap-4 pt-4 mt-4 border-t border-stone-200">
            <div>
              <span className="text-xs text-stone-500 uppercase">Websites</span>
              <p className="text-lg font-semibold text-stone-900">{org.website_count}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Social Profiles</span>
              <p className="text-lg font-semibold text-stone-900">{org.social_profile_count}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Created</span>
              <p className="text-sm font-medium text-stone-900">
                {new Date(org.created_at).toLocaleDateString()}
              </p>
            </div>
          </div>
        </div>

        {/* Linked Websites */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-lg font-semibold text-stone-900 mb-4">Linked Websites</h2>
          {websites.length === 0 ? (
            <p className="text-stone-500 text-sm">
              No websites linked. Assign this organization from a website's detail page.
            </p>
          ) : (
            <div className="space-y-2">
              {websites.map((website) => (
                <Link
                  key={website.id}
                  href={`/admin/websites/${website.id}`}
                  className="flex items-center justify-between p-3 rounded-lg border border-stone-200 hover:bg-stone-50"
                >
                  <div className="flex items-center gap-3">
                    <span className="font-medium text-stone-900">{website.domain}</span>
                    <span
                      className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                        website.status === "approved"
                          ? "bg-green-100 text-green-800"
                          : website.status === "pending_review"
                            ? "bg-yellow-100 text-yellow-800"
                            : "bg-stone-100 text-stone-600"
                      }`}
                    >
                      {website.status.replace(/_/g, " ")}
                    </span>
                  </div>
                  <span className="text-sm text-stone-500">
                    {website.post_count || 0} posts
                  </span>
                </Link>
              ))}
            </div>
          )}
        </div>

        {/* Social Profiles */}
        <div className="bg-white rounded-lg shadow-md p-6">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-stone-900">Social Profiles</h2>
          </div>

          <AddSocialProfileForm orgId={orgId} onAdded={() => {
            refetchProfiles();
            refetchOrg();
          }} />

          {profiles.length === 0 ? (
            <p className="text-stone-500 text-sm mt-4">No social profiles yet.</p>
          ) : (
            <div className="space-y-2 mt-4">
              {profiles.map((profile) => (
                <SocialProfileRow
                  key={profile.id}
                  profile={profile}
                  onDeleted={() => {
                    refetchProfiles();
                    refetchOrg();
                  }}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function AddSocialProfileForm({
  orgId,
  onAdded,
}: {
  orgId: string;
  onAdded: () => void;
}) {
  const [platform, setPlatform] = useState("instagram");
  const [handle, setHandle] = useState("");
  const [url, setUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!handle.trim()) return;

    setLoading(true);
    setError(null);
    try {
      await callService("SocialProfiles", "create", {
        organization_id: orgId,
        platform,
        handle: handle.trim(),
        url: url.trim() || null,
      });
      invalidateService("SocialProfiles");
      setHandle("");
      setUrl("");
      onAdded();
    } catch (err: any) {
      setError(err.message || "Failed to add profile");
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="flex items-center gap-2 bg-stone-50 rounded-lg px-3 py-2"
    >
      <select
        value={platform}
        onChange={(e) => setPlatform(e.target.value)}
        className="px-2 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 bg-white"
        disabled={loading}
      >
        {PLATFORMS.map((p) => (
          <option key={p} value={p}>
            {p.charAt(0).toUpperCase() + p.slice(1)}
          </option>
        ))}
      </select>
      <input
        type="text"
        value={handle}
        onChange={(e) => setHandle(e.target.value)}
        placeholder="Handle (e.g. @example)"
        className="flex-1 px-3 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
        disabled={loading}
      />
      <input
        type="text"
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        placeholder="URL (optional)"
        className="flex-1 px-3 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
        disabled={loading}
      />
      <button
        type="submit"
        disabled={loading || !handle.trim()}
        className="px-3 py-1.5 bg-amber-600 text-white rounded text-sm font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
      >
        {loading ? "..." : "Add"}
      </button>
      {error && <span className="text-red-600 text-xs">{error}</span>}
    </form>
  );
}

function SocialProfileRow({
  profile,
  onDeleted,
}: {
  profile: SocialProfileResult;
  onDeleted: () => void;
}) {
  const handleDelete = async () => {
    try {
      await callService("SocialProfiles", "delete", { id: profile.id });
      invalidateService("SocialProfiles");
      onDeleted();
    } catch (err: any) {
      console.error("Failed to delete profile:", err);
    }
  };

  const platformIcon: Record<string, string> = {
    instagram: "IG",
    facebook: "FB",
    tiktok: "TT",
  };

  return (
    <div className="flex items-center justify-between p-3 rounded-lg border border-stone-200">
      <div className="flex items-center gap-3">
        <span className="px-2 py-0.5 text-xs rounded-full font-medium bg-purple-100 text-purple-800">
          {platformIcon[profile.platform] || profile.platform}
        </span>
        <span className="font-medium text-stone-900">{profile.handle}</span>
        {profile.url && (
          <a
            href={profile.url}
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm text-blue-600 hover:text-blue-800"
            onClick={(e) => e.stopPropagation()}
          >
            {"\u2197"}
          </a>
        )}
        <span className="text-xs text-stone-400">
          every {profile.scrape_frequency_hours}h
        </span>
        {profile.last_scraped_at && (
          <span className="text-xs text-stone-400">
            last: {new Date(profile.last_scraped_at).toLocaleDateString()}
          </span>
        )}
      </div>
      <button
        onClick={handleDelete}
        className="px-2 py-1 text-xs text-stone-500 hover:text-red-700 hover:bg-red-50 rounded transition-colors"
      >
        Delete
      </button>
    </div>
  );
}
