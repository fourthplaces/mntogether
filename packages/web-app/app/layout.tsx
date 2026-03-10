import type { Metadata } from "next";
import GraphQLProvider from "@/lib/urql-provider";
import { Header } from "@/components/Header";
import { Footer } from "@/components/Footer";
import {
  featureDeck,
  featureDeckCondensed,
  featureText,
  blissfulRadiance,
  geistMono,
} from "./broadsheet-fonts";
import "./globals.css";

export const metadata: Metadata = {
  title: "MN Together - Find help. Give help. Come together.",
  description: "Connecting volunteers with organizations and communities in need",
};

const fontVariables = [
  featureDeck.variable,
  featureDeckCondensed.variable,
  featureText.variable,
  blissfulRadiance.variable,
  geistMono.variable,
].join(" ");

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={fontVariables} suppressHydrationWarning>
      <body>
        <GraphQLProvider>
          <div className="app-shell">
            <Header />
            <main>{children}</main>
            <Footer />
          </div>
        </GraphQLProvider>
      </body>
    </html>
  );
}
