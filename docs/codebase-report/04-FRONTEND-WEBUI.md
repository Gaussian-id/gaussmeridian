# 04 — Frontend / Web UI Deep Dive

## Stack Summary

| Aspect     | Choice                                           |
| ---------- | ------------------------------------------------ |
| Framework  | Next.js 16.x (App Router, React 19)              |
| UI Library | shadcn/ui (Radix primitives + Tailwind)          |
| Styling    | Tailwind CSS v4 (CSS-based config, OKLCH tokens) |
| Forms      | React Hook Form + Zod                            |
| Charts     | Recharts                                         |
| Icons      | Lucide React                                     |
| Theming    | next-themes (dark by default)                    |
| State      | React Context (no Redux/Zustand)                 |
| Auth       | JWT in localStorage via React Context            |

**Important:** Despite some docs mentioning Deno/Fresh, the active frontend is **Next.js**. The `routes/` and `deno.json` files are dead legacy code.

---

## Page Routes

### Public Pages

| URL                     | File                                | Description                                                  |
| ----------------------- | ----------------------------------- | ------------------------------------------------------------ |
| `/`                     | `app/page.tsx`                      | Landing page — hero, feature comparison, CTA sections        |
| `/api`                  | `app/api/page.tsx`                  | API keys showcase / marketing page (NOT a Next.js API route) |
| `/auth/login`           | `app/auth/login/page.tsx`           | Login form using `useAuth()`                                 |
| `/auth/signup`          | `app/auth/signup/page.tsx`          | Registration form using `useAuth()`                          |
| `/auth/forgot-password` | `app/auth/forgot-password/page.tsx` | Password reset UI                                            |

### Console (Dashboard) Pages

All under `app/console/` with shared `layout.tsx` (sidebar + nav):

| URL                 | File                            | Description                                             |
| ------------------- | ------------------------------- | ------------------------------------------------------- |
| `/console`          | `app/console/page.tsx`          | Overview dashboard with charts (partly static data)     |
| `/console/usage`    | `app/console/usage/page.tsx`    | Usage analytics using `GaussMeridianClient`               |
| `/console/logs`     | `app/console/logs/page.tsx`     | Request logs table (has `loading.tsx` skeleton)         |
| `/console/api-keys` | `app/console/api-keys/page.tsx` | API key management + tenant enrichment                  |
| `/console/team`     | `app/console/team/page.tsx`     | Team management using direct fetch to `/v1/admin/users` |
| `/console/settings` | `app/console/settings/page.tsx` | System settings via `GaussMeridianClient`                 |

### Other Pages

| URL           | File                      | Description                                                      |
| ------------- | ------------------------- | ---------------------------------------------------------------- |
| `/dashboard`  | `app/dashboard/page.tsx`  | Alternate dashboard (separate from `/console`, static-ish cards) |
| `/onboarding` | `app/onboarding/page.tsx` | Multi-step post-signup onboarding flow                           |
| `/settings`   | `app/settings/page.tsx`   | User settings (separate from `/console/settings`)                |
| `/terminal`   | `app/terminal/page.tsx`   | Mock terminal UI                                                 |

### Special Files

| File                | Purpose                                                                       |
| ------------------- | ----------------------------------------------------------------------------- |
| `app/layout.tsx`    | Root layout: fonts, ThemeProvider, AuthProvider, ToastProvider, ErrorBoundary |
| `app/globals.css`   | Tailwind v4 import, OKLCH design tokens, glass effects, animations            |
| `app/error.tsx`     | Global error boundary                                                         |
| `app/not-found.tsx` | 404 page                                                                      |

---

## Root Layout Providers

```tsx
// app/layout.tsx wraps all pages with:
<ThemeProvider>          // next-themes, dark mode default
  <AuthProvider>         // JWT auth context from lib/auth-context.tsx
    <ToastProvider>      // Custom toast notifications
      <ErrorBoundary>    // Global error catching
        {children}
      </ErrorBoundary>
    </ToastProvider>
  </AuthProvider>
</ThemeProvider>
```

---

## Component Architecture

### UI Primitives (`components/ui/`)

