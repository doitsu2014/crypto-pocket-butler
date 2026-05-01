import { auth } from "@/auth";
import { redirect } from "next/navigation";
import AppLayout from "@/components/AppLayout";
import Link from "next/link";

export default async function AdminPage() {
  const session = await auth();

  if (!session?.user) {
    redirect("/auth/signin");
  }

  if (!session.roles?.includes("administrator")) {
    redirect("/dashboard");
  }

  return (
    <AppLayout userEmail={session.user.email}>
      <div className="max-w-4xl">
        <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-fuchsia-500/40 shadow-[0_0_40px_rgba(217,70,239,0.4)] rounded-2xl p-6">
          <h2 className="text-3xl font-extrabold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-purple-300 bg-clip-text text-transparent mb-4 drop-shadow-[0_0_20px_rgba(232,121,249,0.6)]">
            Administration
          </h2>
          <p className="text-slate-200 mb-6 drop-shadow-[0_0_10px_rgba(226,232,240,0.3)]">
            System-wide configuration shared across all users.
          </p>

          <div className="space-y-6">
            {/* EVM Configuration Section */}
            <div>
              <h3 className="text-xl font-bold text-cyan-300 mb-4 drop-shadow-[0_0_15px_rgba(103,232,249,0.6)]">
                EVM Configuration
              </h3>
              <p className="text-slate-400 text-sm mb-4">
                Manage EVM chains and token registries used during wallet account synchronisation.
              </p>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <Link
                  href="/admin/evm-chains"
                  className="group flex items-start gap-4 p-4 rounded-xl bg-slate-900/50 border border-slate-700/40 hover:border-fuchsia-500/50 hover:bg-slate-900/70 transition-all"
                >
                  <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-violet-500/30 to-fuchsia-500/30 flex items-center justify-center shrink-0 group-hover:from-violet-500/50 group-hover:to-fuchsia-500/50 transition-all">
                    <svg className="w-5 h-5 text-violet-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
                    </svg>
                  </div>
                  <div>
                    <p className="font-semibold text-slate-100 group-hover:text-fuchsia-200 transition-colors">EVM Chains</p>
                    <p className="text-slate-400 text-sm mt-0.5">Configure chains and their RPC URLs</p>
                  </div>
                </Link>

                <Link
                  href="/admin/evm-tokens"
                  className="group flex items-start gap-4 p-4 rounded-xl bg-slate-900/50 border border-slate-700/40 hover:border-fuchsia-500/50 hover:bg-slate-900/70 transition-all"
                >
                  <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-cyan-500/30 to-blue-500/30 flex items-center justify-center shrink-0 group-hover:from-cyan-500/50 group-hover:to-blue-500/50 transition-all">
                    <svg className="w-5 h-5 text-cyan-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                  </div>
                  <div>
                    <p className="font-semibold text-slate-100 group-hover:text-fuchsia-200 transition-colors">EVM Tokens</p>
                    <p className="text-slate-400 text-sm mt-0.5">Manage ERC-20 tokens tracked during sync</p>
                  </div>
                </Link>
              </div>
            </div>

            {/* Solana Configuration Section */}
            <div>
              <h3 className="text-xl font-bold text-emerald-300 mb-4 drop-shadow-[0_0_15px_rgba(52,211,153,0.6)]">
                Solana Configuration
              </h3>
              <p className="text-slate-400 text-sm mb-4">
                Manage the SPL token registry used during Solana wallet account synchronisation.
              </p>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <Link
                  href="/admin/solana-tokens"
                  className="group flex items-start gap-4 p-4 rounded-xl bg-slate-900/50 border border-slate-700/40 hover:border-fuchsia-500/50 hover:bg-slate-900/70 transition-all"
                >
                  <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-emerald-500/30 to-teal-500/30 flex items-center justify-center shrink-0 group-hover:from-emerald-500/50 group-hover:to-teal-500/50 transition-all">
                    <svg className="w-5 h-5 text-emerald-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                  </div>
                  <div>
                    <p className="font-semibold text-slate-100 group-hover:text-fuchsia-200 transition-colors">Solana Tokens</p>
                    <p className="text-slate-400 text-sm mt-0.5">Manage SPL tokens tracked during sync</p>
                  </div>
                </Link>
              </div>
            </div>
          </div>
        </div>
      </div>
    </AppLayout>
  );
}
