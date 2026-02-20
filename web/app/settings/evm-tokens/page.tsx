import { auth } from "@/auth";
import { redirect } from "next/navigation";
import AppLayout from "@/components/AppLayout";
import Link from "next/link";
import EvmTokensClient from "./components/EvmTokensClient";

export default async function EvmTokensPage() {
  const session = await auth();

  if (!session?.user) {
    redirect("/auth/signin");
  }

  return (
    <AppLayout userEmail={session.user.email}>
      <div className="max-w-5xl">
        <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-fuchsia-500/40 shadow-[0_0_40px_rgba(217,70,239,0.4)] rounded-2xl p-6">
          {/* Breadcrumb */}
          <div className="flex items-center gap-2 text-sm text-slate-400 mb-6">
            <Link href="/settings" className="hover:text-fuchsia-300 transition-colors">
              Settings
            </Link>
            <span>/</span>
            <span className="text-slate-200">EVM Tokens</span>
          </div>

          <h2 className="text-3xl font-extrabold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-purple-300 bg-clip-text text-transparent mb-2 drop-shadow-[0_0_20px_rgba(232,121,249,0.6)]">
            EVM Tokens
          </h2>
          <p className="text-slate-400 text-sm mb-6">
            Manage the ERC-20 tokens that are checked during wallet account sync.
            Only active tokens will be scanned when syncing account balances.
          </p>

          <EvmTokensClient />
        </div>
      </div>
    </AppLayout>
  );
}
