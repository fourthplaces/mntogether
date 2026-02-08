"use client";

import Link from "next/link";
import { useRestate } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type { WebsiteList, PostList } from "@/lib/restate/types";

export default function DashboardPage() {
  const { data: websiteData, isLoading: websitesLoading } = useRestate<WebsiteList>(
    "Websites", "list", {}, { revalidateOnFocus: false }
  );
  const { data: postData, isLoading: postsLoading } = useRestate<PostList>(
    "Posts", "list", { status: "pending_approval" }, { revalidateOnFocus: false }
  );
  const { data: allPostData, isLoading: allPostsLoading } = useRestate<PostList>(
    "Posts", "list", {}, { revalidateOnFocus: false }
  );

  const isLoading = websitesLoading || postsLoading || allPostsLoading;

  if (isLoading) {
    return <AdminLoader label="Loading dashboard..." />;
  }

  const websites = websiteData?.websites || [];
  const pendingPosts = postData?.posts || [];
  const allPosts = allPostData?.posts || [];

  // Calculate stats
  const totalWebsites = websites.length;
  const approvedWebsites = websites.filter((d) => d.status === "approved").length;
  const pendingWebsites = websites.filter((d) => d.status === "pending_review").length;
  const totalPosts = allPosts.length;
  const pendingPostsCount = pendingPosts.length;
  const approvedPosts = allPosts.filter((l) => l.status === "active" || l.status === "approved").length;
  const totalPostsFromWebsites = websites.reduce((sum, d) => sum + (d.post_count || 0), 0);

  // Recent activity (last 7 days)
  const sevenDaysAgo = new Date();
  sevenDaysAgo.setDate(sevenDaysAgo.getDate() - 7);
  const recentWebsites = websites.filter(
    (d) => d.created_at && new Date(d.created_at) > sevenDaysAgo
  ).length;
  const recentPosts = allPosts.filter(
    (l) => new Date(l.created_at) > sevenDaysAgo
  ).length;

  const stats = [
    {
      title: "Total Websites",
      value: totalWebsites,
      subtitle: `${approvedWebsites} approved, ${pendingWebsites} pending`,
      color: "bg-blue-500",
      link: "/admin/websites",
    },
    {
      title: "Total Posts",
      value: totalPosts,
      subtitle: `${approvedPosts} approved, ${pendingPostsCount} pending`,
      color: "bg-green-500",
      link: "/admin/posts",
    },
    {
      title: "Pending Approvals",
      value: pendingWebsites + pendingPostsCount,
      subtitle: `${pendingWebsites} websites, ${pendingPostsCount} posts`,
      color: "bg-amber-500",
      link: "/admin/posts",
    },
    {
      title: "Scraped Posts",
      value: totalPostsFromWebsites,
      subtitle: `From ${approvedWebsites} approved websites`,
      color: "bg-purple-500",
      link: "/admin/scraped",
    },
  ];

  const recentActivity = [
    {
      title: "New Websites (7 days)",
      value: recentWebsites,
      icon: "\u{1F310}",
    },
    {
      title: "New Posts (7 days)",
      value: recentPosts,
      icon: "\u{1F4C4}",
    },
  ];

  const quickActions = [
    {
      title: "Review Pending Posts",
      description: "Approve or reject pending posts",
      icon: "\u{2705}",
      link: "/admin/posts",
      color: "bg-green-600 hover:bg-green-700",
      count: pendingPostsCount,
    },
    {
      title: "Approve Websites",
      description: "Review and approve pending websites",
      icon: "\u{1F310}",
      link: "/admin/websites",
      color: "bg-blue-600 hover:bg-blue-700",
      count: pendingWebsites,
    },
    {
      title: "Review Scraped Content",
      description: "See what agents have discovered",
      icon: "\u{1F50D}",
      link: "/admin/scraped",
      color: "bg-amber-600 hover:bg-amber-700",
    },
  ];

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-stone-900 mb-2">Admin Dashboard</h1>
          <p className="text-stone-600">Overview of your community resources platform</p>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          {stats.map((stat, index) => (
            <Link
              key={index}
              href={stat.link}
              className="bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow"
            >
              <div className="flex items-center justify-between mb-2">
                <h3 className="text-sm font-medium text-stone-600 uppercase tracking-wide">
                  {stat.title}
                </h3>
                <div className={`w-2 h-2 rounded-full ${stat.color}`} />
              </div>
              <p className="text-3xl font-bold text-stone-900 mb-1">{stat.value}</p>
              <p className="text-sm text-stone-500">{stat.subtitle}</p>
            </Link>
          ))}
        </div>

        {/* Recent Activity */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
          {recentActivity.map((activity, index) => (
            <div key={index} className="bg-white rounded-lg shadow-md p-6">
              <div className="flex items-center gap-3 mb-2">
                <span className="text-2xl">{activity.icon}</span>
                <h3 className="text-lg font-semibold text-stone-900">{activity.title}</h3>
              </div>
              <p className="text-4xl font-bold text-amber-600">{activity.value}</p>
            </div>
          ))}
        </div>

        {/* Quick Actions */}
        <div className="mb-8">
          <h2 className="text-2xl font-bold text-stone-900 mb-4">Quick Actions</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {quickActions.map((action, index) => (
              <Link
                key={index}
                href={action.link}
                className={`${action.color} text-white rounded-lg shadow-md p-6 transition-all hover:shadow-lg relative overflow-hidden`}
              >
                {action.count !== undefined && action.count > 0 && (
                  <div className="absolute top-2 right-2 bg-white text-stone-900 rounded-full w-8 h-8 flex items-center justify-center text-sm font-bold">
                    {action.count}
                  </div>
                )}
                <div className="text-4xl mb-3">{action.icon}</div>
                <h3 className="text-lg font-semibold mb-1">{action.title}</h3>
                <p className="text-sm opacity-90">{action.description}</p>
              </Link>
            ))}
          </div>
        </div>

        {/* System Status */}
        <div className="bg-white rounded-lg shadow-md p-6">
          <h2 className="text-xl font-bold text-stone-900 mb-4">System Status</h2>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="flex items-center gap-3">
              <div className="w-3 h-3 rounded-full bg-green-500 animate-pulse" />
              <div>
                <p className="text-sm font-medium text-stone-900">Scraper</p>
                <p className="text-xs text-stone-600">Running (hourly)</p>
              </div>
            </div>
            <div className="flex items-center gap-3">
              <div className="w-3 h-3 rounded-full bg-green-500 animate-pulse" />
              <div>
                <p className="text-sm font-medium text-stone-900">Agent Search</p>
                <p className="text-xs text-stone-600">Active (hourly)</p>
              </div>
            </div>
            <div className="flex items-center gap-3">
              <div className="w-3 h-3 rounded-full bg-green-500" />
              <div>
                <p className="text-sm font-medium text-stone-900">Database</p>
                <p className="text-xs text-stone-600">Healthy</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
