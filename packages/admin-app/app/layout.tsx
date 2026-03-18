import type { Metadata } from "next";
import { Inter, Geist } from "next/font/google";
import GraphQLProvider from "@/lib/urql-provider";
import { TooltipProvider } from "@/components/ui/tooltip";
import { featureDeck, featureDeckCondensed, featureText } from "./broadsheet-fonts";
import "./globals.css";
import { cn } from "@/lib/utils";

const geist = Geist({subsets:['latin'],variable:'--font-sans'});

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
    <html lang="en" suppressHydrationWarning className={cn("font-sans", geist.variable, featureDeck.variable, featureDeckCondensed.variable, featureText.variable)}>
      <body className={inter.className}>
        <GraphQLProvider>
          <TooltipProvider>{children}</TooltipProvider>
        </GraphQLProvider>
      </body>
    </html>
  );
}
