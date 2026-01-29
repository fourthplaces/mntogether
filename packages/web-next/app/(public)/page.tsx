import Link from "next/link";

export default async function HomePage() {
  // Fetch data from GraphQL API for SSR
  // const data = await fetch(`${process.env.API_URL}/graphql`, { ... })

  return (
    <main className="min-h-screen bg-gradient-to-b from-blue-50 to-white">
      <div className="container mx-auto px-4 py-16">
        <div className="text-center mb-12">
          <h1 className="text-5xl font-bold text-gray-900 mb-4">
            MN Digital Aid
          </h1>
          <p className="text-xl text-gray-600 max-w-2xl mx-auto">
            Connecting immigrants with essential services in Minnesota
          </p>
        </div>

        <div className="grid md:grid-cols-3 gap-8 max-w-5xl mx-auto">
          <ServiceCard
            title="Legal Services"
            description="Find immigration legal help and representation"
            icon="⚖️"
          />
          <ServiceCard
            title="Healthcare"
            description="Access healthcare services and clinics"
            icon="🏥"
          />
          <ServiceCard
            title="Housing"
            description="Discover housing assistance and resources"
            icon="🏠"
          />
        </div>

        <div className="mt-12 text-center">
          <Link
            href="/search"
            className="inline-block bg-blue-600 text-white px-8 py-3 rounded-lg font-semibold hover:bg-blue-700 transition"
          >
            Search Services
          </Link>
        </div>
      </div>
    </main>
  );
}

function ServiceCard({
  title,
  description,
  icon,
}: {
  title: string;
  description: string;
  icon: string;
}) {
  return (
    <div className="bg-white p-6 rounded-lg shadow-md hover:shadow-lg transition">
      <div className="text-4xl mb-4">{icon}</div>
      <h3 className="text-xl font-semibold mb-2">{title}</h3>
      <p className="text-gray-600">{description}</p>
    </div>
  );
}
