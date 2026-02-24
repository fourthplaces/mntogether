export function AdminLoader({ label = "Loading..." }: { label?: string }) {
  return (
    <div className="flex flex-col items-center justify-center py-24">
      <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-amber-600" />
      <p className="mt-3 text-sm text-stone-500">{label}</p>
    </div>
  );
}
