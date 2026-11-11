import { setupServer } from "msw/node";
import { defaultHandlers } from "./handlers";

/**
 * Feature-local MSW server (unit-test-instructions.md "Mocking guidance").
 *
 * Consuming suites start it in `beforeAll`, reset handlers in `afterEach`, and
 * close it in `afterAll`. The global test setup is intentionally untouched so
 * the pre-existing fetch-stub suites keep their own harness.
 */
export const server = setupServer(...defaultHandlers);
