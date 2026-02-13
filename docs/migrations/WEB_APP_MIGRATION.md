# Web App Migration - Expo to React

Successfully migrated from Expo mobile app to React web app.

## What Changed

### Removed
- ❌ `packages/app/` - Expo mobile app
- ❌ Expo CLI dependency
- ❌ React Native components

### Added
- ✅ `packages/web-app/` - React web app
- ✅ Vite build tool
- ✅ Tailwind CSS styling
- ✅ Apollo Client GraphQL integration
- ✅ TypeScript support

## New Web App Structure

```
packages/web-app/
├── src/
│   ├── components/
│   │   └── PostCard.tsx          # Display published needs
│   ├── pages/
│   │   └── Home.tsx               # Main landing page
│   ├── graphql/
│   │   ├── client.ts              # Apollo Client setup
│   │   ├── queries.ts             # GraphQL queries
│   │   └── mutations.ts           # GraphQL mutations
│   ├── App.tsx                    # Root component
│   ├── main.tsx                   # Entry point
│   └── index.css                  # Global styles
├── index.html
├── vite.config.ts                 # Vite configuration
├── tailwind.config.js             # Tailwind configuration
└── package.json
```

## Tech Stack

- **React 18** - UI framework
- **TypeScript** - Type safety
- **Vite** - Fast build tool and dev server
- **Apollo Client 3.11** - GraphQL client
- **Tailwind CSS 3.4** - Utility-first CSS
- **React Router 7** - Client-side routing

## Features Implemented

### Home Page
- Browse published emergency needs/resources
- Responsive grid layout (mobile, tablet, desktop)
- Organization details and contact information
- Location display

### PostCard Component
- Urgency badges (urgent/high/medium/low)
- Color-coded urgency levels
- Automatic view tracking (analytics)
- Click tracking for contact interactions
- Email, phone, and website links

### GraphQL Integration
- Query: `publishedPosts` - Fetch all active needs
- Mutation: `postViewed` - Track post impressions
- Mutation: `postClicked` - Track contact clicks

## Development

### Start Dev Server
```bash
# Using dev CLI
./dev.sh
# Select: 🌐 Start web app

# Or directly
cd packages/web-app
yarn dev
```

Web app runs on: http://localhost:3001
GraphQL API proxied from: http://localhost:8080/graphql

### Build for Production
```bash
cd packages/web-app
yarn build
# Output: dist/
```

## Dev CLI Updates

The development CLI has been updated:

**Before:**
- 📱 Start mobile (Expo)

**After:**
- 🌐 Start web app

The CLI now:
- Installs `web-app` dependencies (instead of `app`)
- Starts Vite dev server (instead of Expo)
- Uses `yarn` for all package management

## Environment Setup

No additional environment variables needed for the web app. It uses the same GraphQL endpoint as the admin panel.

## Deployment

The web app can be deployed to any static hosting service:

- **Vercel** - Zero config, automatic deploys
- **Netlify** - Easy setup with redirects
- **Cloudflare Pages** - Fast CDN
- **S3 + CloudFront** - AWS hosting

Build command: `yarn build`
Output directory: `dist/`

## Migration Benefits

1. **Simpler Stack** - No native mobile dependencies
2. **Faster Development** - Vite hot reload (< 1s)
3. **Better Tooling** - Modern React ecosystem
4. **Easier Deployment** - Static hosting instead of app stores
5. **Responsive Design** - Works on all screen sizes
6. **Lower Barrier** - No Expo/React Native knowledge required

## Future Enhancements

- [ ] Submit need form for users
- [ ] Search and filter functionality
- [ ] Map view for locations
- [ ] User authentication (volunteer sign-up)
- [ ] Notifications for new needs
- [ ] Share functionality
- [ ] Print-friendly view

## Testing the Migration

1. Start the backend:
   ```bash
   cd packages/server
   docker-compose up
   ```

2. Start the web app:
   ```bash
   ./dev.sh
   # Select: 🌐 Start web app
   ```

3. Visit: http://localhost:3001

4. You should see:
   - List of published needs (if any exist)
   - Organization names and details
   - Contact information
   - Urgency badges

## Notes

### Development Mode
- Admin panel: http://localhost:3000 (standalone Vite dev server)
- Web app: http://localhost:3001 (standalone Vite dev server)
- Both use the same GraphQL API endpoint (proxied)

### Production/Embedded Mode
- Admin panel: http://localhost:8080/admin (embedded in server binary)
- Web app: http://localhost:8080/ (embedded in server binary)
- Both are compiled into the server binary at build time
- See `EMBEDDED_FRONTENDS.md` for details

## Rollback

If you need to revert to Expo:

1. The old `packages/app` directory was removed
2. Restore from git: `git checkout HEAD -- packages/app`
3. Update dev CLI to use Expo again
