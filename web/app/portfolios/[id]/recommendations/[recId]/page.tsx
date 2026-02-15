import { auth } from "@/auth";
import { redirect } from "next/navigation";
import RecommendationDetailClient from "./components/RecommendationDetailClient";
import AppLayout from "@/components/AppLayout";

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
    <AppLayout userEmail={session.user.email}>
      <RecommendationDetailClient portfolioId={id} recommendationId={recId} />
    </AppLayout>
  );
}
