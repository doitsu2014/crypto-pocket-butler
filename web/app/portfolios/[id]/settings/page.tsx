import { auth } from "@/auth";
import { redirect } from "next/navigation";
import SettingsClient from "./components/SettingsClient";
import AppLayout from "@/components/AppLayout";

interface PageProps {
  params: Promise<{
    id: string;
  }>;
}

export default async function PortfolioSettingsPage({ params }: PageProps) {
  const session = await auth();

  if (!session?.user) {
    redirect("/auth/signin");
  }

  const { id } = await params;

  return (
    <AppLayout userEmail={session.user.email}>
      <SettingsClient portfolioId={id} />
    </AppLayout>
  );
}
