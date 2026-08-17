"use client";

import { MessageSquare } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";

const PROJECT_ROUTE = /^(\/orgs\/[^/]+\/projects\/[^/]+)(?:\/|$)/;

/** The Playground is project-scoped. Outside a project there is nowhere to send the user. */
function playgroundHref(pathname: string | null): string | null {
  const projectBase = pathname?.match(PROJECT_ROUTE)?.[1];
  return projectBase ? `${projectBase}/playground` : null;
}

/**
 * Floating launcher for the project Playground. Used to be its own miniature chat panel
 * (`ChatPanel`) — keeping a second, smaller chat UI floating over every screen would just be
 * two competing chat surfaces telling two different stories about the same assistant. This
 * deep-links to the real one instead of duplicating it.
 *
 * Renders only inside a project. The global `/playground` route was removed: a Playground with
 * no project has no API key, no budget and no model catalog to route against, so the shortcut
 * has no destination outside a project and is hidden rather than pointing somewhere broken.
 *
 * Also hidden while already on the Playground route — linking to the page you're standing on is
 * a pointless self-link. Uses the same prefix-match idiom as `resolveActiveHref` in
 * `nav.config.ts` so it also hides on any future nested Playground route.
 */
export function ChatWidget() {
  const pathname = usePathname();
  const href = playgroundHref(pathname);

  if (!href) return null;
  if (pathname === href || pathname?.startsWith(`${href}/`)) return null;

  return (
    <aside aria-label="Playground shortcut">
      <Link
        href={href}
        aria-label="Open the Playground"
        className="bg-brand-gradient shadow-glow fixed right-6 bottom-6 z-50 grid h-14 w-14 place-items-center rounded-full text-white transition-transform hover:scale-105"
      >
        <MessageSquare className="h-6 w-6" aria-hidden="true" />
      </Link>
    </aside>
  );
}
