# Version Management

This project uses a centralized version management system to keep versions synchronized across all components.

## How It Works

### Single Source of Truth
The version is defined **once** in the root `Cargo.toml`:

```toml
[workspace.package]
version = "0.1.4"  # ← Only place to update version
```

### Automatic Inheritance

1. **Root crate** (`Cargo.toml`):
   ```toml
   [package]
   version.workspace = true  # Inherits from workspace
   ```

2. **Python bindings** (`python/Cargo.toml`):
   ```toml
   [package]
   version.workspace = true  # Inherits from workspace
   ```

3. **Python package** (`python/pyproject.toml`):
   ```toml
   [project]
   dynamic = ["version"]  # Maturin reads from Cargo.toml
   ```

## Updating the Version

### Option 1: Manual Update (Recommended)
Edit the root `Cargo.toml` and change the workspace version:

```toml
[workspace.package]
version = "0.1.5"  # Update this line
```

That's it! All other files will automatically use this version.

### Option 2: Using the Script
Run the provided script:

```bash
./update-version.sh 0.1.5
```

## Verification

To verify all versions are in sync:

```bash
# Check Rust crates
cargo metadata --format-version 1 | jq '.packages[] | select(.name == "fast-uuid-v7" or .name == "fastuuidv7") | {name, version}'

# Check Python package (requires maturin)
cd python && maturin develop && python -c "import fastuuidv7; print(fastuuidv7.__version__)"
```

## Benefits

✅ **Single source of truth** - Update version in one place  
✅ **No manual synchronization** - Automatic inheritance  
✅ **Prevents version drift** - Impossible to have mismatched versions  
✅ **Workspace isolation** - Dependencies remain separate per crate  

## Important Notes

- The workspace setup **only shares metadata** (version, edition, license)
- Each crate maintains its **own dependencies** independently
- The main `fast-uuid-v7` crate does **not** depend on `pyo3`
- The `python/fastuuidv7` crate has its own `pyo3` dependency

## Release Checklist

When releasing a new version:

1. ✅ Update version in root `Cargo.toml` (workspace.package.version)
2. ✅ Update CHANGELOG.md with release notes
3. ✅ Commit changes: `git commit -am "Bump version to X.Y.Z"`
4. ✅ Create tag: `git tag vX.Y.Z`
5. ✅ Push: `git push origin main --tags`
6. ✅ Create GitHub release (triggers automatic PyPI publish)

The version will automatically propagate to:
- Root Rust crate
- Python bindings Rust crate
- Python package on PyPI