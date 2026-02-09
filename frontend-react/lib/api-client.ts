/**
 * Utility functions for making authenticated API calls to the backend
 */

const BACKEND_URL = process.env.NEXT_PUBLIC_BACKEND_URL || "http://localhost:3000";

export interface ApiClientOptions {
  method?: "GET" | "POST" | "PUT" | "DELETE" | "PATCH";
  body?: unknown;
  headers?: HeadersInit;
}

/**
 * Make an authenticated API call through the Next.js API proxy
 * This ensures the access token is securely handled server-side
 */
export async function apiClient<T>(
  endpoint: string,
  options: ApiClientOptions = {}
): Promise<T> {
  const { method = "GET", body, headers = {} } = options;

  const config: RequestInit = {
    method,
    headers: {
      "Content-Type": "application/json",
      ...headers,
    },
  };

  if (body) {
    config.body = JSON.stringify(body);
  }

  // Use the Next.js API route as a proxy to handle authentication
  const response = await fetch(`/api/backend${endpoint}`, config);

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ 
      error: `Failed to parse error response (HTTP ${response.status})` 
    }));
    throw new Error(errorData.error || `API request failed: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Direct backend client (for server-side use with access token)
 */
export async function directBackendClient<T>(
  endpoint: string,
  accessToken: string,
  options: ApiClientOptions = {}
): Promise<T> {
  const { method = "GET", body, headers = {} } = options;

  const config: RequestInit = {
    method,
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${accessToken}`,
      ...headers,
    },
  };

  if (body) {
    config.body = JSON.stringify(body);
  }

  const response = await fetch(`${BACKEND_URL}${endpoint}`, config);

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ 
      error: `Failed to parse error response (HTTP ${response.status})` 
    }));
    throw new Error(errorData.error || `API request failed: ${response.statusText}`);
  }

  return response.json();
}
