import { auth } from "@/auth";
import { redirect } from "next/navigation";
import SnapshotsClient from "./components/SnapshotsClient";
import AppLayout from "@/components/AppLayout";

interface PageProps {
  params: Promise<{
    id: string;
  }>;
}

export default async function SnapshotsPage({ params }: PageProps) {
  const session = await auth();

  if (!session?.user) {
    redirect("/auth/signin");
  }

  const { id } = await params;

  return (
    <AppLayout userEmail={session.user.email}>
      <SnapshotsClient portfolioId={id} />
    </AppLayout>
  );
}
