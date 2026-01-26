# CI/CD Not Working - Quick Fix Guide

## Why CI/CD Might Not Be Running

### 1. **GitHub Actions Not Enabled**
**Check:** Go to your repository → Settings → Actions → General
- Ensure "Allow all actions and reusable workflows" is selected
- Make sure Actions are enabled for your repository

### 2. **Workflow File Location**
✅ **Correct:** `.github/workflows/ci.yml` (committed at commit `56623f1`)

### 3. **Common CI/CD Failures**

#### Issue: SQLx Compile-Time Verification
**Symptom:** Backend build fails with "database not found" or SQLx macro errors

**Solution:** Add SQLx offline mode support

Run this in your backend directory:
```bash
cd backend
# Set environment variable for offline mode
$env:SQLX_OFFLINE = "true"

# Or add to Cargo.toml
```

Then add to `backend/Cargo.toml`:
```toml
[dependencies]
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-rustls", "chrono", "uuid", "migrate"], default-features = false }
```

#### Issue: cargo-audit Takes Too Long
**Symptom:** Security audit step times out

**Quick Fix:** The workflow already handles this with `cargo install cargo-audit`

#### Issue: Integration Tests Fail
**Symptom:** Docker compose services don't start properly

**Check:**
- Ensure `docker-compose.yml` is in repository root
- Services have proper health checks

### 4. **How to Check if CI/CD is Running**

Visit: `https://github.com/Brianmatovu511/SoundSense/actions`

You should see:
- Workflow runs for each push
- Green checkmarks ✓ for passing
- Red X for failures
- Yellow circle for in-progress

### 5. **Manual Trigger**

If automatic triggers aren't working:

1. Go to: `https://github.com/Brianmatovu511/SoundSense/actions`
2. Click "CI/CD Pipeline" on the left
3. Click "Run workflow" button
4. Select branch: `main`
5. Click green "Run workflow" button

### 6. **View Build Logs**

To see why it's failing:
1. Go to Actions tab
2. Click on a failed workflow run
3. Click on the failed job (e.g., "Backend")
4. Expand the step that failed
5. Read the error message

### 7. **Expected Workflow Steps**

Your CI/CD should run:
1. ✓ Security Scanning (Trivy)
2. ✓ Backend (fmt, clippy, build, test, coverage)
3. ✓ ML Service (lint, test, coverage)
4. ✓ Integration Tests (docker-compose)
5. ✓ Docker Build & Push
6. ✓ Deploy to Staging (if on main branch)

### 8. **Quick Fixes**

#### If Backend Build Fails:
```bash
# Test locally first
cd backend
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-features
```

#### If ML Service Fails:
```bash
cd ml-service
black --check .
flake8 . --count --select=E9,F63,F7,F82 --show-source --statistics
pytest --cov=. --cov-report=xml
```

#### If Integration Tests Fail:
```bash
docker-compose up -d
sleep 30
curl http://localhost:8080/healthz
curl http://localhost:8000/health
docker-compose down
```

### 9. **Disable Failing Steps Temporarily**

If a specific step keeps failing, you can comment it out in `.github/workflows/ci.yml`:

```yaml
# - name: Security audit
#   run: |
#     cargo install cargo-audit
#     cargo audit
```

### 10. **Check Repository Secrets**

Some steps might need secrets:
- Go to Settings → Secrets and variables → Actions
- Ensure `GITHUB_TOKEN` is available (auto-provided)
- For Codecov, you might need `CODECOV_TOKEN` (optional)

## Most Likely Issue

Based on our audit module using SQLx, the **most likely issue** is SQLx trying to connect to a database during compilation.

**Quick Fix:**
1. The workflow already sets up PostgreSQL service
2. Backend tests use `DATABASE_URL` environment variable
3. Should work as-is

**If still failing:** Check the Actions tab logs for the specific error message.

## Still Not Working?

1. **Check Actions tab:** `https://github.com/Brianmatovu511/SoundSense/actions`
2. **Look for error messages** in failed job logs
3. **Share the error** - I can help fix it!

## Testing Workflow Locally

You can test using `act` (GitHub Actions locally):
```bash
# Install act
choco install act

# Run workflow
act push
```

This will run the CI/CD pipeline on your local machine to debug issues.
