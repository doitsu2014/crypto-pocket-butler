import { auth } from "@/auth";
import { redirect } from "next/navigation";
import AppLayout from "@/components/AppLayout";

export default async function SettingsPage() {
  const session = await auth();

  if (!session?.user) {
    redirect("/auth/signin");
  }

  return (
    <AppLayout userEmail={session.user.email}>
      <div className="max-w-4xl">
        <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-fuchsia-500/40 shadow-[0_0_40px_rgba(217,70,239,0.4)] rounded-2xl p-6">
          <h2 className="text-3xl font-extrabold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-purple-300 bg-clip-text text-transparent mb-4 drop-shadow-[0_0_20px_rgba(232,121,249,0.6)]">
            Settings
          </h2>
          <p className="text-slate-200 mb-6 drop-shadow-[0_0_10px_rgba(226,232,240,0.3)]">
            Manage your application settings and preferences.
          </p>

          <div className="space-y-6">
            {/* User Profile Section */}
            <div className="border-b border-slate-700/50 pb-6">
              <h3 className="text-xl font-bold text-cyan-300 mb-4 drop-shadow-[0_0_15px_rgba(103,232,249,0.6)]">
                User Profile
              </h3>
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-slate-300">Email</span>
                  <span className="text-slate-100 font-medium">{session.user.email}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-slate-300">Name</span>
                  <span className="text-slate-100 font-medium">{session.user.name || "Not set"}</span>
                </div>
              </div>
            </div>

            {/* About Section */}
            <div>
              <h3 className="text-xl font-bold text-cyan-300 mb-4 drop-shadow-[0_0_15px_rgba(103,232,249,0.6)]">
                About
              </h3>
              <div className="space-y-2">
                <p className="text-slate-300">
                  <span className="font-semibold">Version:</span> 1.0.0
                </p>
                <p className="text-slate-300">
                  <span className="font-semibold">Description:</span> Crypto portfolio management with intelligent rebalancing
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </AppLayout>
  );
}
