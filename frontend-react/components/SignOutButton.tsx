"use client";

import { signOut } from "next-auth/react";

export function SignOutButton() {
  return (
    <button
      onClick={() => signOut({ callbackUrl: "/" })}
      className="inline-flex items-center px-4 py-2 border-2 border-red-500 text-sm font-bold rounded-lg text-red-300 bg-red-950/30 hover:bg-red-900/50 hover:border-red-400 focus:outline-none focus:ring-4 focus:ring-red-500/50 shadow-[0_0_20px_rgba(239,68,68,0.4)] hover:shadow-[0_0_30px_rgba(239,68,68,0.7)] transition-all duration-300 transform hover:scale-105"
    >
      <svg className="w-4 h-4 mr-2 drop-shadow-[0_0_5px_rgba(252,165,165,0.6)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
      </svg>
      Sign Out
    </button>
  );
}
