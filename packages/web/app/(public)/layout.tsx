import { Header } from "@/components/public/Header";
import { Footer } from "@/components/public/Footer";

export default function PublicLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen bg-surface text-text-primary">
      <Header />
      <main>{children}</main>
      <Footer />
    </div>
  );
}
