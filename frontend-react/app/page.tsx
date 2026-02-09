import { auth } from "@/auth";
import { redirect } from "next/navigation";
import Link from "next/link";

export default async function Home() {
  const session = await auth();

  if (session?.user) {
    redirect("/dashboard");
  }

  return (
    <div className="min-h-screen bg-black relative overflow-hidden">
      {/* Intense neon background pattern */}
      <div className="absolute inset-0 bg-[url('data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNjAiIGhlaWdodD0iNjAiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+PGRlZnM+PHBhdHRlcm4gaWQ9ImdyaWQiIHdpZHRoPSI2MCIgaGVpZ2h0PSI2MCIgcGF0dGVyblVuaXRzPSJ1c2VyU3BhY2VPblVzZSI+PHBhdGggZD0iTSAxMCAwIEwgMCAwIDAgMTAiIGZpbGw9Im5vbmUiIHN0cm9rZT0icmdiYSgxMzksIDkyLCAyNDYsIDAuMykiIHN0cm9rZS13aWR0aD0iMiIvPjwvcGF0dGVybj48L2RlZnM+PHJlY3Qgd2lkdGg9IjEwMCUiIGhlaWdodD0iMTAwJSIgZmlsbD0idXJsKCNncmlkKSIvPjwvc3ZnPg==')] opacity-30"></div>
      
      {/* Intense glowing orbs */}
      <div className="absolute top-0 left-0 w-[600px] h-[600px] bg-fuchsia-500/30 rounded-full blur-[120px] animate-pulse"></div>
      <div className="absolute top-1/3 right-0 w-[500px] h-[500px] bg-cyan-500/30 rounded-full blur-[120px] animate-pulse" style={{ animationDelay: '1s' }}></div>
      <div className="absolute bottom-0 left-1/3 w-[550px] h-[550px] bg-violet-500/25 rounded-full blur-[120px] animate-pulse" style={{ animationDelay: '2s' }}></div>
      
      <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
        <div className="text-center">
          <h1 className="text-5xl font-extrabold sm:text-6xl md:text-7xl drop-shadow-[0_0_30px_rgba(168,85,247,0.8)]">
            <span className="block bg-gradient-to-r from-fuchsia-400 via-purple-400 to-cyan-400 bg-clip-text text-transparent animate-pulse">
              Crypto Pocket Butler
            </span>
            <span className="block bg-gradient-to-r from-violet-400 to-fuchsia-400 bg-clip-text text-transparent mt-2 text-4xl sm:text-5xl drop-shadow-[0_0_20px_rgba(168,85,247,0.6)]">Portfolio Management</span>
          </h1>
          <p className="mt-6 max-w-2xl mx-auto text-xl text-slate-200 drop-shadow-[0_0_10px_rgba(148,163,184,0.5)]">
            Manage your crypto portfolio across wallets and exchanges with intelligent rebalancing suggestions.
          </p>
          <div className="mt-10 flex justify-center gap-4">
            <Link
              href="/auth/signin"
              className="group inline-flex items-center px-8 py-3 border-2 border-fuchsia-500 text-base font-medium rounded-lg text-white bg-gradient-to-r from-fuchsia-600 via-purple-600 to-violet-600 hover:from-fuchsia-500 hover:via-purple-500 hover:to-violet-500 shadow-[0_0_30px_rgba(217,70,239,0.6)] hover:shadow-[0_0_50px_rgba(217,70,239,0.9)] transition-all duration-300 transform hover:scale-110 animate-pulse"
            >
              <span className="mr-2 font-bold">Get Started</span>
              <svg className="w-4 h-4 group-hover:translate-x-1 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7l5 5m0 0l-5 5m5-5H6" />
              </svg>
            </Link>
            <Link
              href="/dashboard"
              className="inline-flex items-center px-8 py-3 border-2 border-cyan-500 text-base font-medium rounded-lg text-cyan-200 bg-slate-900/50 hover:bg-slate-800/70 hover:border-cyan-400 backdrop-blur-sm shadow-[0_0_20px_rgba(34,211,238,0.4)] hover:shadow-[0_0_40px_rgba(34,211,238,0.7)] transition-all duration-300 transform hover:scale-105"
            >
              View Dashboard
            </Link>
          </div>
        </div>

        <div className="mt-20 grid grid-cols-1 gap-8 sm:grid-cols-2 lg:grid-cols-3">
          <div className="group bg-slate-900/60 backdrop-blur-sm rounded-xl border-2 border-fuchsia-500/50 hover:border-fuchsia-400 p-6 transition-all duration-300 shadow-[0_0_30px_rgba(217,70,239,0.3)] hover:shadow-[0_0_50px_rgba(217,70,239,0.6)] hover:transform hover:scale-105">
            <div className="flex items-center justify-center h-12 w-12 rounded-lg bg-gradient-to-br from-fuchsia-500 to-purple-600 text-white mb-4 shadow-[0_0_30px_rgba(217,70,239,0.8)] animate-pulse">
              <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
              </svg>
            </div>
            <h3 className="text-lg font-bold text-fuchsia-300 group-hover:text-fuchsia-200 transition-colors drop-shadow-[0_0_10px_rgba(232,121,249,0.5)]">Secure Authentication</h3>
            <p className="mt-2 text-slate-300">
              Protected with Keycloak OAuth 2.0 + PKCE for maximum security.
            </p>
          </div>

          <div className="group bg-slate-900/60 backdrop-blur-sm rounded-xl border-2 border-cyan-500/50 hover:border-cyan-400 p-6 transition-all duration-300 shadow-[0_0_30px_rgba(34,211,238,0.3)] hover:shadow-[0_0_50px_rgba(34,211,238,0.6)] hover:transform hover:scale-105">
            <div className="flex items-center justify-center h-12 w-12 rounded-lg bg-gradient-to-br from-cyan-500 to-blue-600 text-white mb-4 shadow-[0_0_30px_rgba(34,211,238,0.8)] animate-pulse" style={{ animationDelay: '0.5s' }}>
              <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
              </svg>
            </div>
            <h3 className="text-lg font-bold text-cyan-300 group-hover:text-cyan-200 transition-colors drop-shadow-[0_0_10px_rgba(103,232,249,0.5)]">Portfolio Tracking</h3>
            <p className="mt-2 text-slate-300">
              Monitor your holdings across multiple wallets and exchanges in one place.
            </p>
          </div>

          <div className="group bg-slate-900/60 backdrop-blur-sm rounded-xl border-2 border-violet-500/50 hover:border-violet-400 p-6 transition-all duration-300 shadow-[0_0_30px_rgba(139,92,246,0.3)] hover:shadow-[0_0_50px_rgba(139,92,246,0.6)] hover:transform hover:scale-105">
            <div className="flex items-center justify-center h-12 w-12 rounded-lg bg-gradient-to-br from-violet-500 to-fuchsia-600 text-white mb-4 shadow-[0_0_30px_rgba(139,92,246,0.8)] animate-pulse" style={{ animationDelay: '1s' }}>
              <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
              </svg>
            </div>
            <h3 className="text-lg font-bold text-violet-300 group-hover:text-violet-200 transition-colors drop-shadow-[0_0_10px_rgba(167,139,250,0.5)]">Smart Suggestions</h3>
            <p className="mt-2 text-slate-300">
              AI-powered rebalancing recommendations with built-in guardrails.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
