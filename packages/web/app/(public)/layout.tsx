import { Header } from "@/components/public/Header";
import { Footer } from "@/components/public/Footer";

export default function PublicLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen bg-[#E8E2D5] text-[#3D3D3D]">
      <Header />
      <main>{children}</main>
      <Footer />
    </div>
  );
}
