"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { SignOutButton } from "./SignOutButton";

interface AppLayoutProps {
  children: React.ReactNode;
  userEmail?: string | null;
}

const navigation = [
  {
    name: "Dashboard",
    href: "/dashboard",
    matchExact: true,
    icon: (
      <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
      </svg>
    ),
  },
  {
    name: "Portfolios",
    href: "/portfolios",
    matchExact: false,
    icon: (
      <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
      </svg>
    ),
  },
  {
    name: "Accounts",
    href: "/accounts",
    matchExact: true,
    icon: (
      <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2m2 4h10a2 2 0 002-2v-6a2 2 0 00-2-2H9a2 2 0 00-2 2v6a2 2 0 002 2zm7-5a2 2 0 11-4 0 2 2 0 014 0z" />
      </svg>
    ),
  },
  {
    name: "Settings",
    href: "/settings",
    matchExact: true,
    icon: (
      <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    ),
  },
  {
    name: "Administration",
    href: "/admin",
    matchExact: false,
    icon: (
      <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
      </svg>
    ),
  },
];

export default function AppLayout({ children, userEmail }: AppLayoutProps) {
  const pathname = usePathname();

  const isActive = (item: typeof navigation[0]) => {
    if (!pathname) return false;
    
    if (item.matchExact) {
      return pathname === item.href;
    }
    // For non-exact matches (like /portfolios), we want to highlight when:
    // 1. User is exactly on the route (/portfolios), OR
    // 2. User is on any subpage (/portfolios/123, /portfolios/123/settings, etc.)
    return pathname === item.href || pathname.startsWith(`${item.href}/`);
  };

  return (
    <div className="min-h-screen bg-black relative overflow-hidden">
      {/* Cyberpunk neon background pattern */}
      <div className="absolute inset-0 bg-[url('data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNjAiIGhlaWdodD0iNjAiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+PGRlZnM+PHBhdHRlcm4gaWQ9ImdyaWQiIHdpZHRoPSI2MCIgaGVpZ2h0PSI2MCIgcGF0dGVyblVuaXRzPSJ1c2VyU3BhY2VPblVzZSI+PHBhdGggZD0iTSAxMCAwIEwgMCAwIDAgMTAiIGZpbGw9Im5vbmUiIHN0cm9rZT0icmdiYSgxMzksIDkyLCAyNDYsIDAuMikiIHN0cm9rZS13aWR0aD0iMSIvPjwvcGF0dGVybj48L2RlZnM+PHJlY3Qgd2lkdGg9IjEwMCUiIGhlaWdodD0iMTAwJSIgZmlsbD0idXJsKCNncmlkKSIvPjwvc3ZnPg==')] opacity-25"></div>
      
      {/* Ambient glowing orbs */}
      <div className="absolute top-0 right-0 w-[450px] h-[450px] bg-fuchsia-500/15 rounded-full blur-[110px]"></div>
      <div className="absolute bottom-0 left-0 w-[450px] h-[450px] bg-cyan-500/15 rounded-full blur-[110px]"></div>
      
      {/* Top navbar */}
      <nav className="relative bg-slate-950/80 backdrop-blur-xl border-b-2 border-fuchsia-500/30 shadow-[0_0_20px_rgba(217,70,239,0.25)]" aria-label="User and branding">
        <div className="px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between h-16">
            <div className="flex items-center">
              <Link href="/dashboard" className="flex items-center gap-3 group">
                <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-fuchsia-500 to-violet-600 flex items-center justify-center shadow-[0_0_20px_rgba(217,70,239,0.5)] group-hover:shadow-[0_0_25px_rgba(217,70,239,0.7)] transition-all">
                  <svg className="w-6 h-6 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                </div>
                <h1 className="text-xl font-bold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-cyan-300 bg-clip-text text-transparent drop-shadow-[0_0_8px_rgba(232,121,249,0.4)]">
                  Crypto Pocket Butler
                </h1>
              </Link>
            </div>
            <div className="flex items-center space-x-4">
              {userEmail && (
                <span className="text-sm text-slate-300 drop-shadow-[0_0_5px_rgba(203,213,225,0.5)]">
                  {userEmail}
                </span>
              )}
              <SignOutButton />
            </div>
          </div>
        </div>
      </nav>

      <div className="flex">
        {/* Sidebar */}
        <aside className="relative w-64 min-h-[calc(100vh-4rem)] bg-slate-950/60 backdrop-blur-sm border-r-2 border-fuchsia-500/20" aria-label="Main navigation sidebar">
          <nav className="p-4 space-y-2" aria-label="Main navigation">
            {navigation.map((item) => {
              const active = isActive(item);
              return (
                <Link
                  key={item.name}
                  href={item.href}
                  aria-current={active ? "page" : undefined}
                  className={`
                    flex items-center gap-3 px-4 py-3 rounded-lg transition-all duration-200
                    ${
                      active
                        ? "bg-gradient-to-r from-fuchsia-600/30 to-violet-600/30 border-2 border-fuchsia-500/50 text-fuchsia-200 shadow-[0_0_20px_rgba(217,70,239,0.4)]"
                        : "text-slate-300 hover:bg-slate-800/50 hover:text-cyan-200 border-2 border-transparent hover:border-cyan-500/30"
                    }
                  `}
                >
                  <div className={active ? "text-fuchsia-300" : "text-slate-400"}>
                    {item.icon}
                  </div>
                  <span className="font-medium">{item.name}</span>
                </Link>
              );
            })}
          </nav>
        </aside>

        {/* Main content */}
        <main className="relative flex-1 px-4 sm:px-6 lg:px-8 py-6">
          {children}
        </main>
      </div>
    </div>
  );
}
