# Embedded Frontends in Server

The server embeds both the admin panel and public web app as static assets at compile time.

## Architecture

Both frontends are built into the server binary using `rust-embed`:

```
Server Binary
├── Admin Panel (/admin/*)      → packages/admin-spa/dist
└── Web App (/)                 → packages/web-app/dist
```

## Routes

### Admin Panel
- **URL:** `http://localhost:8080/admin`
- **Source:** `packages/admin-spa/`
- **Protected:** Yes (requires JWT authentication)
- **Purpose:** Approve/reject needs, manage content

### Web App
- **URL:** `http://localhost:8080/`
- **Source:** `packages/web-app/`
- **Protected:** No (public access)
- **Purpose:** Browse published needs, view organizations

## Build Process

### Automatic Build (build.rs)

When you build the server, both frontends are automatically built:

```bash
cargo build --bin server
```

The `build.rs` script:
1. Detects yarn/npm
2. Builds `packages/admin-spa/` → `dist/`
3. Builds `packages/web-app/` → `dist/`
4. Embeds both `dist/` folders into the server binary

### Development Mode (dev-watch.sh)

When Docker starts, the dev-watch script:
1. Builds admin-spa on startup
2. Builds web-app on startup
3. Starts cargo-watch (rebuilds on Rust changes)

### Skip Frontend Builds

To skip frontend builds (faster Rust-only iteration):

```bash
SKIP_FRONTEND_BUILD=1 cargo build --bin server
```

## File Structure

```
packages/
├── server/
│   ├── build.rs                    # Builds both frontends
│   ├── dev-watch.sh                # Dev container startup
│   └── src/server/
│       ├── static_files.rs         # Embed & serve logic
│       │   ├── AdminAssets         # ../admin-spa/dist
│       │   ├── WebAppAssets        # ../web-app/dist
│       │   ├── serve_admin()       # /admin handler
│       │   └── serve_web_app()     # / handler
│       └── app.rs                  # Route registration
│           ├── /admin → serve_admin
│           ├── /admin/*path → serve_admin
│           ├── / → serve_web_app
│           └── /*path → serve_web_app (catch-all)
│
├── admin-spa/
│   ├── src/                        # React source
│   ├── dist/                       # Build output (embedded)
│   └── package.json
│
└── web-app/
    ├── src/                        # React source
    ├── dist/                       # Build output (embedded)
    └── package.json
```

## Route Priority

Routes are matched in order:

1. `/graphql` - GraphQL API
2. `/health` - Health check
3. `/admin`, `/admin/*path` - Admin panel (protected)
4. `/`, `/*path` - Web app (catch-all, must be last)

The catch-all `/*path` ensures client-side routing works for both SPAs.

## Development Workflow

### Separate Dev Servers (Recommended)

For fastest development with HMR:

```bash
# Terminal 1: Start server
cd packages/server
docker-compose up

# Terminal 2: Start web-app dev server
cd packages/web-app
yarn dev
# Visit: http://localhost:3001

# Terminal 3: Start admin-spa dev server (if needed)
cd packages/admin-spa
yarn dev
# Visit: http://localhost:3000
```

### Embedded Mode

To test the embedded builds:

```bash
# Build frontends
cd packages/web-app && yarn build
cd packages/admin-spa && yarn build

# Rebuild server (embeds the dist folders)
cd packages/server
cargo build --bin server

# Run server
docker-compose up --build

# Visit:
# - http://localhost:8080 (web app - embedded)
# - http://localhost:8080/admin (admin panel - embedded)
```

## Production Deployment

In production, the server binary contains both frontends:

1. Build frontends: `yarn build` in each package
2. Build server: `cargo build --release --bin server`
3. Deploy single binary
4. Access both apps through single domain:
   - `https://yourdomain.com/` → Web app
   - `https://yourdomain.com/admin` → Admin panel

## Benefits

### Single Binary Deployment
- No separate static hosting needed
- Single deployment process
- Easier SSL/HTTPS setup

### Consistent CORS
- Both frontends on same domain
- No cross-origin issues
- Simplified authentication

### Simple Routing
- No reverse proxy configuration needed
- Server handles all routes
- SPA fallback built-in

## Updating Frontends

### After Frontend Changes

```bash
# Rebuild specific frontend
cd packages/web-app
yarn build

# Rebuild server to embed new build
cd packages/server
cargo build
```

### In Docker Development

```bash
# Restart server container (rebuilds frontends)
docker-compose restart api
```

## Debugging

### Check Embedded Assets

```bash
# Build server in verbose mode
cargo build --bin server --verbose

# Look for:
# "cargo:warning=Building admin-spa..."
# "cargo:warning=admin-spa built successfully"
# "cargo:warning=Building web-app..."
# "cargo:warning=web-app built successfully"
```

### Verify Routes

```bash
# Test web app (should return HTML)
curl http://localhost:8080/

# Test admin panel (should return HTML)
curl http://localhost:8080/admin

# Test API (should return GraphQL playground)
curl http://localhost:8080/graphql
```

### Common Issues

**"404 Not Found" on admin or web app:**
- Check that `dist/` folders exist in both packages
- Rebuild frontends: `yarn build`
- Rebuild server: `cargo build`

**Frontend not updating:**
- Clear dist folders and rebuild
- Restart Docker container
- Check build.rs warnings

**Assets not loading:**
- Check browser console for 404s
- Verify Vite `base` path in vite.config.ts
- Admin: `base: '/admin/'`
- Web app: `base: '/'` (default)

## Performance Notes

- Embedded assets are served from memory (very fast)
- Gzip compression applied automatically by tower-http
- No disk I/O on each request
- Cache headers set appropriately
- SPA fallback adds negligible overhead
