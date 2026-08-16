# Gaussian Front-End Starter (`gauss-boilerplate`)

The common ancestor for every Gaussian front-end — products, SBUs, and consultancy sites.
Fork it and a new surface starts with one identity, the backend plumbing, and enforced
standards already in place. Then it is free to diverge.

> One package, one feel. A user should never sense a seam between the Gaussian landing page
> and a Gaussian product.

## Stack

Next.js 16 (App Router, `src/`) · TypeScript · Tailwind v4 · shadcn/ui (baseline) ·
TanStack Query · Zod · next-themes · Lenis · GSAP · Three.js · ESLint + Prettier · Vitest.

## Getting started

```bash
pnpm install
pnpm dev          # http://localhost:3000
```

Configure the backend the client talks to:

```bash
# .env.local
NEXT_PUBLIC_API_BASE_URL=https://your-gaussian-backend.example.com
```

## How to fork this for a new app

1. Create the new repo from this one (GitHub "Use this template", or clone + re-point origin).
2. Re-brand in **`src/@core/config/site.config.ts`** (name, navbar position, contact) and
   **`src/@core/config/nav.config.ts`** (navigation).
3. Adjust the palette/fonts in **`src/@theme/tokens.css`** and **`src/@core/config/fonts.ts`**
   if the sub-brand needs it — but keep within the Gaussian system.
4. Point **`NEXT_PUBLIC_API_BASE_URL`** at the app's backend and implement screens.
5. Everything below stays — that's the point.

## Project structure

```
src/
├─ app/                  # routes ONLY (App Router), grouped by surface
│  ├─ (marketing)/       # public chrome (Navbar + Footer) — landing page
│  ├─ (auth)/            # split brand/form chrome — /login
│  └─ (app)/             # authed shell (sidebar + topbar) — /dashboard, /settings
├─ @core/                # all configuration + plumbing       (alias: @core)
│  ├─ config/            # site.config, nav.config (public + app), fonts
│  ├─ adapters/          # llm-byok · data-query · auth (interface + HTTP ref impl)
│  ├─ providers/         # adapter registry, TanStack Query, theme
│  └─ lib/               # utils + env (Zod)
├─ @theme/               # all styling                         (alias: @theme)
│  ├─ tokens.css         # semantic tokens — light default + dark toggle
│  ├─ globals.css        # tailwind import + base + brand utilities
│  └─ theme.config.ts
├─ components/
│  ├─ ui/                # shadcn baseline (Button, Card, Badge, Input, Label)
│  ├─ layout/            # Shell + Navbar (marketing); AppShell + sidebar/topbar (app)
│  ├─ sections/          # marketing sections (hero, capabilities, products, cta)
│  ├─ charts/            # token-themed Recharts wrappers (TrendChart)
│  └─ motion/            # SmoothScroll (Lenis), Reveal (GSAP), BrandOrb (Three.js)
├─ hooks/                # query/mutation hooks (useResourceQuery, useSignIn, useByok)
└─ test/                # fakes + render helper
```

## Routes & surfaces

Route groups `()` keep each surface's chrome separate while sharing the same tokens and
components — URLs are unaffected.

| Route        | Group       | Demonstrates                                                               |
| ------------ | ----------- | -------------------------------------------------------------------------- |
| `/`          | (marketing) | The design system: hero, capabilities, products, CTA                       |
| `/login`     | (auth)      | Split brand/form layout; auth adapter via `useSignIn`                      |
| `/dashboard` | (app)       | App shell; stat cards; token-themed chart; `useResourceQuery` data pattern |
| `/settings`  | (app)       | BYOK via `useRegisterKey` — provider key never persisted client-side       |

The authenticated app also mounts a global **assistant** — a floating chat widget
(bottom-right) backed by the BYOK adapter's streaming `streamChat`, so the provider key
stays server-side. A presentational highlight of it lives on the landing page.

## Identity rules (do not break across forks)

- **One look, light-first.** Stylized light is canonical; dark is a class toggle off the
  **same** tokens. Never author a second palette.
- **Never hard-code colors.** Use semantic tokens (`bg-background`, `text-primary`,
  `border-border`). Brand values live only in `@theme/tokens.css`.
- **Components read declaratively.** Co-located compound components — each owns its subparts.
  See `CONVENTIONS.md`.

## Architecture rules

- The front-end is a **client**. It never talks to a database. All backend access goes
  through a typed **adapter** resolved from the adapter registry (the single seam).
- **Validate at the boundary** with Zod. Server state lives in TanStack Query.
- **BYOK keys are server-backed only** — sent to the backend once, never persisted in the
  browser; the client holds only a session token. Enforced by `byok-security.test.ts`.

## Backend contract

The front-end is a client; these are the endpoints the reference adapters call, relative to
`NEXT_PUBLIC_API_BASE_URL`. Implement them on your backend to make a fork fully live.

| Method & path            | Body                         | Returns                                         |
| ------------------------ | ---------------------------- | ----------------------------------------------- |
| `POST /auth/signin`      | `{ email, password }`        | `AuthSession`                                   |
| `GET /auth/session`      | —                            | `AuthSession` (or 401)                          |
| `POST /auth/signout`     | —                            | `{ ok: boolean }`                               |
| `POST /byok/keys`        | `{ provider, apiKey }`       | `ByokSession` — a `sessionToken`, never the key |
| `POST /byok/complete`    | `{ sessionToken, prompt }`   | `{ text }`                                      |
| `POST /byok/keys/revoke` | `{ sessionToken }`           | `{ ok: boolean }`                               |
| `POST /byok/chat`        | `{ sessionToken, messages }` | streamed text chunks (the assistant)            |
| `GET /{resource}`        | query params                 | a shape validated by the caller's Zod schema    |

`AuthSession` = `{ userId, displayName, token, expiresAt }`.
`ByokSession` = `{ provider, sessionToken, expiresAt }`.
The provider key is sent once to `POST /byok/keys` and must never be returned to the client.

## Scripts

| Command                         | Purpose          |
| ------------------------------- | ---------------- |
| `pnpm dev`                      | Dev server       |
| `pnpm build`                    | Production build |
| `pnpm lint` / `pnpm lint:fix`   | ESLint           |
| `pnpm format`                   | Prettier write   |
| `pnpm typecheck`                | `tsc --noEmit`   |
| `pnpm test` / `pnpm test:watch` | Vitest           |

## Quality gates

A pre-commit hook (Husky + lint-staged) fixes and formats staged files; CI
(`.github/workflows/ci.yml`) runs lint → typecheck → test → build on every push and PR.

## Adding shadcn components

`components.json` is configured, so:

```bash
pnpm dlx shadcn@latest add dialog
```

New primitives land in `src/components/ui` already wired to Gaussian tokens.
