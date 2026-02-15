import { auth } from "@/auth";
import { redirect } from "next/navigation";
import RecommendationsClient from "./components/RecommendationsClient";
import AppLayout from "@/components/AppLayout";

interface PageProps {
  params: Promise<{
    id: string;
  }>;
}

export default async function RecommendationsPage({ params }: PageProps) {
  const session = await auth();

  if (!session?.user) {
    redirect("/auth/signin");
  }

  const { id } = await params;

  return (
    <AppLayout userEmail={session.user.email}>
      <RecommendationsClient portfolioId={id} />
    </AppLayout>
  );
}