shadcn/ui components generated with `new-york` style variant. Radix-based, styled with Tailwind. Includes:
- Button, Input, Dialog, Sheet, Tabs, Table
- Sidebar, Navigation Menu, Dropdown Menu
- Form (react-hook-form integration), Calendar
- Chart wrapper (Recharts), Card, Badge, Tooltip
- Toast (sonner), Command palette (cmdk)

### Feature Components (`components/`)

| Component                       | Used By                             |
| ------------------------------- | ----------------------------------- |
| `dashboard-sidebar.tsx`         | Console layout — sidebar navigation |
| `dashboard-nav.tsx`             | Console layout — top navigation bar |
| `navigation.tsx`                | Landing page, dashboard             |
| `request-volume-chart.tsx`      | Console overview                    |
| `latency-percentiles-chart.tsx` | Console overview                    |
| `admin-terminal.tsx`            | Terminal page                       |
| `command-palette.tsx`           | Global command palette              |

### Islands (`islands/`)

Client-side "island" modules that provide richer interactive UIs:

| Island                    | Purpose                                  |
| ------------------------- | ---------------------------------------- |
| `console-content.tsx`     | Full console overview with real API data |
| `dashboard-content.tsx`   | Dashboard content with metrics           |
| `api-keys.tsx`            | API key CRUD operations                  |
| `request-logs.tsx`        | Log viewer with filtering                |
| `usage-analytics.tsx`     | Usage charts and breakdowns              |
| `settings-page.tsx`       | Settings management                      |
| `team-management.tsx`     | Team member CRUD                         |
| `provider-management.tsx` | Provider configuration UI                |
| `model-management.tsx`    | Model listing and configuration          |
| `agent-management.tsx`    | MoA agent management                     |
| `analytics-dashboard.tsx` | Detailed analytics views                 |
| `auth-guard.tsx`          | Client-side route protection             |

These overlap with `app/console/*` pages — some pages render islands, creating a two-layer architecture.

---

## API Communication

### GaussMeridianClient (`lib/api-client.ts`)

Singleton HTTP client for talking to the Rust backend:

```typescript
class GaussMeridianClient {
    private baseUrl: string;    // from NEXT_PUBLIC_GAUSSMERIDIAN_API_URL or localhost:8000
    private apiKey?: string;    // from NEXT_PUBLIC_GAUSSMERIDIAN_API_KEY
    private jwtToken?: string;  // set via setJwtToken()
    
    // Auth header priority: JWT Bearer > API Key (x-api-key)
    
    // Methods:
    getModels()
    getHealth()
    getMetrics()              // Parses Prometheus text format
    getRequestLogs(params)
    getUsageAnalytics(params)
    getCostAnalytics(params)
    getProviders()
    getTenants()
    getApiKeys()
    // ... more
}

// Singleton getter:
export function getGaussMeridianClient(): GaussMeridianClient
```

### Auth Functions (`lib/auth.ts`)

Direct fetch helpers for auth operations:

```typescript
loginUser(email, password)    // POST /v1/auth/login
registerUser(data)            // POST /v1/auth/register
getCurrentUser(token)         // GET /v1/auth/me
getUserApiKeys(token)         // GET /v1/api/keys
createUserApiKey(token, data) // POST /v1/api/keys
revokeUserApiKey(token, id)   // POST /v1/api/keys/revoke
```

### MoA Client (`lib/moa-client.ts`)

Separate client for the Mixture of Agents service:

```typescript
// Connects to NEXT_PUBLIC_MOA_API_URL (default: localhost:8081)
// Bearer token from NEXT_PUBLIC_MOA_API_KEY
// Endpoints: /api/v1/process, /api/v1/agents, etc.
```

### Known Issue: JWT Not Wired to Client

`GaussMeridianClient.setJwtToken()` exists but is **never called** after login in the app code. Console pages using `getGaussMeridianClient()` rely on the **API key from env** rather than the user's JWT. The team page is an exception — it passes the token directly via its own `fetch` calls.

---

## Authentication on Frontend

### AuthContext (`lib/auth-context.tsx`)

```typescript
interface AuthContextType {
    user: User | null;
    token: string | null;
    isAuthenticated: boolean;
    isLoading: boolean;
    login(email, password): Promise<void>;
    register(data): Promise<void>;
    logout(): void;
    hasRole(role): boolean;
    hasAnyRole(roles): boolean;
    isAdmin(): boolean;
}
```

