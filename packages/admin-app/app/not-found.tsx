export const metadata = {
  title: "Not Found",
};

export default function NotFound() {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        minHeight: "100vh",
        fontFamily: "system-ui, sans-serif",
        color: "#555",
      }}
    >
      <p>Not found</p>
    </div>
  );
}
