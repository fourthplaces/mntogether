import type { Metadata } from "next";
import { Inter } from "next/font/google";
import GraphQLProvider from "@/lib/urql-provider";
import { Header } from "@/components/Header";
import { Footer } from "@/components/Footer";
import "./globals.css";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "MN Together - Find help. Give help. Come together.",
  description: "Connecting volunteers with organizations and communities in need",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={inter.className}>
        <GraphQLProvider>
          <div className="min-h-screen bg-[#E8E2D5] text-[#3D3D3D]">
            <Header />
            <main>{children}</main>
            <Footer />
          </div>
        </GraphQLProvider>
      </body>
    </html>
  );
}
