"use client";

import { useState } from "react";
import Link from "next/link";
import { useRestate } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type { JobListResult, JobResult } from "@/lib/restate/types";

type StatusFilter = "running" | "completed" | "backing-off" | null;

const WORKFLOW_LABELS: Record<string, string> = {
  CrawlWebsiteWorkflow: "Crawl Website",
  RegeneratePostsWorkflow: "Regenerate Posts",
  DeduplicatePostsWorkflow: "Deduplicate Posts",
  ExtractPostsFromUrlWorkflow: "Extract Posts",
  WebsiteResearchWorkflow: "Website Research",
  RegisterMemberWorkflow: "Register Member",
};

function workflowLabel(name: string): string {
  return WORKFLOW_LABELS[name] || name.replace(/Workflow$/, "").replace(/([A-Z])/g, " $1").trim();
}

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, string> = {
    running: "bg-blue-100 text-blue-800",
    completed: "bg-green-100 text-green-800",
    failed: "bg-red-100 text-red-800",
    suspended: "bg-yellow-100 text-yellow-800",
  };
  return (
    <span
      className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${colors[status] || "bg-stone-100 text-stone-600"}`}
    >
      {status}
    </span>
  );
}

function timeAgo(dateStr: string | null | undefined): string {
  if (!dateStr) return "";
  const date = new Date(dateStr);
  if (isNaN(date.getTime())) return "";
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}d ago`;
}

function duration(startStr: string | null | undefined, endStr: string | null | undefined): string {
  if (!startStr) return "";
  const start = new Date(startStr);
  const end = endStr ? new Date(endStr) : new Date();
  if (isNaN(start.getTime())) return "";
  const diffMs = end.getTime() - start.getTime();
  if (diffMs < 0) return "";
  const seconds = Math.floor(diffMs / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSec = seconds % 60;
  if (minutes < 60) return `${minutes}m ${remainingSec}s`;
  const hours = Math.floor(minutes / 60);
  const remainingMin = minutes % 60;
  return `${hours}h ${remainingMin}m`;
}

function JobRow({ job }: { job: JobResult }) {
  return (
    <tr className="hover:bg-stone-50">
      <td className="px-6 py-4 whitespace-nowrap">
        <StatusBadge status={job.status} />
      </td>
      <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-stone-900">
        {workflowLabel(job.workflow_name)}
      </td>
      <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-600">
        {job.website_id ? (
          <Link
            href={`/admin/websites/${job.website_id}`}
            className="underline decoration-stone-300 hover:decoration-stone-600 hover:text-stone-900 transition-colors"
          >
            {job.website_domain || job.website_id.slice(0, 8) + "..."}
          </Link>
        ) : (
          <span className="text-stone-400">-</span>
        )}
      </td>
      <td className="px-6 py-4 text-sm text-stone-600 max-w-xs truncate">
        {job.progress || <span className="text-stone-400">-</span>}
      </td>
      <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-500">
        {timeAgo(job.created_at)}
      </td>
      <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-500">
        {duration(job.created_at, job.completed_at || job.modified_at)}
      </td>
    </tr>
  );
}

export default function JobsPage() {
  const [filter, setFilter] = useState<StatusFilter>(null);

  const variables: Record<string, unknown> = { limit: 100 };
  if (filter) {
    variables.status = filter;
  }

  const isRunningView = filter === "running";

  const { data, isLoading } = useRestate<JobListResult>(
    "Jobs",
    "list",
    variables,
    {
      revalidateOnFocus: false,
      keepPreviousData: true,
      ...(isRunningView ? { refreshInterval: 5000 } : {}),
    }
  );

  const jobs = data?.jobs || [];

  const runningCount = filter === null
    ? jobs.filter((j) => j.status === "running").length
    : undefined;

  const filters: { label: string; value: StatusFilter }[] = [
    { label: "All", value: null },
    { label: "Running", value: "running" },
    { label: "Completed", value: "completed" },
    { label: "Failed", value: "backing-off" },
  ];

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        <div className="mb-6">
          <h1 className="text-3xl font-bold text-stone-900">Workflow Jobs</h1>
          <p className="text-sm text-stone-600 mt-1">
            All workflow invocations with live progress.
            {isRunningView && (
              <span className="text-blue-600 ml-2">Auto-refreshing every 5s</span>
            )}
          </p>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-3 gap-4 mb-6">
          <div className="bg-white border border-stone-200 rounded-lg p-4">
            <div className="text-2xl font-bold text-stone-900">
              {runningCount !== undefined ? runningCount : jobs.length}
            </div>
            <div className="text-xs text-stone-500">
              {runningCount !== undefined ? "Currently running" : `${filter || "all"} jobs shown`}
            </div>
          </div>
          <div className="bg-white border border-stone-200 rounded-lg p-4">
            <div className="text-2xl font-bold text-stone-900">{jobs.length}</div>
            <div className="text-xs text-stone-500">Jobs shown</div>
          </div>
          <div className="bg-white border border-stone-200 rounded-lg p-4">
            <div className="text-2xl font-bold text-stone-900">
              {new Set(jobs.map((j) => j.workflow_name)).size}
            </div>
            <div className="text-xs text-stone-500">Workflow types</div>
          </div>
        </div>

        {/* Filter tabs */}
        <div className="flex gap-1 mb-4">
          {filters.map((f) => (
            <button
              key={f.label}
              onClick={() => setFilter(f.value)}
              className={`px-3 py-1.5 text-sm font-medium rounded ${
                filter === f.value
                  ? "bg-amber-100 text-amber-800"
                  : "text-stone-600 hover:bg-stone-100"
              }`}
            >
              {f.label}
            </button>
          ))}
        </div>

        {/* Job list */}
        {isLoading && jobs.length === 0 ? (
          <AdminLoader />
        ) : jobs.length === 0 ? (
          <div className="bg-white border border-stone-200 rounded-lg p-12 text-center">
            <p className="text-stone-500">
              {filter
                ? `No ${filter} jobs found.`
                : "No workflow jobs found."}
            </p>
          </div>
        ) : (
          <div className="bg-white rounded-lg shadow overflow-hidden">
            <table className="min-w-full divide-y divide-stone-200">
              <thead className="bg-stone-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Status
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Workflow
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Website
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Progress
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Started
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Duration
                  </th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-stone-200">
                {jobs.map((job) => (
                  <JobRow key={job.id} job={job} />
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
