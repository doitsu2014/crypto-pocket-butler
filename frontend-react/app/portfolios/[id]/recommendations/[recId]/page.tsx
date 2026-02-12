import { auth } from "@/auth";
import { redirect } from "next/navigation";
import Link from "next/link";
import RecommendationDetailClient from "./components/RecommendationDetailClient";

interface PageProps {
  params: Promise<{
    id: string;
    recId: string;
  }>;
}

export default async function RecommendationDetailPage({ params }: PageProps) {
  const session = await auth();

  if (!session?.user) {
    redirect("/auth/signin");
  }

  const { id, recId } = await params;

  return (
    <div className="min-h-screen bg-black relative overflow-hidden">
      {/* Cyberpunk neon background pattern */}
      <div className="absolute inset-0 bg-[url('data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNjAiIGhlaWdodD0iNjAiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+PGRlZnM+PHBhdHRlcm4gaWQ9ImdyaWQiIHdpZHRoPSI2MCIgaGVpZ2h0PSI2MCIgcGF0dGVyblVuaXRzPSJ1c2VyU3BhY2VPblVzZSI+PHBhdGggZD0iTSAxMCAwIEwgMCAwIDAgMTAiIGZpbGw9Im5vbmUiIHN0cm9rZT0icmdiYSgxMzksIDkyLCAyNDYsIDAuMikiIHN0cm9rZS13aWR0aD0iMSIvPjwvcGF0dGVybj48L2RlZnM+PHJlY3Qgd2lkdGg9IjEwMCUiIGhlaWdodD0iMTAwJSIgZmlsbD0idXJsKCNncmlkKSIvPjwvc3ZnPg==')] opacity-25"></div>
      
      {/* Ambient glowing orbs */}
      <div className="absolute top-0 right-0 w-[450px] h-[450px] bg-fuchsia-500/15 rounded-full blur-[110px]"></div>
      <div className="absolute bottom-0 left-0 w-[450px] h-[450px] bg-cyan-500/15 rounded-full blur-[110px]"></div>
      
      <nav className="relative bg-slate-950/80 backdrop-blur-xl border-b-2 border-fuchsia-500/30 shadow-[0_0_20px_rgba(217,70,239,0.25)]">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
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
              <Link
                href="/portfolios"
                className="text-sm text-cyan-300 hover:text-cyan-200 transition-colors drop-shadow-[0_0_6px_rgba(103,232,249,0.4)]"
              >
                Portfolios
              </Link>
              <Link
                href={`/portfolios/${id}`}
                className="text-sm text-cyan-300 hover:text-cyan-200 transition-colors drop-shadow-[0_0_6px_rgba(103,232,249,0.4)]"
              >
                Portfolio
              </Link>
              <Link
                href={`/portfolios/${id}/recommendations`}
                className="text-sm text-cyan-300 hover:text-cyan-200 transition-colors drop-shadow-[0_0_6px_rgba(103,232,249,0.4)]"
              >
                Recommendations
              </Link>
              <Link
                href="/dashboard"
                className="text-sm text-cyan-300 hover:text-cyan-200 transition-colors drop-shadow-[0_0_6px_rgba(103,232,249,0.4)]"
              >
                Dashboard
              </Link>
            </div>
          </div>
        </div>
      </nav>

      <main className="relative max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
        <div className="px-4 py-6 sm:px-0">
          <RecommendationDetailClient portfolioId={id} recommendationId={recId} />
        </div>
      </main>
    </div>
  );
}
