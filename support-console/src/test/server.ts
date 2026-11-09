import { setupServer } from "msw/node";

/**
 * The C2 API is faked at the network boundary with msw, so component tests
 * exercise the real fetch path (bearer header, RFC 7807 mapping, 401/403).
 * Individual suites register the handlers they need; an unhandled request
 * fails the test rather than silently passing.
 */
export const server = setupServer();
