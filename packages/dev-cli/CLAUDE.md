# Dev CLI Guidelines

## No Magic Strings

**CRITICAL:** Never use hardcoded paths or magic strings. Always use the configuration system.

### Incorrect Pattern (DO NOT DO THIS)

```rust
// WRONG: Hardcoded path
if file_path.starts_with("packages/api-core/") {
    // ...
}

// WRONG: Magic string for package directory
let path = format!("packages/{}/", pkg_name);
```

### Correct Pattern

```rust
// CORRECT: Use Config to get package information
if let Some(pkg) = self.packages.get(pkg_name) {
    let pkg_path = pkg.path.strip_prefix(&self.repo_root).unwrap_or(&pkg.path);
    // Use pkg_path...
}

// CORRECT: Use workspaces config for patterns
for pattern in &config.global.workspaces.packages {
    // ...
}
```

### Where Configuration Lives

- **Global settings**: `.dev/config.toml` → `Config.global`
- **Package info**: Auto-discovered → `Config.packages`
- **Package paths**: `PackageConfig.path`
- **Workspace patterns**: `Config.global.workspaces.packages`

### Before Adding a New Hardcoded Value

Ask yourself:

- [ ] Is this value already in Config?
- [ ] Should this be configurable in `.dev/config.toml`?
- [ ] Can I derive this from existing PackageConfig fields?

If the answer is "yes" to any of these, use the config system instead.

## Package Commands System

Packages define their own commands in `dev.toml` files. The CLI discovers and runs these commands.

### dev.toml Command Syntax

```toml
# Simple commands (shorthand)
[cmd]
test = "npx jest"

# Commands with variants and dependencies
[cmd.build]
default = "npx tsc"
watch = "npx tsc --watch"

[cmd.lint]
default = "npx eslint src"
fix = "npx eslint src --fix"

[cmd.typecheck]
default = "npx tsc --noEmit"
deps = ["common:build"]  # Run common:build first
```

### Running Commands

```bash
# Build is a top-level command (routes to [cmd.build])
./dev.sh build                   # Build all packages
./dev.sh build api-server        # Build specific package
./dev.sh build --release         # Build with release variant
./dev.sh build --watch           # Build with watch variant

# Other commands use the cmd subcommand
./dev.sh cmd --list              # List all available commands
./dev.sh cmd typecheck           # Run typecheck (respects deps)
./dev.sh cmd lint:fix            # Run lint with fix variant
```

### Implementation

- `CmdEntry` enum handles both simple strings and full config
- `CmdConfig` uses custom deserializer to capture arbitrary variants
- Dependency resolution with topological sort
- Parallel execution with thread spawning
