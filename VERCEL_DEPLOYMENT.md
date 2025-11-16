# Quick Fix: Deploying to Vercel

## The Problem

Your current Vercel deployment at https://solana-trade-bot-two.vercel.app/ shows a 404 error because:

1. **Vercel cannot run Rust applications** - Your backend is written in Rust
2. **Project structure issue** - Vercel is looking at the root directory, but your frontend is in `/frontend`
3. **Missing backend** - The frontend needs a backend API to function

## Quick Solution

### Step 1: Deploy Backend First (Required)

Your Rust backend **must** be deployed to a Rust-compatible platform. Choose one:

**Option A: Fly.io (Recommended - Free tier available)**
```bash
# Install Fly CLI
curl -L https://fly.io/install.sh | sh

# Login
fly auth login

# Navigate to backend
cd backend

# Create fly.toml (see DEPLOYMENT_GUIDE.md for content)
# Create Dockerfile (see DEPLOYMENT_GUIDE.md for content)

# Deploy
fly launch --no-deploy
fly secrets set WALLET_PASSWORD=your_secure_password
fly secrets set ENABLE_PAPER_TRADING=true
fly deploy

# Get your backend URL (e.g., https://solana-tradebot-backend.fly.dev)
fly status
```

**Option B: Railway.app (Also free tier)**
```bash
npm install -g @railway/cli
railway login
cd backend
railway up
# Get your backend URL from Railway dashboard
```

### Step 2: Update Frontend Configuration

After deploying your backend, update the frontend with your backend URL:

1. Go to your Vercel dashboard: https://vercel.com/dashboard
2. Select your project: `solana-trade-bot-two`
3. Go to **Settings** → **Environment Variables**
4. Add this variable:
   - **Name**: `VITE_API_URL`
   - **Value**: `https://your-backend-url.fly.dev` (use your actual backend URL)
5. Save and redeploy

### Step 3: Reconfigure Vercel Deployment

**Option A: Via Vercel Dashboard**

1. Go to your project settings
2. Navigate to **General** → **Build & Development Settings**
3. Set **Root Directory** to `frontend`
4. Set **Build Command** to `npm run build`
5. Set **Output Directory** to `dist`
6. Save and redeploy

**Option B: Delete and Redeploy**

1. Delete current deployment from Vercel dashboard
2. Create new deployment:
   - Import your GitHub repository
   - Set **Root Directory** to `frontend`
   - Add environment variable: `VITE_API_URL` = `https://your-backend-url.fly.dev`
   - Deploy

### Step 4: Verify Deployment

Once both are deployed:

```bash
# Test backend
curl https://your-backend-url.fly.dev/health
# Expected: {"status":"ok"}

# Test frontend
open https://solana-trade-bot-two.vercel.app
# Should load the dashboard
```

## Alternative: Deploy Everything to One Platform

If you want to avoid Vercel entirely:

### Railway.app (Simplest)

Railway can host both your Rust backend and React frontend together:

```bash
npm install -g @railway/cli
railway login
railway up
```

Railway will automatically detect your project structure and deploy both components.

### Fly.io with Docker

Create a multi-stage Dockerfile that builds both frontend and backend (see DEPLOYMENT_GUIDE.md for details).

## Current Project Status

✅ **Backend code is production-ready** - Just needs deployment
✅ **Frontend code is configured** - Will work once backend URL is set
⚠️ **Vercel can only host frontend** - Backend must be deployed elsewhere

## Files Added for Deployment

1. ✅ `frontend/vercel.json` - Vercel configuration
2. ✅ `frontend/.env.production` - Production environment variables (update with your backend URL)
3. ✅ `frontend/src/config.ts` - API configuration (already using environment variables)
4. ✅ All components updated to use `API_BASE_URL` from config

## Next Steps

1. **Deploy backend to Fly.io or Railway** (15 minutes)
2. **Get your backend URL**
3. **Update VITE_API_URL in Vercel environment variables**
4. **Reconfigure Vercel root directory to `frontend`**
5. **Redeploy on Vercel**

Your app will then be fully functional! 🚀

## Need Help?

See the comprehensive deployment guide: [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md)

## Cost

- **Fly.io Backend**: Free tier (3 shared VMs) or ~$5/month
- **Vercel Frontend**: Free tier (unlimited bandwidth for personal projects)
- **Total**: $0-5/month for testing, ~$10-20/month for production
