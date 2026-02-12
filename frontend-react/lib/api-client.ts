/**
 * Utility functions for making authenticated API calls to the backend
 */

const BACKEND_URL = process.env.NEXT_PUBLIC_BACKEND_URL || "http://localhost:3001";

export interface ApiClientOptions {
  method?: "GET" | "POST" | "PUT" | "DELETE" | "PATCH";
  body?: unknown;
  headers?: HeadersInit;
}

export type ApiErrorType = "network" | "auth" | "validation" | "server" | "unknown";

export class ApiError extends Error {
  public type: ApiErrorType;
  public statusCode?: number;
  public details?: unknown;

  constructor(message: string, type: ApiErrorType, statusCode?: number, details?: unknown) {
    super(message);
    this.name = "ApiError";
    this.type = type;
    this.statusCode = statusCode;
    this.details = details;
  }
}

function parseErrorResponse(response: Response, errorData: any): ApiError {
  const status = response.status;
  let message = errorData?.error || errorData?.message || response.statusText || "An error occurred";
  let type: ApiErrorType = "unknown";

  // Categorize errors by status code
  if (status === 401 || status === 403) {
    type = "auth";
    message = errorData?.error || "Authentication failed. Please sign in again.";
  } else if (status === 400 || status === 422) {
    type = "validation";
    message = errorData?.error || "Invalid input. Please check your data.";
  } else if (status >= 500) {
    type = "server";
    message = errorData?.error || "Server error. Please try again later.";
  } else if (status >= 400) {
    type = "unknown";
  }

  return new ApiError(message, type, status, errorData);
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

  try {
    // Use the Next.js API route as a proxy to handle authentication
    const response = await fetch(`/api/backend${endpoint}`, config);

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ 
        error: `Failed to parse error response (HTTP ${response.status})` 
      }));
      throw parseErrorResponse(response, errorData);
    }

    return response.json();
  } catch (error) {
    // Handle network errors
    if (error instanceof TypeError && error.message.includes("fetch")) {
      throw new ApiError("Network error. Please check your connection.", "network");
    }
    // Re-throw ApiError as-is
    if (error instanceof ApiError) {
      throw error;
    }
    // Wrap other errors
    throw new ApiError(
      error instanceof Error ? error.message : "An unexpected error occurred",
      "unknown"
    );
  }
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

  try {
    const response = await fetch(`${BACKEND_URL}${endpoint}`, config);

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ 
        error: `Failed to parse error response (HTTP ${response.status})` 
      }));
      throw parseErrorResponse(response, errorData);
    }

    return response.json();
  } catch (error) {
    // Handle network errors
    if (error instanceof TypeError && error.message.includes("fetch")) {
      throw new ApiError("Network error. Please check your connection.", "network");
    }
    // Re-throw ApiError as-is
    if (error instanceof ApiError) {
      throw error;
    }
    // Wrap other errors
    throw new ApiError(
      error instanceof Error ? error.message : "An unexpected error occurred",
      "unknown"
    );
  }
}
