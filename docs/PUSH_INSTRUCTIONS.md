# Push Instructions for Both Repositories

## ✅ Repository Setup Complete!

You now have two repositories properly configured:

### 1. **swictation-env** (Private Environment)
Location: `/opt/swictation-env`
Remote: `https://github.com/robertelee78/swictation-env.git`
Status: ✅ Initial commit created

### 2. **swictation** (Main Project)
Location: `/opt/swictation`
Remote: Already configured
Status: ✅ Committed with .claude/ removed and symlinked

---

## 🚀 Push Commands

### Step 1: Push swictation-env (First Time)

```bash
cd /opt/swictation-env

# You'll need to authenticate (use personal access token or SSH)
git push -u origin master

# If you want to rename master to main:
git branch -M main
git push -u origin main
```

**Authentication Options:**
- **HTTPS**: Use GitHub Personal Access Token as password
- **SSH**: Set up SSH keys (recommended)

### Step 2: Push swictation (Main Repo)

```bash
cd /opt/swictation

git push origin main
```

---

## 🔐 GitHub Authentication Setup

### Option A: Personal Access Token (HTTPS)

1. Go to GitHub → Settings → Developer settings → Personal access tokens → Tokens (classic)
2. Generate new token with `repo` scope
3. Use token as password when prompted

### Option B: SSH Keys (Recommended)

```bash
# Generate SSH key (if you don't have one)
ssh-keygen -t ed25519 -C "your_email@example.com"

# Add to ssh-agent
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519

# Copy public key
cat ~/.ssh/id_ed25519.pub

# Add to GitHub → Settings → SSH and GPG keys
```

Then update remote URLs:
```bash
cd /opt/swictation-env
git remote set-url origin git@github.com:robertelee78/swictation-env.git

cd /opt/swictation
git remote set-url origin git@github.com:robertelee78/swictation.git
```

---

## 📦 What Was Committed?

### swictation-env (202 files, ~47KB)
```
✅ .claude/agents/           (54 agent definitions)
✅ .claude/commands/         (Custom slash commands)
✅ .claude/skills/           (27 reusable skills)
✅ .claude/helpers/          (Automation scripts)
✅ .claude/settings.json     (Global settings)
✅ README.md                 (Documentation)
```

### swictation (Main changes)
```
✅ .gitignore updated        (Comprehensive patterns)
✅ .claude/ → symlink        (Points to swictation-env)
✅ tests/test_canary_chunked.py  (New)
✅ tests/test_canary_vad.py      (New)
✅ All 189 .claude/ files removed from tracking
```

---

## 🔄 Future Workflow

### Working on Environment Configuration
```bash
cd /opt/swictation-env
# Make changes to agents, commands, skills
git add .
git commit -m "Update agent configurations"
git push origin master  # or main
```

### Working on Swictation Code
```bash
cd /opt/swictation
# Code changes automatically use .claude/ via symlink
git add .
git commit -m "Implement new feature"
git push origin main
```

---

## ⚠️ Important Notes

1. **Keep swictation-env Private**: Contains your custom agent strategies
2. **Symlink is Local**: Not tracked in git (others will need to set up their own)
3. **Environment Changes**: Commit to swictation-env, not swictation
4. **Both Repos**: Keep both in `/opt/` for symlink to work

---

## 🎯 Verification

After pushing, verify:

```bash
# Check swictation-env on GitHub
https://github.com/robertelee78/swictation-env

# Check swictation
https://github.com/robertelee78/swictation
```

Both should show recent commits with proper separation!
