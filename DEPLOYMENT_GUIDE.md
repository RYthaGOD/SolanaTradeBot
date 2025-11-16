# 🚀 Deployment Guide for Solana Trade Bot

## Overview

This is a full-stack application with two components:
- **Backend**: Rust-based trading engine (cannot run on Vercel)
- **Frontend**: React/Vite dashboard (can run on Vercel)

## ⚠️ Important: Why Vercel Shows 404

Vercel is designed for **frontend applications and Node.js serverless functions only**. Your Rust backend **cannot run on Vercel** because:

1. ✅ Vercel supports: Static sites, Next.js, Node.js serverless functions
2. ❌ Vercel does NOT support: Rust applications, long-running servers, WebSocket servers

**The 404 error occurs because:**
- Your project root has no `package.json` (Vercel looks for this)
- The frontend is in a subdirectory (`/frontend`)
- The Rust backend needs a separate hosting platform

## ✅ Recommended Deployment Architecture

### Option 1: Separate Hosting (Recommended)

**Backend (Rust):** Deploy to platforms that support Rust:
- **Fly.io** (recommended - supports Rust, PostgreSQL, Redis)
- **Railway.app** (easy Rust deployment)
- **DigitalOcean App Platform** (supports Docker)
- **AWS ECS/EC2** (more control, higher cost)
- **Google Cloud Run** (supports Docker)

**Frontend (React):** Deploy to Vercel
- Fast CDN delivery
- Automatic HTTPS
- Environment variables for backend API URL

### Option 2: All-in-One Docker Deployment

Deploy both backend and frontend together:
- **Fly.io** (recommended - Rust-friendly)
- **Railway.app** (simple configuration)
- **DigitalOcean App Platform**
- **Render.com** (free tier available)

## 🛠️ Step-by-Step Deployment

### Backend Deployment (Fly.io - Recommended)

1. **Install Fly CLI**
```bash
curl -L https://fly.io/install.sh | sh
```

2. **Login to Fly.io**
```bash
fly auth login
```

3. **Create Fly.toml in backend directory**
```bash
cd backend
```

Create `backend/fly.toml`:
```toml
app = "solana-tradebot-backend"
primary_region = "iad"

[build]
  [build.args]
    RUST_VERSION = "1.75"

[env]
  PORT = "8080"
  RUST_LOG = "info"

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = false
  auto_start_machines = true
  min_machines_running = 1

[[services]]
  protocol = "tcp"
  internal_port = 8080
  processes = ["app"]

  [[services.ports]]
    port = 80
    handlers = ["http"]
    force_https = true

  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]

  [services.concurrency]
    type = "connections"
    hard_limit = 25
    soft_limit = 20

[[vm]]
  cpu_kind = "shared"
  cpus = 1
  memory_mb = 512
```

4. **Create Dockerfile in backend directory**

Create `backend/Dockerfile`:
```dockerfile
# Build stage
FROM rust:1.75 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/solana_tradebot /app/
COPY .env.example .env

ENV PORT=8080
EXPOSE 8080

CMD ["/app/solana_tradebot"]
```

5. **Deploy Backend**
```bash
cd backend
fly launch --no-deploy
fly secrets set WALLET_PASSWORD=your_secure_password_here
fly secrets set ENABLE_PAPER_TRADING=true
fly secrets set ENABLE_TRADING=false
fly deploy
```

6. **Get Backend URL**
```bash
fly status
# Note the URL: https://solana-tradebot-backend.fly.dev
```

### Frontend Deployment (Vercel)

1. **Update Frontend API URL**

Create `frontend/.env.production`:
```env
VITE_API_URL=https://solana-tradebot-backend.fly.dev
```

2. **Update Frontend Code to Use Environment Variable**

Edit `frontend/src/App.tsx` or wherever API calls are made:
```typescript
const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:8080';
```

3. **Add Build Script**

Add to `package.json`:
```json
{
  "scripts": {
    "build": "tsc && vite build",
    "vercel-build": "npm run build"
  }
}
```

4. **Deploy to Vercel**

**Option A: Via Vercel Dashboard**
- Go to https://vercel.com/new
- Import your GitHub repository
- Set root directory to `frontend`
- Add environment variable: `VITE_API_URL=https://your-backend-url.fly.dev`
- Deploy

