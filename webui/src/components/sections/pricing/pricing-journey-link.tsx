"use client";

import Link from "next/link";

import { useSession } from "@/hooks/useSession";
import {
  ADD_CREDIT_LOGIN_HREF,
  ADD_CREDIT_ORG_RESOLVER_HREF,
  ADD_CREDIT_SIGNUP_HREF,
} from "@/lib/billing/billing-intent";

import type { MouseEvent, ReactNode } from "react";

type JourneyLinkProps = {
  children: ReactNode;
  className?: string;
};

function stopWhileResolving(event: MouseEvent<HTMLAnchorElement>, resolving: boolean) {
  if (resolving) event.preventDefault();
}

export function PricingCreditLink({ children, className }: JourneyLinkProps) {
  const session = useSession();
  const resolving = session.isPending;

  return (
    <Link
      href={session.data ? ADD_CREDIT_ORG_RESOLVER_HREF : ADD_CREDIT_SIGNUP_HREF}
      className={className}
      aria-disabled={resolving || undefined}
      onClick={(event) => stopWhileResolving(event, resolving)}
    >
      {session.data ? "Choose organization" : children}
    </Link>
  );
}

export function PricingAccountLink({ children, className }: JourneyLinkProps) {
  const session = useSession();
  const resolving = session.isPending;

  return (
    <Link
      href={session.data ? "/orgs" : ADD_CREDIT_LOGIN_HREF}
      className={className}
      aria-disabled={resolving || undefined}
      onClick={(event) => stopWhileResolving(event, resolving)}
    >
      {session.data ? "Open dashboard" : children}
    </Link>
  );
}
