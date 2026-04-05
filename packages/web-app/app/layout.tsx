import type { Metadata } from "next";
import GraphQLProvider from "@/lib/urql-provider";
import {
  featureDeck,
  featureDeckCondensed,
  featureText,
  blissfulRadiance,
  geistMono,
} from "./broadsheet-fonts";
import "./globals.css";
import "./broadsheet.css";
import "./broadsheet-detail.css";

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
      <head>
        <meta name="darkreader-lock" />
      </head>
      <body>
        <GraphQLProvider>
          {children}
        </GraphQLProvider>
      </body>
    </html>
  );
}
