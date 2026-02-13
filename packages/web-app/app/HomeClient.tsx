"use client";

import { useState } from "react";
import Link from "next/link";
import { SubmitSheet } from "@/components/SubmitSheet";
import { PostFeed } from "@/components/PostFeed";

type ActiveSheet = "search" | "submit" | null;

export function HomeClient() {
  const [activeSheet, setActiveSheet] = useState<ActiveSheet>(null);

  return (
    <div className="relative leading-relaxed">
      {/* Skyline background */}
     

      {/* Hero */}
      <section className="px-6 md:px-12 pt-16 pb-8 max-w-[1200px] mx-auto relative z-10">
        <h1 className="text-4xl sm:text-[3.5rem] font-bold text-[#3D3D3D] leading-tight max-w-[800px]">
          Strength in Community, Together.
        </h1>
      </section>

      {/* Pathway Cards */}
      <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-4 pb-16 grid grid-cols-1 md:grid-cols-3 gap-6 relative z-10">
        <Link href="/posts?post_type=offering" className="bg-[#F4D9B8] border-2 border-[#ECC89E] rounded-2xl p-8 flex flex-col cursor-pointer hover:bg-[#ECC89E] hover:border-[#E4B784] transition-all duration-300">
          <div className="flex items-center gap-3 mb-4">
            <img src="/icon-support.svg" alt="" className="w-10 h-10" />
            <h3 className="text-2xl font-bold text-[#3D3D3D]">I Want to Support</h3>
          </div>
          <p className="text-[#4D4D4D] text-base leading-relaxed mb-6 flex-1">
            Volunteer opportunities and ways to give back to neighbors who need it.
          </p>
          <span className="bg-[#3D3D3D] text-white px-7 py-3 rounded-full text-[0.95rem] font-semibold self-center mt-auto">
            Find Opportunities
          </span>
        </Link>

        <Link href="/posts?post_type=seeking" className="bg-[#B8CFC4] border-2 border-[#A0BFAF] rounded-2xl p-8 flex flex-col cursor-pointer hover:bg-[#A0BFAF] hover:border-[#88AF9A] transition-all duration-300">
          <div className="flex items-center gap-3 mb-4">
            <img src="/icon-help.svg" alt="" className="w-10 h-10" />
            <h3 className="text-2xl font-bold text-[#3D3D3D]">I Need Help</h3>
          </div>
          <p className="text-[#4D4D4D] text-base leading-relaxed mb-6 flex-1">
            People looking for food, shelter, legal aid, and other resources. You&apos;re not alone.
          </p>
          <span className="bg-[#3D3D3D] text-white px-7 py-3 rounded-full text-[0.95rem] font-semibold self-center mt-auto">
            Find Help
          </span>
        </Link>

        <Link href="/posts?post_type=announcement" className="bg-[#C4BAD4] border-2 border-[#B0A4C4] rounded-2xl p-8 flex flex-col cursor-pointer hover:bg-[#B0A4C4] hover:border-[#9C8EB4] transition-all duration-300">
          <div className="flex items-center gap-3 mb-4">
            <img src="/icon-events.svg" alt="" className="w-10 h-10" />
            <h3 className="text-2xl font-bold text-[#3D3D3D]">Community Events</h3>
          </div>
          <p className="text-[#4D4D4D] text-base leading-relaxed mb-6 flex-1">
            Food drives, gatherings, and opportunities to connect with your community.
          </p>
          <span className="bg-[#3D3D3D] text-white px-7 py-3 rounded-full text-[0.95rem] font-semibold self-center mt-auto">
            Explore Community Events
          </span>
        </Link>
      </section>

      {/* Posts Section */}
      <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-8 pb-16" style={{ zIndex: 10, position: "relative" }}>
        <PostFeed title="Get Involved" showSeeMore />
      </section>

      <div
        className="absolute inset-0 w-screen h-screen z-0 opacity-50 pointer-events-none"
        style={{
          backgroundImage: "url('/skyline.png')",
          backgroundPosition: "center 100px",
          backgroundRepeat: "no-repeat",
          backgroundSize: "80%",
        }}
      />

      {/* Bottom Sheets */}
      <SubmitSheet isOpen={activeSheet === "submit"} onClose={() => setActiveSheet(null)} />
    </div>
  );
}
