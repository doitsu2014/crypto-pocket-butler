import { auth } from "@/auth";
import { redirect } from "next/navigation";
import PortfolioDetailClient from "./components/PortfolioDetailClient";
import AppLayout from "@/components/AppLayout";

interface PageProps {
  params: Promise<{
    id: string;
  }>;
}

export default async function PortfolioDetailPage({ params }: PageProps) {
  const session = await auth();

  if (!session?.user) {
    redirect("/auth/signin");
  }

  const { id } = await params;

  return (
    <AppLayout userEmail={session.user.email}>
      <PortfolioDetailClient portfolioId={id} />
    </AppLayout>
  );
}
