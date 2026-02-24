# Yarn Modern (v4) Upgrade

Successfully upgraded from Yarn Classic (1.22.22) to Yarn Modern (4.12.0).

## What Changed

### Package Manager
- **Before:** Yarn Classic 1.22.22
- **After:** Yarn Modern 4.12.0 (Berry)

### Benefits
- ✅ **Faster installs** - Improved resolution and caching
- ✅ **Better workspace support** - Enhanced monorepo features
- ✅ **Improved security** - Better dependency verification
- ✅ **Modern features** - Constraints, plugins, and more
- ✅ **Smaller disk usage** - Better dependency deduplication

## Files Updated

### Package Configuration
- `packages/admin-spa/package.json` - Updated `packageManager` field
- `packages/web-app/package.json` - Updated `packageManager` field
- `packages/admin-spa/.yarnrc.yml` - Created (Yarn Modern config)
- `packages/web-app/.yarnrc.yml` - Created (Yarn Modern config)

### Gitignore
- `.gitignore` - Added Yarn Modern cache exclusions

## Usage

All yarn commands remain the same:

```bash
# Install dependencies
yarn install

# Build
yarn build

# Run dev server
yarn dev
```

## Configuration

Each package has a `.yarnrc.yml` file:

```yaml
nodeLinker: node-modules
```

This uses the traditional `node_modules` strategy (compatible with existing tooling).

## Corepack

Yarn version is managed via Corepack (built into Node.js 16.10+):

```bash
# Enable corepack
corepack enable

# Update to latest stable yarn
corepack prepare yarn@stable --activate
```

The `packageManager` field in package.json automatically ensures the correct version is used.

## Docker / Build Scripts

No changes needed! The build scripts (`build.rs`, `dev-watch.sh`) automatically detect and use yarn.

## Compatibility

✅ All existing scripts work
✅ Docker builds work
✅ CI/CD unchanged
✅ IDE integration maintained

## Lockfiles

Yarn Modern uses the same `yarn.lock` format (v1) for compatibility. The lockfile was automatically upgraded during migration.

## Rollback

To rollback to Yarn Classic:

1. Update `packageManager` in package.json:
   ```json
   "packageManager": "yarn@1.22.22"
   ```

2. Remove `.yarnrc.yml` files

3. Run `yarn install`

## Documentation

- [Yarn Modern Migration Guide](https://yarnpkg.com/getting-started/migration)
- [Yarn 4 Documentation](https://yarnpkg.com/)
