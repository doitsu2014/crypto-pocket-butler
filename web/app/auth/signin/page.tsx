import { signIn } from "@/auth";
import { redirect } from "next/navigation";

export default async function SignInPage({
  searchParams,
}: {
  searchParams: Promise<{ callbackUrl?: string }>;
}) {
  const { callbackUrl } = await searchParams;

  return (
    <div className="min-h-screen flex items-center justify-center bg-black relative overflow-hidden">
      {/* Intense neon animated background */}
      <div className="absolute inset-0 bg-[url('data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNjAiIGhlaWdodD0iNjAiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+PGRlZnM+PHBhdHRlcm4gaWQ9ImdyaWQiIHdpZHRoPSI2MCIgaGVpZ2h0PSI2MCIgcGF0dGVyblVuaXRzPSJ1c2VyU3BhY2VPblVzZSI+PHBhdGggZD0iTSAxMCAwIEwgMCAwIDAgMTAiIGZpbGw9Im5vbmUiIHN0cm9rZT0icmdiYSgxMzksIDkyLCAyNDYsIDAuMykiIHN0cm9rZS13aWR0aD0iMSIvPjwvcGF0dGVybj48L2RlZnM+PHJlY3Qgd2lkdGg9IjEwMCUiIGhlaWdodD0iMTAwJSIgZmlsbD0idXJsKCNncmlkKSIvPjwvc3ZnPg==')] opacity-40"></div>
      
      {/* Multiple glowing orbs with intense neon */}
      <div className="absolute top-0 left-1/4 w-[600px] h-[600px] bg-fuchsia-500/40 rounded-full blur-[150px] animate-pulse"></div>
      <div className="absolute bottom-0 right-1/4 w-[600px] h-[600px] bg-cyan-500/40 rounded-full blur-[150px] animate-pulse" style={{ animationDelay: '1s' }}></div>
      <div className="absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 w-[500px] h-[500px] bg-violet-500/30 rounded-full blur-[150px] animate-pulse" style={{ animationDelay: '2s' }}></div>
      
      <div className="relative max-w-md w-full space-y-8 bg-slate-950/90 backdrop-blur-xl p-10 rounded-2xl border-2 border-fuchsia-500/50 shadow-[0_0_80px_rgba(217,70,239,0.6)]">
        <div className="text-center">
          <div className="inline-block p-4 rounded-2xl bg-gradient-to-br from-fuchsia-500/30 to-violet-500/30 border-2 border-fuchsia-400/50 mb-4 shadow-[0_0_40px_rgba(217,70,239,0.8)] animate-pulse">
            <svg
              className="h-14 w-14 text-fuchsia-300 drop-shadow-[0_0_15px_rgba(232,121,249,0.9)]"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
              />
            </svg>
          </div>
          <h2 className="text-4xl font-extrabold drop-shadow-[0_0_20px_rgba(168,85,247,0.8)]">
            <span className="bg-gradient-to-r from-fuchsia-300 via-purple-300 to-cyan-300 bg-clip-text text-transparent animate-pulse">
              Crypto Pocket Butler
            </span>
          </h2>
          <p className="mt-2 text-sm text-slate-300 drop-shadow-[0_0_10px_rgba(148,163,184,0.5)]">
            Sign in to manage your crypto portfolio
          </p>
        </div>

        <form
          action={async () => {
            "use server";
            await signIn("keycloak", {
              redirectTo: callbackUrl || "/dashboard",
            });
          }}
          className="mt-8 space-y-6"
        >
          <button
            type="submit"
            className="group relative w-full flex justify-center py-4 px-4 border-2 border-fuchsia-500 text-base font-bold rounded-xl text-white bg-gradient-to-r from-fuchsia-600 via-purple-600 to-violet-600 hover:from-fuchsia-500 hover:via-purple-500 hover:to-violet-500 shadow-[0_0_40px_rgba(217,70,239,0.8)] hover:shadow-[0_0_60px_rgba(217,70,239,1)] transition-all duration-300 transform hover:scale-105 focus:outline-none focus:ring-4 focus:ring-fuchsia-500/50 animate-pulse"
          >
            <span className="absolute left-0 inset-y-0 flex items-center pl-4">
              <svg
                className="h-6 w-6 text-fuchsia-200 group-hover:text-fuchsia-100 drop-shadow-[0_0_10px_rgba(245,208,254,0.8)]"
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 20 20"
                fill="currentColor"
                aria-hidden="true"
              >
                <path
                  fillRule="evenodd"
                  d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z"
                  clipRule="evenodd"
                />
              </svg>
            </span>
            Sign in with Keycloak
          </button>
        </form>

        <div className="text-center text-xs text-slate-400 mt-4">
          <p className="flex items-center justify-center gap-2">
            <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse shadow-[0_0_10px_rgba(74,222,128,0.8)]"></span>
            <span className="drop-shadow-[0_0_5px_rgba(148,163,184,0.5)]">Secured with OAuth 2.0 + PKCE</span>
          </p>
        </div>
      </div>
    </div>
  );
}
