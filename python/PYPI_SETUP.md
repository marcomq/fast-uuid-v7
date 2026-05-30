# PyPI Trusted Publishing Setup Guide

This guide explains how to set up Trusted Publishing for the `fastuuidv7` Python package on PyPI and TestPyPI.

## What is Trusted Publishing?

Trusted Publishing is a secure authentication method that uses OpenID Connect (OIDC) to allow GitHub Actions to publish packages to PyPI without using API tokens. It's more secure because:

- ✅ No long-lived API tokens to manage or leak
- ✅ Automatic authentication via GitHub's identity
- ✅ Scoped to specific repositories and workflows
- ✅ Recommended by PyPI as best practice

## Prerequisites

1. A PyPI account (create at https://pypi.org/account/register/)
2. A TestPyPI account (create at https://test.pypi.org/account/register/)
3. Admin access to your GitHub repository

## Step-by-Step Setup

### 1. Register Your Package Name (First Time Only)

Before setting up Trusted Publishing, you need to register your package name on PyPI.

**Option A: Manual Registration (Recommended for first release)**
1. Build your package locally:
   ```bash
   cd python
   pip install maturin
   maturin build --release
   ```

2. Upload manually using twine:
   ```bash
   pip install twine
   twine upload target/wheels/*.whl
   ```
   - Enter your PyPI username and password when prompted
   - This creates the package on PyPI

**Option B: Use Pending Publisher (Easier)**
- You can configure Trusted Publishing BEFORE the first release
- PyPI will hold the configuration as "pending" until first upload
- See Step 2 below

### 2. Configure Trusted Publishing on PyPI

#### For PyPI (Production):

1. **Log in to PyPI**: Go to https://pypi.org and sign in

2. **Navigate to Publishing Settings**:
   - If package exists: Go to your package page → "Manage" → "Publishing"
   - If package doesn't exist yet: Go to your account → "Publishing" → "Add a new pending publisher"

3. **Add GitHub Publisher**:
   - Click "Add a new publisher"
   - Fill in the form:
     - **PyPI Project Name**: `fastuuidv7`
     - **Owner**: `marcomq` (your GitHub username/org)
     - **Repository name**: `fast-uuid-v7`
     - **Workflow name**: `publish-python.yml`
     - **Environment name**: `pypi`
   - Click "Add"

4. **Verify Configuration**:
   - You should see the publisher listed with status "Active" or "Pending"

#### For TestPyPI (Testing):

Repeat the same process on TestPyPI:

1. **Log in to TestPyPI**: Go to https://test.pypi.org and sign in

2. **Navigate to Publishing Settings**:
   - Go to your account → "Publishing" → "Add a new pending publisher"

3. **Add GitHub Publisher**:
   - Fill in the form:
     - **PyPI Project Name**: `fastuuidv7`
     - **Owner**: `marcomq`
     - **Repository name**: `fast-uuid-v7`
     - **Workflow name**: `publish-python.yml`
     - **Environment name**: `testpypi`
   - Click "Add"

### 3. Configure GitHub Environments

GitHub Environments provide an additional security layer and allow you to require approvals.

1. **Go to Repository Settings**:
   - Navigate to your GitHub repository
   - Click "Settings" → "Environments"

2. **Create PyPI Environment**:
   - Click "New environment"
   - Name: `pypi`
   - (Optional) Add protection rules:
     - ✅ Required reviewers: Add yourself or team members
     - ✅ Wait timer: Add a delay before deployment
     - ✅ Deployment branches: Limit to specific branches
   - Click "Save protection rules"

3. **Create TestPyPI Environment**:
   - Click "New environment"
   - Name: `testpypi`
   - (Optional) Add protection rules (usually less strict than production)
   - Click "Save protection rules"

### 4. Test the Setup

#### Test with TestPyPI First:

1. **Trigger Manual Workflow**:
   - Go to "Actions" tab in your GitHub repository
   - Select "Publish Python Package" workflow
   - Click "Run workflow"
   - Select branch: `main`
   - Select target: `testpypi`
   - Click "Run workflow"

2. **Monitor the Build**:
   - Watch the workflow execution
   - All 8 platform builds should complete
   - Check the publish step succeeds

3. **Verify on TestPyPI**:
   - Visit https://test.pypi.org/project/fastuuidv7/
   - Confirm your package appears with all wheels

4. **Test Installation**:
   ```bash
   pip install --index-url https://test.pypi.org/simple/ fastuuidv7
   python -c "import fastuuidv7; print(fastuuidv7.gen_id_str())"
   ```

#### Production Release:

1. **Create a Git Tag**:
   ```bash
   git tag v0.1.3
   git push origin v0.1.3
   ```

2. **Create GitHub Release**:
   - Go to "Releases" → "Create a new release"
   - Choose the tag you just created
   - Add release notes
   - Click "Publish release"

3. **Automatic Publishing**:
   - The workflow triggers automatically
   - Builds wheels for all platforms
   - Publishes to PyPI
   - No manual intervention needed!

4. **Verify on PyPI**:
   - Visit https://pypi.org/project/fastuuidv7/
   - Confirm all wheels are available

## Troubleshooting

### "Trusted publishing exchange failure"

**Cause**: Mismatch between PyPI configuration and workflow settings

**Solution**: Double-check that:
- Repository owner/name matches exactly
- Workflow filename is correct: `publish-python.yml`
- Environment name matches: `pypi` or `testpypi`
- The workflow is running from the correct branch

### "Package name already exists"

**Cause**: Someone else registered the package name

**Solution**: 
- Choose a different package name
- Update `python/pyproject.toml` with the new name
- Update PyPI trusted publishing configuration

### "Environment protection rules failed"

**Cause**: Required reviewers haven't approved

**Solution**:
- Check the workflow run for pending approvals
- Approve the deployment in the GitHub UI
- Or remove protection rules for testing

### Builds fail for specific platforms

**Cause**: Platform-specific compilation issues

**Solution**:
- Check the build logs for that platform
- The workflow uses `fail-fast: false`, so other platforms still build
- You can temporarily remove problematic platforms from the matrix

### Windows ARM64 not supported

**Note**: Windows ARM64 (aarch64-pc-windows-msvc) builds are currently disabled in the workflow.

**Reason**: Cross-compiling Python extensions for Windows ARM64 is not well supported due to:
- Lack of Python development libraries (python3X.lib) for ARM64 in CI environments
- Limited toolchain support for cross-compilation to Windows ARM64
- PyO3's extension module linking requires native Python libraries for the target platform

**Workaround**: If Windows ARM64 support is needed:
- Build natively on a Windows ARM64 machine
- Use emulation/virtualization with native ARM64 Windows
- Wait for improved CI toolchain support for this platform

## Security Best Practices

1. ✅ **Use Trusted Publishing** - Never use long-lived API tokens
2. ✅ **Enable Environment Protection** - Require approvals for production
3. ✅ **Test on TestPyPI First** - Always test before production release
4. ✅ **Review Workflow Changes** - Audit any changes to `.github/workflows/publish-python.yml`
5. ✅ **Monitor Releases** - Set up notifications for new releases

## Additional Resources

- [PyPI Trusted Publishing Guide](https://docs.pypi.org/trusted-publishers/)
- [GitHub Actions OIDC](https://docs.github.com/en/actions/deployment/security-hardening-your-deployments/about-security-hardening-with-openid-connect)
- [Maturin Documentation](https://www.maturin.rs/)
- [PyO3 Guide](https://pyo3.rs/)

## Summary

Once configured, your release process is:

1. **Development**: Make changes, test locally
2. **Tag**: Create and push a version tag
3. **Release**: Create GitHub release
4. **Automatic**: Workflow builds and publishes to PyPI
5. **Done**: Package available on PyPI within minutes!

No API tokens, no manual uploads, fully automated! 🚀