### Flow

1. **Login/Register**: POST to `/v1/auth/login` or `/v1/auth/register`
2. **Storage**: JWT and user object saved to `localStorage`:
   - `gaussmeridian_jwt` — the JWT token
   - `gaussmeridian_user` — serialized user object
3. **Session check on load**: Read from `localStorage`, check JWT expiry via `atob(payload)`, optional `GET /v1/auth/me`
4. **Logout**: Clear storage, redirect to `/auth/login`

### Route Protection

- **No Next.js middleware** — there is no `middleware.ts` at the project root
- Console is **NOT** automatically protected at the routing layer
- Protection relies on components using `useAuth()` and redirecting manually, or wrapping with `AuthGuard` / `withAuth` HOC

---

## Styling System

### Tailwind v4 Configuration

All in `app/globals.css` (no `tailwind.config.js`):

```css
@import "tailwindcss";
@import "tw-animate-css";

@theme inline {
  --color-background: oklch(0.145 0 0);      /* Dark background */
  --color-primary: oklch(0.546 0.245 262.88); /* Indigo primary */
  --font-sans: 'Inter', ui-sans-serif, ...;
  --radius-sm: 0.375rem;
  /* ... more tokens */
}
```

### Design Tokens (OKLCH)

Uses **OKLCH** color space for perceptually uniform colors:
- Background, foreground, card, popover, muted, accent
- Chart colors (5 slots)
- Sidebar-specific tokens
- Dark mode as default (`.dark` class variant)

### Notable CSS Features

- Glass morphism effects (`.glass` utility)
- Gradient text utilities
- Custom scrollbar styling
- Page transition animations
- Glow/shimmer effects

---

## Environment Variables

| Variable                            | Required    | Default                 | Purpose                       |
| ----------------------------------- | ----------- | ----------------------- | ----------------------------- |
| `NEXT_PUBLIC_GAUSSMERIDIAN_API_URL` | Yes         | `http://localhost:8000` | Rust API base URL             |
| `NEXT_PUBLIC_GAUSSMERIDIAN_API_KEY`   | Recommended | —                       | Default API key for client    |
| `NEXT_PUBLIC_MOA_API_URL`           | Optional    | `http://localhost:8081` | MoA service URL               |
| `NEXT_PUBLIC_MOA_API_KEY`           | Optional    | —                       | MoA auth token                |
| `NODE_ENV`                          | Auto        | `development`           | Controls error detail logging |

---

## Build Configuration

### `next.config.mjs`

```javascript
{
    typescript: { ignoreBuildErrors: true },  // ⚠️ Hides type errors
    images: { unoptimized: true }
}
```

### `components.json` (shadcn/ui)

```json
{
    "style": "new-york",
    "rsc": true,
    "tsx": true,
    "tailwind": { "config": "", "css": "app/globals.css" },
    "aliases": { "components": "@/components", "utils": "@/lib/utils", "ui": "@/components/ui" }
}
```

---

## Dead Code / Legacy

| Path                     | Status         | Notes                                              |
| ------------------------ | -------------- | -------------------------------------------------- |
| `routes/`                | **Dead**       | Deno/Fresh routes, not used by Next.js             |
| `deno.json`              | **Dead**       | Deno task runner config                            |
| `import_map.json`        | **Dead**       | Deno import map                                    |
| `app/dashboard/page.tsx` | **Redundant?** | Separate from `/console`, with overlapping purpose |
| `app/settings/page.tsx`  | **Redundant?** | Separate from `/console/settings`                  |

---

## Key Issues

1. **`typescript.ignoreBuildErrors: true`** — Type errors are silently ignored during build
2. **JWT not connected to API client** — Console pages may fail without env API key
3. **No route protection middleware** — Console accessible without auth at routing level
4. **Duplicate auth implementations** — `auth-context.tsx`, `auth.ts`, `api-client.ts` all have auth logic
5. **Placeholder metrics** — Console overview shows hardcoded numbers alongside real data
6. **Two dashboard pages** — `/console` and `/dashboard` serve similar purposes
7. **`package.json` name** — Still `my-v0-project` (template residue)
