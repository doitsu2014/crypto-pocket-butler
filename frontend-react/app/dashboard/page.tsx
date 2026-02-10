import { auth } from "@/auth";
import { redirect } from "next/navigation";
import { SignOutButton } from "@/components/SignOutButton";
import { UserInfo } from "@/components/UserInfo";
import Link from "next/link";

export default async function DashboardPage() {
  const session = await auth();

  if (!session?.user) {
    redirect("/auth/signin");
  }

  return (
    <div className="min-h-screen bg-black relative overflow-hidden">
      {/* Intense neon background pattern */}
      <div className="absolute inset-0 bg-[url('data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNjAiIGhlaWdodD0iNjAiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+PGRlZnM+PHBhdHRlcm4gaWQ9ImdyaWQiIHdpZHRoPSI2MCIgaGVpZ2h0PSI2MCIgcGF0dGVyblVuaXRzPSJ1c2VyU3BhY2VPblVzZSI+PHBhdGggZD0iTSAxMCAwIEwgMCAwIDAgMTAiIGZpbGw9Im5vbmUiIHN0cm9rZT0icmdiYSgxMzksIDkyLCAyNDYsIDAuMikiIHN0cm9rZS13aWR0aD0iMSIvPjwvcGF0dGVybj48L2RlZnM+PHJlY3Qgd2lkdGg9IjEwMCUiIGhlaWdodD0iMTAwJSIgZmlsbD0idXJsKCNncmlkKSIvPjwvc3ZnPg==')] opacity-30"></div>
      
      {/* Glowing orbs */}
      <div className="absolute top-0 right-0 w-[500px] h-[500px] bg-fuchsia-500/25 rounded-full blur-[120px] animate-pulse"></div>
      <div className="absolute bottom-0 left-0 w-[500px] h-[500px] bg-cyan-500/25 rounded-full blur-[120px] animate-pulse" style={{ animationDelay: '1.5s' }}></div>
      
      <nav className="relative bg-slate-950/80 backdrop-blur-xl border-b-2 border-fuchsia-500/30 shadow-[0_0_30px_rgba(217,70,239,0.3)]">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between h-16">
            <div className="flex items-center">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-fuchsia-500 to-violet-600 flex items-center justify-center shadow-[0_0_30px_rgba(217,70,239,0.8)] animate-pulse">
                  <svg className="w-6 h-6 text-white drop-shadow-[0_0_10px_rgba(255,255,255,0.8)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                </div>
                <h1 className="text-xl font-bold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-cyan-300 bg-clip-text text-transparent drop-shadow-[0_0_15px_rgba(232,121,249,0.6)]">
                  Crypto Pocket Butler
                </h1>
              </div>
            </div>
            <div className="flex items-center space-x-4">
              <span className="text-sm text-slate-300 drop-shadow-[0_0_5px_rgba(203,213,225,0.5)]">
                {session.user.email}
              </span>
              <SignOutButton />
            </div>
          </div>
        </div>
      </nav>

      <main className="relative max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
        <div className="px-4 py-6 sm:px-0">
          <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-fuchsia-500/40 shadow-[0_0_40px_rgba(217,70,239,0.4)] rounded-2xl p-6">
            <h2 className="text-3xl font-extrabold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-purple-300 bg-clip-text text-transparent mb-4 drop-shadow-[0_0_20px_rgba(232,121,249,0.6)] animate-pulse">
              Welcome to your Dashboard
            </h2>
            <p className="text-slate-200 mb-6 drop-shadow-[0_0_10px_rgba(226,232,240,0.3)]">
              You are successfully authenticated with Keycloak using OAuth 2.0 + PKCE flow.
            </p>
            
            <UserInfo />
          </div>

          <Link href="/portfolios" className="block mt-6 bg-slate-950/70 backdrop-blur-sm border-2 border-cyan-500/40 shadow-[0_0_40px_rgba(34,211,238,0.4)] hover:shadow-[0_0_60px_rgba(34,211,238,0.6)] hover:border-cyan-300 rounded-2xl p-6 transition-all duration-300 hover:scale-[1.02] group">
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-3">
                <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-cyan-500 to-blue-600 flex items-center justify-center shadow-[0_0_30px_rgba(34,211,238,0.8)] animate-pulse group-hover:shadow-[0_0_40px_rgba(34,211,238,1)]">
                  <svg className="w-7 h-7 text-white drop-shadow-[0_0_10px_rgba(255,255,255,0.8)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                  </svg>
                </div>
                <h3 className="text-xl font-bold text-cyan-300 group-hover:text-cyan-200 drop-shadow-[0_0_15px_rgba(103,232,249,0.6)]">
                  Portfolios
                </h3>
              </div>
              <svg className="w-6 h-6 text-cyan-400 group-hover:translate-x-1 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7l5 5m0 0l-5 5m5-5H6" />
              </svg>
            </div>
            <p className="text-slate-300">
              Manage your portfolios, create new ones, and track your holdings across wallets and exchanges.
            </p>
          </Link>
        </div>
      </main>
    </div>
  );
}
