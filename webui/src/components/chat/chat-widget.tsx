"use client";

import { MessageSquare } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";

const PLAYGROUND_HREF = "/playground";
const PROJECT_ROUTE = /^(\/orgs\/[^/]+\/projects\/[^/]+)(?:\/|$)/;

function playgroundHref(pathname: string | null): string {
  const projectBase = pathname?.match(PROJECT_ROUTE)?.[1];
  return projectBase ? `${projectBase}/playground` : PLAYGROUND_HREF;
}

/**
 * Floating launcher for the global Playground. Used to be its own miniature chat panel
 * (`ChatPanel`) — now that M5 ships a full ChatGPT-style chat at `/playground`, keeping a
 * second, smaller chat UI floating over every screen would just be two competing chat
 * surfaces telling two different stories about the same assistant. This deep-links to the
 * real one instead of duplicating it.
 *
 * Hidden while already on the Playground route — linking to the page you're standing on is
 * a pointless self-link. Uses the same prefix-match idiom as `resolveActiveHref` in
 * `nav.config.ts` so it also hides on any future nested Playground route.
 */
export function ChatWidget() {
  const pathname = usePathname();
  const href = playgroundHref(pathname);
  const onPlayground = pathname === href || pathname?.startsWith(`${href}/`);

  if (onPlayground) return null;

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
