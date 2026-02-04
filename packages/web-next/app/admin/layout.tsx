"use client";

import { useState } from "react";
import { AdminNav } from "@/components/admin/AdminNav";
import { Chatroom } from "@/components/admin/Chatroom";

export default function AdminLayout({ children }: { children: React.ReactNode }) {
  const [isChatOpen, setIsChatOpen] = useState(false);

  return (
    <div className="min-h-screen bg-stone-50">
      <AdminNav />

      <main className="pb-20">{children}</main>

      {/* Chat FAB */}
      <button
        onClick={() => setIsChatOpen(true)}
        className="fixed bottom-6 right-6 w-14 h-14 bg-amber-500 text-white rounded-full shadow-lg hover:bg-amber-600 transition-colors flex items-center justify-center text-2xl z-40"
        title="Open Assistant"
      >
        {"\u{1F4AC}"}
      </button>

      {/* Chatroom Sidebar */}
      <Chatroom isOpen={isChatOpen} onClose={() => setIsChatOpen(false)} withAgent="admin" />
    </div>
  );
}
