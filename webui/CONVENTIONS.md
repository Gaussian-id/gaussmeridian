# Gaussian Front-End Conventions

House rules for every app forked from this starter. The linter and formatter enforce what
can be enforced mechanically; this document covers the rest. When in doubt, optimize for
**readability at the call site** — a page should read like a description of the screen, not
an implementation of it.

## Project shape

- **`src/app`** — routes only. No business logic, no reusable components defined here.
- **`src/@core`** — all configuration and plumbing (`config`, `adapters`, `providers`, `lib`).
- **`src/@theme`** — all styling: tokens, the Tailwind/token bridge, global styles.
- **`src/components`** — UI. `ui/` (shadcn baseline), `layout/`, `motion/`.
- **`src/hooks`** — shared React hooks (data hooks live close to their adapter where possible).

Import via aliases, never long relative chains: `@core/*`, `@theme/*`, `@/*`.

## Naming

| Thing                 | Convention                  | Example                |
| --------------------- | --------------------------- | ---------------------- |
| Components            | `PascalCase`                | `NavbarMenu`           |
| Component files       | `kebab-case.tsx`            | `navbar-menu.tsx`      |
| Hooks                 | `useCamelCase`              | `useQueryClientConfig` |
| Variables / functions | `camelCase`                 | `resolveAdapter`       |
| Constants             | `UPPER_SNAKE_CASE`          | `DEFAULT_THEME`        |
| Types / interfaces    | `PascalCase`, no `I` prefix | `AuthAdapter`          |
| Config objects        | `camelCase` + `*.config.ts` | `siteConfig`           |
| Booleans              | `is/has/should` prefix      | `isAuthenticated`      |

Name by intent, not type: `navItems`, not `navArray`.

## Components — co-located compound pattern

We use atomic-design thinking **without** atoms/molecules/organisms folders. Instead, each
component owns and wraps its own subparts in its own folder, and exposes a clean high-level API:

```
components/layout/navbar/
  navbar.tsx        # high-level, reads declaratively
  navbar-logo.tsx   # owned subcomponent
  navbar-menu.tsx   # owned subcomponent
  navbar-actions.tsx
  index.ts          # re-exports the public surface
```

A page should read like prose:

```tsx
<Shell>
  <Navbar />
  <Hero />
</Shell>
```

If a component's JSX becomes a wall of `<div>`s, extract the parts into named subcomponents.

## Styling

- **Never hard-code colors.** Use semantic tokens (`bg-background`, `text-foreground`,
  `bg-primary`, `border-border`, `ring-ring`). All brand values live in `@theme/tokens.css`.
- One identity, light-first; dark is a class toggle off the same tokens. Don't author a
  second palette.
- Tailwind classes are auto-sorted by `prettier-plugin-tailwindcss`. Don't hand-sort.
- Reach for `cn()` (`@core/lib/utils`) to merge conditional classes.

## Data & adapters

- The front-end is a **client**. It never talks to a database. All backend access goes
  through a typed **adapter** resolved from the adapter registry — never call `fetch`
  directly from a component.
- **Validate at the boundary.** Every adapter parses external responses with a Zod schema
  before returning them. Trust nothing that crossed the network.
- Server state lives in **TanStack Query**, not in component state or ad-hoc effects.

## BYOK & secrets

- **Never** persist a provider key in the browser (no `localStorage`, no cookies, no React
  state at rest). Keys go to the backend once; the client holds only a short-lived session
  token. This is a hard rule — it is what makes the posture auditable.

## Anti-patterns (rejected in review)

- `any` (use `unknown` + narrowing, or a real type) and non-null `!` to silence the compiler.
- `fetch` / API calls inside components instead of an adapter + query hook.
- Hard-coded hex/rgb colors or magic spacing values instead of tokens.
- Deeply nested ternaries in JSX — extract a subcomponent or a variable.
- `useEffect` for data fetching — use TanStack Query.
- Barrel files that re-export entire trees (kills tree-shaking); export only the public surface.
- Default-exporting utilities (default export is for Next.js pages/layouts only).
- Storing secrets, tokens, or PII in client-persisted storage.
