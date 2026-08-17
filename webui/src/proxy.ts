import { NextRequest, NextResponse } from "next/server";

const GUARDED_PREFIXES = ["/orgs", "/onboarding", "/account", "/admin"];
const STATE_CHANGING_METHODS = new Set(["POST", "PUT", "PATCH", "DELETE"]);

export function proxy(request: NextRequest) {
  const { pathname } = request.nextUrl;

  // CSRF mitigation (D4): reject cross-origin state-changing requests even
  // while SameSite=Lax blocks cross-site unsafe requests — this is the
  // Origin-header defense-in-depth layer for same-site-but-different-app
  // edge cases. The Origin header's host is compared against the request's own
  // Host header (not `nextUrl.origin`, which the standalone/Docker runtime
  // normalizes to the server's bind hostname and therefore never matches).
  // Fail-closed: a missing or unparseable Origin is rejected.
  if (STATE_CHANGING_METHODS.has(request.method)) {
    const origin = request.headers.get("origin");
    const host = request.headers.get("host");
    let originHost: string | null = null;
    try {
      originHost = origin ? new URL(origin).host : null;
    } catch {
      originHost = null;
    }
    if (!originHost || !host || originHost !== host) {
      return new NextResponse(null, { status: 403 });
    }
  }

  const isGuarded = GUARDED_PREFIXES.some((prefix) => pathname.startsWith(prefix));
  if (!isGuarded) return NextResponse.next();

  const session = request.cookies.get("gaussmeridian_session");
  if (!session) {
    const loginUrl = new URL("/login", request.url);
    loginUrl.searchParams.set("redirectTo", `${pathname}${request.nextUrl.search}`);
    return NextResponse.redirect(loginUrl);
  }

  return NextResponse.next();
}

export const config = {
  matcher: [
    "/orgs/:path*",
    "/onboarding/:path*",
    "/account/:path*",
    "/admin/:path*",
    "/api/auth/:path*",
    "/api/gaussmeridian/:path*",
  ],
};
