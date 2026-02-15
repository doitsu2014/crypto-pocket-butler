import { auth } from "@/auth";
import { redirect } from "next/navigation";
import { UserInfo } from "@/components/UserInfo";
import Link from "next/link";
import AppLayout from "@/components/AppLayout";

export default async function DashboardPage() {
  const session = await auth();

  if (!session?.user) {
    redirect("/auth/signin");
  }

  return (
    <AppLayout userEmail={session.user.email}>
      <div className="max-w-7xl">
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

        <Link href="/accounts" className="block mt-6 bg-slate-950/70 backdrop-blur-sm border-2 border-fuchsia-500/40 shadow-[0_0_40px_rgba(217,70,239,0.4)] hover:shadow-[0_0_60px_rgba(217,70,239,0.6)] hover:border-fuchsia-300 rounded-2xl p-6 transition-all duration-300 hover:scale-[1.02] group">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-3">
              <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-fuchsia-500 to-violet-600 flex items-center justify-center shadow-[0_0_30px_rgba(217,70,239,0.8)] animate-pulse group-hover:shadow-[0_0_40px_rgba(217,70,239,1)]">
                <svg className="w-7 h-7 text-white drop-shadow-[0_0_10px_rgba(255,255,255,0.8)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2m2 4h10a2 2 0 002-2v-6a2 2 0 00-2-2H9a2 2 0 00-2 2v6a2 2 0 002 2zm7-5a2 2 0 11-4 0 2 2 0 014 0z" />
                </svg>
              </div>
              <h3 className="text-xl font-bold text-fuchsia-300 group-hover:text-fuchsia-200 drop-shadow-[0_0_15px_rgba(232,121,249,0.6)]">
                Accounts
              </h3>
            </div>
            <svg className="w-6 h-6 text-fuchsia-400 group-hover:translate-x-1 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7l5 5m0 0l-5 5m5-5H6" />
            </svg>
          </div>
          <p className="text-slate-300">
            Manage your wallets and exchange accounts, sync balances, and view connection status.
          </p>
        </Link>
      </div>
    </AppLayout>
  );
}
