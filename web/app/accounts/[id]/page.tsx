import { Suspense } from "react";
import { LoadingSkeleton } from "@/components/Loading";
import AccountDetailClient from "./components/AccountDetailClient";

export default function AccountDetailPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <Suspense fallback={<LoadingSkeleton />}>
          <AccountDetailClient />
        </Suspense>
      </div>
    </div>
  );
}
