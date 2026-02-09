import { auth } from "@/auth";
import { redirect } from "next/navigation";
import Link from "next/link";

export default async function Home() {
  const session = await auth();

  if (session?.user) {
    redirect("/dashboard");
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-950 to-slate-900">
      {/* Mysterious background pattern */}
      <div className="absolute inset-0 bg-[url('data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNjAiIGhlaWdodD0iNjAiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+PGRlZnM+PHBhdHRlcm4gaWQ9ImdyaWQiIHdpZHRoPSI2MCIgaGVpZ2h0PSI2MCIgcGF0dGVyblVuaXRzPSJ1c2VyU3BhY2VPblVzZSI+PHBhdGggZD0iTSAxMCAwIEwgMCAwIDAgMTAiIGZpbGw9Im5vbmUiIHN0cm9rZT0icmdiYSgxMzksIDkyLCAyNDYsIDAuMSkiIHN0cm9rZS13aWR0aD0iMSIvPjwvcGF0dGVybj48L2RlZnM+PHJlY3Qgd2lkdGg9IjEwMCUiIGhlaWdodD0iMTAwJSIgZmlsbD0idXJsKCNncmlkKSIvPjwvc3ZnPg==')] opacity-20"></div>
      
      <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
        <div className="text-center">
          <h1 className="text-5xl font-extrabold sm:text-6xl md:text-7xl">
            <span className="block bg-gradient-to-r from-purple-400 via-violet-400 to-cyan-400 bg-clip-text text-transparent">
              Crypto Pocket Butler
            </span>
            <span className="block text-violet-400 mt-2 text-4xl sm:text-5xl">Portfolio Management</span>
          </h1>
          <p className="mt-6 max-w-2xl mx-auto text-xl text-slate-300">
            Manage your crypto portfolio across wallets and exchanges with intelligent rebalancing suggestions.
          </p>
          <div className="mt-10 flex justify-center gap-4">
            <Link
              href="/auth/signin"
              className="group inline-flex items-center px-8 py-3 border border-violet-500/50 text-base font-medium rounded-lg text-white bg-gradient-to-r from-violet-600 to-purple-600 hover:from-violet-500 hover:to-purple-500 shadow-lg shadow-violet-500/50 hover:shadow-violet-500/70 transition-all duration-300 transform hover:scale-105"
            >
              <span className="mr-2">Get Started</span>
              <svg className="w-4 h-4 group-hover:translate-x-1 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7l5 5m0 0l-5 5m5-5H6" />
              </svg>
            </Link>
            <Link
              href="/dashboard"
              className="inline-flex items-center px-8 py-3 border border-slate-600 text-base font-medium rounded-lg text-slate-300 bg-slate-800/50 hover:bg-slate-700/50 hover:border-slate-500 backdrop-blur-sm transition-all duration-300"
            >
              View Dashboard
            </Link>
          </div>
        </div>

        <div className="mt-20 grid grid-cols-1 gap-8 sm:grid-cols-2 lg:grid-cols-3">
          <div className="group bg-slate-800/50 backdrop-blur-sm rounded-xl border border-slate-700 hover:border-violet-500/50 p-6 transition-all duration-300 hover:shadow-xl hover:shadow-violet-500/20 hover:transform hover:scale-105">
            <div className="flex items-center justify-center h-12 w-12 rounded-lg bg-gradient-to-br from-violet-500 to-purple-600 text-white mb-4 shadow-lg shadow-violet-500/50">
              <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
              </svg>
            </div>
            <h3 className="text-lg font-semibold text-slate-100 group-hover:text-violet-300 transition-colors">Secure Authentication</h3>
            <p className="mt-2 text-slate-400">
              Protected with Keycloak OAuth 2.0 + PKCE for maximum security.
            </p>
          </div>

          <div className="group bg-slate-800/50 backdrop-blur-sm rounded-xl border border-slate-700 hover:border-cyan-500/50 p-6 transition-all duration-300 hover:shadow-xl hover:shadow-cyan-500/20 hover:transform hover:scale-105">
            <div className="flex items-center justify-center h-12 w-12 rounded-lg bg-gradient-to-br from-cyan-500 to-blue-600 text-white mb-4 shadow-lg shadow-cyan-500/50">
              <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
              </svg>
            </div>
            <h3 className="text-lg font-semibold text-slate-100 group-hover:text-cyan-300 transition-colors">Portfolio Tracking</h3>
            <p className="mt-2 text-slate-400">
              Monitor your holdings across multiple wallets and exchanges in one place.
            </p>
          </div>

          <div className="group bg-slate-800/50 backdrop-blur-sm rounded-xl border border-slate-700 hover:border-purple-500/50 p-6 transition-all duration-300 hover:shadow-xl hover:shadow-purple-500/20 hover:transform hover:scale-105">
            <div className="flex items-center justify-center h-12 w-12 rounded-lg bg-gradient-to-br from-purple-500 to-pink-600 text-white mb-4 shadow-lg shadow-purple-500/50">
              <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
              </svg>
            </div>
            <h3 className="text-lg font-semibold text-slate-100 group-hover:text-purple-300 transition-colors">Smart Suggestions</h3>
            <p className="mt-2 text-slate-400">
              AI-powered rebalancing recommendations with built-in guardrails.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
