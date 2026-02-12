import { auth } from "@/auth";
import { NextResponse } from "next/server";

const BACKEND_URL = process.env.NEXT_PUBLIC_BACKEND_URL || "http://localhost:3000";

export async function POST(
  request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  const session = await auth();

  if (!session?.accessToken) {
    return NextResponse.json(
      { error: "Unauthorized" },
      { status: 401 }
    );
  }

  const { id } = await params;

  try {
    const response = await fetch(`${BACKEND_URL}/api/v1/accounts/${id}/sync`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${session.accessToken}`,
        "Content-Type": "application/json",
      },
    });

    if (!response.ok) {
      const errorText = await response.text();
      return NextResponse.json(
        { error: `Backend error: ${response.statusText}`, details: errorText },
        { status: response.status }
      );
    }

    const data = await response.json();
    return NextResponse.json(data);
  } catch (error) {
    return NextResponse.json(
      { 
        error: "Failed to sync account", 
        details: error instanceof Error ? error.message : String(error) 
      },
      { status: 500 }
    );
  }
}
