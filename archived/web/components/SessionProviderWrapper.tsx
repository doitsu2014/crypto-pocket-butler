"use client";

import { SessionProvider } from "next-auth/react";

/**
 * Thin client-side wrapper that provides the NextAuth SessionProvider to the
 * entire component tree. Required so that client components (e.g. AppLayout)
 * can call `useSession()` to access session data including user roles.
 */
export default function SessionProviderWrapper({
  children,
}: {
  children: React.ReactNode;
}) {
  return <SessionProvider>{children}</SessionProvider>;
}
