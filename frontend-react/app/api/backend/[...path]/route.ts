import { auth } from "@/auth";
import { NextRequest, NextResponse } from "next/server";

const BACKEND_URL = process.env.NEXT_PUBLIC_BACKEND_URL || "http://localhost:3001";

/**
 * Catch-all route handler for proxying requests to the Rust backend.
 * Automatically attaches the access token from NextAuth session.
 * 
 * Supports all HTTP methods: GET, POST, PUT, DELETE, PATCH, etc.
 * Forwards query parameters, headers, and request body to the backend.
 * 
 * Note: More specific routes (e.g., /api/backend/me/route.ts) take precedence
 * over this catch-all. This acts as a fallback for any endpoint that doesn't
 * have a dedicated route handler.
 */
async function handleRequest(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const session = await auth();

  if (!session?.accessToken) {
    return NextResponse.json(
      { error: "Unauthorized" },
      { status: 401 }
    );
  }

  const { path } = await params;
  // Reconstruct the path from the catch-all segments
  const backendPath = path.join("/");
  
  // Get query string from the request URL
  const { searchParams } = new URL(request.url);
  const queryString = searchParams.toString();
  const backendUrl = `${BACKEND_URL}/api/${backendPath}${queryString ? `?${queryString}` : ""}`;

  try {
    // Prepare headers
    const headers: HeadersInit = {
      Authorization: `Bearer ${session.accessToken}`,
    };

    // Copy Content-Type if present
    const contentType = request.headers.get("content-type");
    if (contentType) {
      headers["Content-Type"] = contentType;
    }

    // Prepare fetch options
    const fetchOptions: RequestInit = {
      method: request.method,
      headers,
    };

    // Include body for methods that support it
    if (request.method !== "GET" && request.method !== "HEAD") {
      try {
        const body = await request.text();
        if (body) {
          fetchOptions.body = body;
        }
      } catch (error) {
        // If reading body fails, log with context and continue without it
        console.warn(
          `Failed to read request body for ${request.method} ${backendPath}:`,
          error instanceof Error ? error.message : String(error)
        );
      }
    }

    const response = await fetch(backendUrl, fetchOptions);

    // Handle 204 No Content responses first (before consuming body)
    if (response.status === 204) {
      return new NextResponse(null, { status: 204 });
    }

    if (!response.ok) {
      // Check if error response is JSON or text
      const errorContentType = response.headers.get("content-type");
      let errorDetails;
      if (errorContentType?.includes("application/json")) {
        const errorJson = await response.json();
        errorDetails = JSON.stringify(errorJson);
      } else {
        errorDetails = await response.text();
      }
      return NextResponse.json(
        { error: `Backend error: ${response.statusText}`, details: errorDetails },
        { status: response.status }
      );
    }

    // For successful responses with content, check content type
    const responseContentType = response.headers.get("content-type");
    if (responseContentType?.includes("application/json")) {
      const data = await response.json();
      return NextResponse.json(data, { status: response.status });
    } else {
      // For non-JSON responses, return as text
      const text = await response.text();
      return new NextResponse(text, { 
        status: response.status,
        headers: { "Content-Type": responseContentType || "text/plain" }
      });
    }
  } catch (error) {
    return NextResponse.json(
      { 
        error: "Failed to proxy request to backend",
        endpoint: backendPath,
        details: error instanceof Error ? error.message : String(error) 
      },
      { status: 500 }
    );
  }
}

// Export all HTTP methods
export const GET = handleRequest;
export const POST = handleRequest;
export const PUT = handleRequest;
export const DELETE = handleRequest;
export const PATCH = handleRequest;
