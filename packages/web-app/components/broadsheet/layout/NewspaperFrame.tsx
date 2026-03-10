export function NewspaperFrame({ children }: { children: React.ReactNode }) {
  return (
    <div className="newspaper">
      <div className="crease-under" />
      <div className="content">
        {children}
      </div>
    </div>
  );
}
