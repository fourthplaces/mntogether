# Web App

Public-facing web application for Emergency Resource Aggregator built with React + GraphQL + Apollo.

## Tech Stack

- **React 18** - UI framework
- **TypeScript** - Type safety
- **Vite** - Build tool and dev server
- **Apollo Client** - GraphQL client
- **Tailwind CSS** - Styling
- **React Router** - Client-side routing

## Development

```bash
# Install dependencies
yarn install

# Start dev server (runs on port 3001)
yarn dev

# Build for production
yarn build

# Preview production build
yarn preview
```

## Features

### Home Page
- Browse published emergency needs/resources
- View organization details
- Contact organizations directly
- Responsive card-based layout

### PostCard Component
- Displays need details with urgency badges
- Automatic view tracking (analytics)
- Click tracking for contact interactions
- Color-coded urgency levels (urgent/high/medium/low)

## GraphQL API

The app connects to the GraphQL API at `http://localhost:8080/graphql`.

### Queries
- `publishedPosts` - Get all published needs/resources

### Mutations
- `submitNeed` - Submit a new need (future feature)
- `postViewed` - Track when a post is viewed
- `postClicked` - Track when contact info is clicked

## Project Structure

```
packages/web-app/
├── src/
│   ├── components/       # Reusable UI components
│   │   └── PostCard.tsx
│   ├── pages/           # Page components
│   │   └── Home.tsx
│   ├── graphql/         # GraphQL queries and mutations
│   │   ├── client.ts
│   │   ├── queries.ts
│   │   └── mutations.ts
│   ├── contexts/        # React contexts (future)
│   ├── App.tsx          # Root app component
│   ├── main.tsx         # Entry point
│   └── index.css        # Global styles
├── index.html
├── vite.config.ts
├── tailwind.config.js
├── tsconfig.json
└── package.json
```

## Development Server

The dev server runs on port 3001 and proxies GraphQL requests to the backend:

- Web app: http://localhost:3001
- GraphQL API: http://localhost:8080/graphql (proxied)

Make sure the backend server is running before starting the web app.

## Building

```bash
yarn build
```

Builds are output to `dist/` directory and can be deployed to any static hosting service.
