# GitHub Actions Setup Guide

This guide explains how to set up GitHub Actions CI/CD for automated building and publishing of Swictation packages.

## Table of Contents

1. [Creating NPM_TOKEN](#creating-npm_token)
2. [Adding Secrets to GitHub](#adding-secrets-to-github)
3. [Configuring Workflow Permissions](#configuring-workflow-permissions)
4. [Manually Triggering Workflows](#manually-triggering-workflows)
5. [Debugging Failed Builds](#debugging-failed-builds)
6. [Cost Considerations](#cost-considerations)

## Creating NPM_TOKEN

The `NPM_TOKEN` is required for automated npm publishing from GitHub Actions.

### Step 1: Generate NPM Token

1. **Log in to npmjs.com**
   - Go to https://www.npmjs.com and sign in with your account

2. **Navigate to Access Tokens**
   - Click your profile avatar (top right)
   - Select "Access Tokens" from the dropdown

3. **Generate New Token**
   - Click "Generate New Token" → "Classic Token"
   - Choose token type: **Automation** (for CI/CD)
   - Name it: `GitHub Actions - Swictation CI/CD`

4. **Set Permissions**
   - **Read and Publish** - Required for `npm publish` commands
   - **Note**: "Automation" tokens are designed for CI/CD and bypass 2FA

5. **Copy and Save**
   - **IMPORTANT**: Copy the token immediately
   - Token format: `npm_xxxxxxxxxxxxxxxxxxxxx`
   - You won't be able to see it again after closing this page
   - Store it securely (you'll add it to GitHub in the next step)

### Token Types Comparison

| Token Type | Use Case | 2FA Bypass | Expires |
|------------|----------|------------|---------|
| **Automation** | CI/CD pipelines | ✅ Yes | Never |
| **Publish** | Manual publishing | ❌ No | Never |
| **Read-only** | Private package installs | N/A | Never |

**For GitHub Actions, always use "Automation" tokens.**

### Security Best Practices

- ✅ Use automation tokens for CI/CD only
- ✅ Store tokens as GitHub repository secrets (never commit them)
- ✅ Regularly rotate tokens (every 6-12 months)
- ✅ Revoke old tokens after rotation
- ❌ Never share tokens or commit them to git
- ❌ Never use publish tokens in CI/CD (they require 2FA)

## Adding Secrets to GitHub

GitHub Secrets allow you to securely store sensitive information like npm tokens.

### Step 1: Navigate to Repository Settings

1. Go to your repository on GitHub
2. Click **Settings** (top navigation bar)
3. In left sidebar, go to **Secrets and variables** → **Actions**

### Step 2: Add NPM_TOKEN Secret

1. Click **New repository secret**
2. Fill in the form:
   - **Name**: `NPM_TOKEN` (must match exactly what workflows expect)
   - **Secret**: Paste the npm token you copied earlier (starts with `npm_`)
3. Click **Add secret**

### Step 3: Verify Secret Was Added

You should see `NPM_TOKEN` listed under "Repository secrets". The value will be hidden for security.

### Using Secrets in Workflows

In your workflow files, secrets are accessed via `${{ secrets.NPM_TOKEN }}`:

```yaml
- name: Publish to npm
  run: npm publish
  env:
    NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

### Secret Security

- Secrets are encrypted at rest
- Secret values are redacted in GitHub Actions logs
- Only workflows in the same repository can access the secrets
- Secrets are not passed to workflows triggered by forks (security feature)

## Configuring Workflow Permissions

GitHub Actions workflows need proper permissions to read/write code, publish releases, etc.

### Default Permissions

By default, GitHub Actions workflows have **read-only** permissions for security.

### Setting Repository Permissions

1. Go to **Settings** → **Actions** → **General**
2. Scroll to **Workflow permissions**
3. Choose one of:
   - **Read repository contents and packages permissions** (default, most secure)
   - **Read and write permissions** (needed for creating releases, tags)

### Recommended Configuration for Swictation

**Option 1: Least Privilege (Recommended)**
- Keep default **read-only** permissions
- Grant specific permissions per workflow using `permissions:` key

```yaml
# .github/workflows/release.yml
permissions:
  contents: write   # For creating releases
  packages: write   # For publishing packages
```

**Option 2: Repository-Wide Write Access**
- Enable **Read and write permissions** at repository level
- Simpler, but less secure (all workflows get write access)

**We recommend Option 1** - explicitly grant permissions in each workflow file.

### Permissions Reference

| Permission | Purpose | Needed For |
|------------|---------|------------|
| `contents: read` | Read repository code | All workflows (default) |
| `contents: write` | Create releases, tags | Release workflow |
| `packages: write` | Publish GitHub packages | Package publishing |
| `actions: read` | Access workflow runs | Debugging workflows |
| `issues: write` | Create/update issues | Automated issue creation |

## Manually Triggering Workflows

GitHub Actions supports manual workflow triggers via `workflow_dispatch`.

### Workflow Configuration

Add `workflow_dispatch` to your workflow triggers:

```yaml
name: Release
on:
  push:
    tags:
      - 'v*'      # Automatic trigger on version tags
  workflow_dispatch:  # Manual trigger
    inputs:
      version:
        description: 'Version to release (e.g., 0.7.9)'
        required: true
        type: string
      dry_run:
        description: 'Dry run (test without publishing)'
        required: false
        type: boolean
        default: false
```

### Triggering from GitHub UI

1. Go to **Actions** tab in your repository
2. Select the workflow you want to run (e.g., "Release")
3. Click **Run workflow** button (top right)
4. Fill in any required inputs
5. Click **Run workflow** (green button)

### Triggering via GitHub CLI

```bash
# Trigger release workflow
gh workflow run release.yml \
  -f version=0.7.9 \
  -f dry_run=false

# Trigger build workflow for specific branch
gh workflow run build-linux.yml \
  --ref feature-branch

# List recent workflow runs
gh run list --workflow=release.yml
```

### Using Workflow Inputs

Access inputs in workflow steps:

```yaml
- name: Publish packages
  run: node scripts/publish-all.js --tag latest
  if: ${{ !inputs.dry_run }}

- name: Dry run
  run: node scripts/publish-all.js --dry-run
  if: ${{ inputs.dry_run }}
```

## Debugging Failed Builds

When workflows fail, GitHub provides several debugging tools.

### 1. Viewing Logs

**Basic Log Viewing:**
1. Go to **Actions** tab
2. Click on the failed workflow run
3. Click on the failed job
4. Expand the failed step to see logs

**Log Features:**
- Color-coded output (red for errors, yellow for warnings)
- Timestamps for each line
- Expandable/collapsible sections
- Search functionality (Ctrl+F / Cmd+F)

### 2. Enable Debug Logging

Add debug logging for more verbose output:

**Repository-Level:**
1. Go to **Settings** → **Secrets and variables** → **Actions**
2. Add new variables:
   - `ACTIONS_RUNNER_DEBUG` = `true`
   - `ACTIONS_STEP_DEBUG` = `true`

**Workflow-Level:**
```yaml
env:
  ACTIONS_RUNNER_DEBUG: true
  ACTIONS_STEP_DEBUG: true
```

### 3. Common Failure Scenarios

#### NPM Authentication Failed

**Error:**
```
npm ERR! code ENEEDAUTH
npm ERR! need auth This command requires you to be logged in.
```

**Fix:**
- Verify `NPM_TOKEN` secret exists and is correct
- Check token has "Automation" type (not "Publish")
- Ensure workflow has `NODE_AUTH_TOKEN` environment variable:
  ```yaml
  env:
    NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
  ```

#### Version Already Published

**Error:**
```
npm ERR! code EPUBLISHCONFLICT
npm ERR! You cannot publish over the previously published versions
```

**Fix:**
- Bump version with `npm run version:bump`
- Or manually edit `versions.json` and run `npm run version:sync`

#### Platform Package Not Found

**Error:**
```
Error: Platform package @agidreams/linux-x64 not found
```

**Fix:**
- Ensure build-linux.yml workflow completed successfully
- Check that artifact was uploaded
- Verify publish-all.js waits for npm registry propagation

#### macOS Build Failed - Runner Out of Memory

**Error:**
```
error: linking with `cc` failed: exit status: 1
= note: ld: warning: could not create compact unwind
```

**Fix:**
- macOS runners have limited memory (7GB for macos-14)
- Use `--release` flag for optimized builds (smaller memory footprint)
- Consider splitting large crates into smaller ones

### 4. Re-running Failed Jobs

**Re-run All Jobs:**
1. Click "Re-run all jobs" button (top right)
2. Useful when failure was due to transient issue (network, rate limits)

**Re-run Only Failed Jobs:**
1. Click dropdown next to "Re-run all jobs"
2. Select "Re-run failed jobs"
3. Faster and cheaper than re-running everything

### 5. Using `tmate` for Interactive Debugging

For complex debugging, you can SSH into the runner:

```yaml
- name: Setup tmate session
  uses: mxschmitt/action-tmate@v3
  if: failure()  # Only run on failure
```

This provides an interactive SSH session to debug the exact environment.

### 6. Debugging Checklist

When a build fails, check:

- [ ] Logs show clear error message
- [ ] All secrets are properly configured
- [ ] Workflow has required permissions
- [ ] All dependencies are available (Rust, Node.js, etc.)
- [ ] Previous successful runs to compare what changed
- [ ] GitHub Status page for service outages
- [ ] Rate limits (npm, GitHub API)

## Cost Considerations

GitHub Actions provides free minutes for public/private repositories, but costs vary by runner type.

### Runner Pricing

| Runner Type | Minutes Multiplier | Effective Cost | Specs |
|-------------|-------------------|----------------|-------|
| **ubuntu-latest** | 1x | 1 minute = 1 minute | 4 vCPU, 16GB RAM, 14GB SSD |
| **macos-14** (ARM64) | 10x | 1 minute = 10 minutes | 4 vCPU (Apple M1), 7GB RAM |
| **macos-13** (Intel) | 10x | 1 minute = 10 minutes | 4 vCPU (Intel), 14GB RAM |
| **windows-latest** | 2x | 1 minute = 2 minutes | 4 vCPU, 16GB RAM |

**Example:**
- Linux build: 5 minutes actual = 5 minutes charged
- macOS build: 5 minutes actual = **50 minutes charged**
- **Total: 55 minutes charged for 10 minutes of actual build time**

### Free Tier Limits

**Public Repositories:**
- **Unlimited minutes** for public repos (free forever)
- All runner types included

**Private Repositories:**
- **2,000 minutes/month** (free tier)
- After that: $0.008/minute for Linux, $0.08/minute for macOS
- Example: 100 minutes macOS = 1000 billed minutes = $80

### Cost Optimization Strategies

#### 1. Use Linux Runners When Possible

**Before (macOS for everything):**
```yaml
jobs:
  build-all:
    runs-on: macos-14  # 10x cost
    steps:
      - name: Build Linux binaries
        run: cargo build --target x86_64-unknown-linux-gnu
```
Cost: 10 minutes × 10 = **100 minutes charged**

**After (platform-specific):**
```yaml
jobs:
  build-linux:
    runs-on: ubuntu-latest  # 1x cost
  build-macos:
    runs-on: macos-14       # 10x cost (necessary)
```
Cost: (5 minutes × 1) + (5 minutes × 10) = **55 minutes charged**

**Savings: 45% reduction**

#### 2. Cache Dependencies

```yaml
- name: Cache Cargo
  uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/bin/
      ~/.cargo/registry/index/
      ~/.cargo/registry/cache/
      ~/.cargo/git/db/
      target/
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

**Impact:**
- First run: 8 minutes
- Cached run: 2 minutes
- **Savings: 75% time reduction** (especially valuable on macOS)

#### 3. Parallelize Independent Jobs

**Before (sequential):**
```yaml
jobs:
  build-linux:
    runs-on: ubuntu-latest
  build-macos:
    runs-on: macos-14
    needs: build-linux  # Waits for Linux to finish
```
Total time: 5 min (Linux) + 5 min (macOS) = 10 minutes

**After (parallel):**
```yaml
jobs:
  build-linux:
    runs-on: ubuntu-latest
  build-macos:
    runs-on: macos-14  # Runs simultaneously
```
Total time: max(5 min, 5 min) = **5 minutes** (50% faster)

#### 4. Use Conditional Jobs

Only run expensive jobs when necessary:

```yaml
jobs:
  build-macos:
    runs-on: macos-14
    # Only run on:
    # - version tags
    # - changes to macOS-specific code
    if: |
      startsWith(github.ref, 'refs/tags/v') ||
      contains(github.event.head_commit.modified, 'tauri-ui/')
```

#### 5. Use Self-Hosted Runners (Advanced)

For very high usage, consider self-hosted runners:
- **Pros**: No per-minute costs, full control
- **Cons**: Maintenance overhead, security responsibility

### Monthly Cost Estimation

**Swictation CI/CD Usage (estimated):**

| Scenario | Linux Mins | macOS Mins | Total Charged | Monthly Cost |
|----------|-----------|-----------|---------------|--------------|
| **Development** (20 PRs/month) | 100 | 100 | 1,100 mins | $0 (public repo) |
| **Releases** (4 releases/month) | 20 | 20 | 220 mins | $0 (public repo) |
| **Total** | 120 | 120 | 1,320 mins | **$0** |

**If using private repository:**
- Monthly minutes: 1,320
- Free tier: 2,000 minutes
- Overage: 0 minutes
- **Cost: $0/month** (within free tier)

**Heavy usage scenario (private repo):**
- Monthly minutes: 5,000 charged
- Free tier: 2,000 minutes
- Overage: 3,000 minutes
- Breakdown: 300 Linux (3,000 billed mins) at $0.008/min = $24
- **Cost: $24/month**

### Best Practices Summary

1. ✅ **Prefer Linux runners** for platform-agnostic tasks
2. ✅ **Use macOS runners only** for macOS-specific builds
3. ✅ **Cache dependencies** aggressively (Cargo, npm, ONNX Runtime)
4. ✅ **Run jobs in parallel** when possible
5. ✅ **Use conditional execution** to skip unnecessary runs
6. ✅ **Monitor usage** in Settings → Billing → Usage
7. ✅ **Public repos** get unlimited free minutes (if possible, make repo public)

### Monitoring Your Usage

**View current usage:**
1. Go to your GitHub profile → **Settings**
2. Select **Billing and plans**
3. Click **Usage this month**
4. See breakdown by runner type

**Set spending limits:**
1. Billing → **Spending limits**
2. Set maximum monthly spending (e.g., $10/month)
3. Actions will stop when limit is reached

## Additional Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [npm Token Types](https://docs.npmjs.com/creating-and-viewing-access-tokens)
- [GitHub Actions Pricing](https://docs.github.com/en/billing/managing-billing-for-github-actions/about-billing-for-github-actions)
- [Workflow Syntax](https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions)
- [GitHub Status](https://www.githubstatus.com/) - Check for service outages

## Support

If you encounter issues not covered in this guide:

1. Check [GitHub Community Forum](https://github.com/orgs/community/discussions/categories/actions)
2. Review [Swictation GitHub Issues](https://github.com/robertelee78/swictation/issues)
3. Contact repository maintainers

---

**Last Updated:** 2025-11-26
**Maintained By:** Swictation Project
