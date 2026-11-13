import { afterEach, describe, expect, it } from "vitest";
import { ApiError, fetchSupportMetadata, listMyGrants, requestSupportGrant } from "../api/client";
import { activeGrant, supportMetadata } from "./fixtures";
import { server } from "./server";
import {
  supportGrantConflictHandlers,
  supportGrantForbiddenHandlers,
  supportGrantHandlers,
  supportGrantRateLimitedHandlers,
  supportGrantUnavailableHandlers,
} from "./supportHandlers";

afterEach(() => {
  server.resetHandlers();
});

describe("support grant handlers", () => {
  it("answers the three §8 routes from one handler set", async () => {
    const grants = [activeGrant({ id: "grant-2" }), activeGrant({ id: "grant-1" })];
    const created = activeGrant({ id: "grant-created" });
    server.use(
      ...supportGrantHandlers({
        grants,
        created,
        metadata: supportMetadata({ displayName: "Shared Fixture" }),
      }),
    );

    const listed = await listMyGrants("token");
    expect(listed.map((grant) => grant.id)).toEqual(["grant-2", "grant-1"]);

    const metadata = await fetchSupportMetadata("grant-2", "token");
    expect(metadata.displayName).toBe("Shared Fixture");
    expect(metadata.accounts).toHaveLength(2);

    const made = await requestSupportGrant({ userId: "user-1", reason: "Need to check" }, "token");
    expect(made.id).toBe("grant-created");
  });

  it("delays every answer by the requested time", async () => {
    server.use(...supportGrantHandlers({ grants: [], delayMs: 50 }));

    const started = Date.now();
    await listMyGrants("token");

    expect(Date.now() - started).toBeGreaterThanOrEqual(40);
  });

  it("answers a duplicate request with 409", async () => {
    server.use(...supportGrantConflictHandlers());

    const error = await requestSupportGrant({ userId: "user-1", reason: "Twice" }, "token").catch(
      (thrown: unknown) => thrown,
    );

    expect(error).toBeInstanceOf(ApiError);
    const problem = (error as ApiError).problem;
    expect((error as ApiError).status).toBe(409);
    expect(problem.title).toBe("A support grant is already open");
    expect(problem.correlationId).toBe("corr-support-409");
  });

  it("answers a read without a live grant with 403", async () => {
    server.use(...supportGrantForbiddenHandlers());

    const error = await fetchSupportMetadata("grant-1", "token").catch((thrown: unknown) => thrown);

    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).status).toBe(403);
    expect((error as ApiError).problem.correlationId).toBe("corr-support-403");
  });

  it("answers a burst with 429 and a Retry-After", async () => {
    server.use(...supportGrantRateLimitedHandlers());

    const error = await listMyGrants("token").catch((thrown: unknown) => thrown);

    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).status).toBe(429);
    expect((error as ApiError).retryAfterSeconds).toBe(30);
  });

  it("answers an outage with 503 and a Retry-After", async () => {
    server.use(...supportGrantUnavailableHandlers());

    const error = await fetchSupportMetadata("grant-1", "token").catch((thrown: unknown) => thrown);

    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).status).toBe(503);
    expect((error as ApiError).retryAfterSeconds).toBe(60);
  });
});