**Option B: Via Vercel CLI**
```bash
npm install -g vercel
cd frontend
vercel --prod
```

5. **Configure vercel.json in frontend directory**

Create `frontend/vercel.json`:
```json
{
  "rewrites": [
    {
      "source": "/(.*)",
      "destination": "/index.html"
    }
  ],
  "headers": [
    {
      "source": "/api/(.*)",
      "headers": [
        {
          "key": "Access-Control-Allow-Origin",
          "value": "*"
        }
      ]
    }
  ]
}
```

## 🔧 Alternative: Deploy Everything Together

### Using Railway.app (Easiest)

1. **Install Railway CLI**
```bash
npm install -g @railway/cli
```

2. **Login**
```bash
railway login
```

3. **Create railway.json in root**
```json
{
  "build": {
    "builder": "DOCKERFILE",
    "dockerfilePath": "Dockerfile"
  },
  "deploy": {
    "startCommand": "cd backend && ./target/release/solana_tradebot",
    "restartPolicyType": "ON_FAILURE"
  }
}
```

4. **Create Root Dockerfile**
```dockerfile
# Build backend
FROM rust:1.75 as backend-builder
WORKDIR /app/backend
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
RUN cargo build --release

# Build frontend
FROM node:18 as frontend-builder
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm install
COPY frontend/ ./
RUN npm run build

# Final runtime
FROM nginx:alpine
COPY --from=backend-builder /app/backend/target/release/solana_tradebot /app/backend
COPY --from=frontend-builder /app/frontend/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 80
CMD nginx && /app/backend
```

5. **Deploy**
```bash
railway up
```

## 🔐 Environment Variables

Set these in your hosting platform:

### Backend Required
```env
WALLET_PASSWORD=your_secure_password
ENABLE_PAPER_TRADING=true
ENABLE_TRADING=false
SOLANA_NETWORK=devnet
RPC_URL=https://api.devnet.solana.com
```

### Frontend Required
```env
VITE_API_URL=https://your-backend-url.com
```

## 🧪 Testing Deployment

After deployment:

1. **Test Backend**
```bash
curl https://your-backend-url.com/health
# Expected: {"status":"ok"}
```

2. **Test Frontend**
```bash
curl https://your-frontend-url.vercel.app
# Should return HTML
```

3. **Test Full Integration**
- Visit your frontend URL
- Open browser console
- Check for API errors
- Verify data loads

## 📊 Cost Estimates

### Fly.io (Backend)
- Free tier: 3 shared-cpu-1x, 256MB VMs (enough for testing)
- Paid: ~$5-10/month for production

### Vercel (Frontend)
- Free tier: Unlimited bandwidth for personal projects
- Hobby: Free for non-commercial
- Pro: $20/month per member for teams

### Railway.app (All-in-One)
- Free tier: $5 credit/month
- Paid: ~$10-20/month depending on usage

## 🔍 Troubleshooting

### "404 on Vercel"
- ✅ Deploy only frontend to Vercel
- ✅ Deploy backend to Fly.io/Railway
- ✅ Update frontend API URL to backend URL

### "CORS Errors"
Add to `backend/src/api.rs`:
```rust
.with(warp::cors()
    .allow_any_origin()
    .allow_methods(vec!["GET", "POST", "PUT", "DELETE"])
    .allow_headers(vec!["Content-Type"]))
```

### "Backend Not Responding"
- Check backend logs: `fly logs` or `railway logs`
- Verify environment variables are set
- Check firewall rules

### "Frontend Can't Connect to Backend"
- Verify `VITE_API_URL` is set correctly
- Check CORS headers in backend
- Verify backend is running: `curl https://backend-url.com/health`

## 📚 Recommended Reading

- [Fly.io Rust Guide](https://fly.io/docs/languages-and-frameworks/rust/)
- [Railway.app Deployment](https://docs.railway.app/deploy/deployments)
- [Vercel Environment Variables](https://vercel.com/docs/concepts/projects/environment-variables)

## 🎯 Quick Fix for Current Issue

**Right now, to fix the 404:**

1. Delete the Vercel deployment
2. Redeploy with root directory set to `frontend`
3. Or follow the backend deployment guide above first

**The application requires TWO deployments:**
- Backend: Fly.io, Railway, or similar (Rust support)
- Frontend: Vercel (React/Vite)

They communicate via HTTP API calls from frontend to backend.
