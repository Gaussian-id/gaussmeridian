import { NextRequest } from "next/server";
import { describe, expect, it } from "vitest";

import { config, proxy } from "../../proxy";

describe("route guard proxy (Next.js 16 — formerly `middleware`)", () => {
  it("does not guard public marketing routes", () => {
    const req = new NextRequest(new URL("http://localhost:3000/"));
    const res = proxy(req);
    expect(res?.status).not.toBe(307);
  });

  it("redirects an unauthenticated request to /orgs to /login (M1 tenancy shell)", () => {
    const req = new NextRequest(new URL("http://localhost:3000/orgs"));
    const res = proxy(req);
    expect(res?.status).toBe(307);
    expect(res?.headers.get("location")).toContain("/login");
  });

  it("preserves the complete checkout return path when authentication must be recovered", () => {
    const req = new NextRequest(
      new URL(
        "https://console.example.test/orgs/org-1/billing/return?order_id=topup-1&cancelled=1",
      ),
    );
    const res = proxy(req);
    const location = new URL(res?.headers.get("location") ?? "https://invalid.test");

    expect(location.pathname).toBe("/login");
    expect(location.searchParams.get("redirectTo")).toBe(
      "/orgs/org-1/billing/return?order_id=topup-1&cancelled=1",
    );
  });

  it("allows an authenticated request to /orgs through", () => {
    const req = new NextRequest(new URL("http://localhost:3000/orgs"), {
      headers: { cookie: "gaussmeridian_session=jwt-abc" },
    });
    const res = proxy(req);
    expect(res?.status).not.toBe(307);
  });

  // The global /playground was removed — the Playground is project-scoped, so it is already
  // covered by the /orgs guard. The prefix must NOT be guarded any more, or the router would
  // bounce a 404 through /login instead of just 404ing.
  it("no longer guards a global /playground", () => {
    const req = new NextRequest(new URL("http://localhost:3000/playground"));
    const res = proxy(req);
    expect(res?.status).not.toBe(307);
  });

  it("still guards the project-scoped Playground via the /orgs tree", () => {
    const req = new NextRequest(
      new URL("http://localhost:3000/orgs/org-1/projects/proj-1/playground"),
    );
    const res = proxy(req);
    expect(res?.status).toBe(307);
    expect(res?.headers.get("location")).toContain("/login");
  });

  it("redirects an unauthenticated request to /account/me to /login", () => {
    const req = new NextRequest(new URL("http://localhost:3000/account/me"));
    const res = proxy(req);
    expect(res?.status).toBe(307);
    expect(res?.headers.get("location")).toContain("/login");
  });

  it("allows an authenticated request to /account/me through", () => {
    const req = new NextRequest(new URL("http://localhost:3000/account/me"), {
      headers: { cookie: "gaussmeridian_session=jwt-abc" },
    });
    const res = proxy(req);
    expect(res?.status).not.toBe(307);
  });

  it("guards the /orgs and /account trees in the matcher config", () => {
    expect(config.matcher).toContain("/orgs/:path*");
    expect(config.matcher).toContain("/account/:path*");
    expect(config.matcher).not.toContain("/playground/:path*");
  });

  it("redirects an unauthenticated request to /admin to /login (PRD-23 Wave C)", () => {
    const req = new NextRequest(new URL("http://localhost:3000/admin"));
    const res = proxy(req);
    expect(res?.status).toBe(307);
    expect(res?.headers.get("location")).toContain("/login");
  });

  it("allows an authenticated request to /admin through — SuperadminGate does the real authorization check client-side, not this edge guard", () => {
    const req = new NextRequest(new URL("http://localhost:3000/admin"), {
      headers: { cookie: "gaussmeridian_session=jwt-abc" },
    });
    const res = proxy(req);
    expect(res?.status).not.toBe(307);
  });

  it("guards the /admin tree in the matcher config", () => {
    expect(config.matcher).toContain("/admin/:path*");
  });

  it("does not guard the retired /dashboard tree (M6 — legacy route deleted)", () => {
    expect(config.matcher).not.toContain("/dashboard/:path*");
    const req = new NextRequest(new URL("http://localhost:3000/dashboard"));
    const res = proxy(req);
    expect(res?.status).not.toBe(307);
  });

  it("rejects a state-changing request with a cross-origin Origin header (CSRF)", () => {
    const req = new NextRequest(new URL("http://localhost:3000/api/auth/login"), {
      method: "POST",
      headers: { origin: "https://evil.example.com" },
    });
    const res = proxy(req);
    expect(res?.status).toBe(403);
  });

  it("rejects a cross-origin POST to the gaussmeridian data proxy (CSRF)", () => {
    const req = new NextRequest(new URL("http://localhost:3000/api/gaussmeridian/v1/api/keys"), {
      method: "POST",
      headers: { origin: "https://evil.example.com" },
    });
    const res = proxy(req);
    expect(res?.status).toBe(403);
  });

  it("rejects a state-changing request with no Origin header at all (fail closed, not fail open)", () => {
    const req = new NextRequest(new URL("http://localhost:3000/api/gaussmeridian/v1/api/keys"), {
      method: "POST",
    });
    const res = proxy(req);
    expect(res?.status).toBe(403);
  });

  it("allows a state-changing request with a matching same-origin Origin header", () => {
    // Origin is compared against the request's own Host header (deployment-agnostic —
    // nextUrl.origin gets normalized to the server's bind host under standalone/Docker).
    // Real HTTP requests always carry Host; the test must set it explicitly.
    const req = new NextRequest(new URL("http://localhost:3000/api/gaussmeridian/v1/api/keys"), {
      method: "POST",
      headers: { origin: "http://localhost:3000", host: "localhost:3000" },
    });
    const res = proxy(req);
    expect(res?.status).not.toBe(403);
  });

  it("rejects when the Origin host does not match the Host header", () => {
    const req = new NextRequest(new URL("http://localhost:3000/api/gaussmeridian/v1/api/keys"), {
      method: "POST",
      headers: { origin: "http://localhost:4000", host: "localhost:3000" },
    });
    const res = proxy(req);
    expect(res?.status).toBe(403);
  });

  it("rejects an unparseable Origin header (fail closed)", () => {
    const req = new NextRequest(new URL("http://localhost:3000/api/gaussmeridian/v1/api/keys"), {
      method: "POST",
      headers: { origin: "not-a-url", host: "localhost:3000" },
    });
    const res = proxy(req);
    expect(res?.status).toBe(403);
  });

  it("actually runs the proxy on the gaussmeridian proxy route (matcher config)", () => {
    // The proxy function's CSRF check has no path guard of its own — it only ever runs
    // for a request if Next.js's matcher includes that path. Without this, the CSRF
    // check above would be dead code in production despite passing in tests that call
    // proxy() directly, bypassing the matcher entirely.
    expect(config.matcher).toContain("/api/gaussmeridian/:path*");
  });
});
