import { screen } from "@testing-library/react";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import type { AdminUserRecord as AdminUserRecordType } from "../../api/types";
import { renderWithQuery } from "../../test/fixtures";
import { AdminUserRecord } from "./AdminUserRecord";

function record(overrides: Partial<AdminUserRecordType> = {}): AdminUserRecordType {
  return {
    id: "user-1",
    displayName: "Test User",
    email: "user@example.com",
    emailVerified: true,
    status: "active",
    role: "admin",
    createdAt: "2026-11-01T10:00:00.000Z",
    updatedAt: "2026-11-02T10:00:00.000Z",
    ...overrides,
  };
}

function renderRecord(overrides: Partial<AdminUserRecordType> = {}) {
  return renderWithQuery(
    <AdminUserRecord
      record={record(overrides)}
      signedInEmail="admin@example.test"
      token="admin-token"
      onChanged={vi.fn()}
    />,
  );
}

describe("AdminUserRecord", () => {
  it("renders identity, role, status and timestamps", () => {
    renderRecord();

    expect(screen.getByTestId("admin-user-record")).toBeInTheDocument();
    expect(screen.getByTestId("admin-user-name")).toHaveTextContent("Test User");
    expect(screen.getByTestId("admin-user-email")).toHaveTextContent("user@example.com");
    expect(screen.getByTestId("admin-user-verified")).toHaveTextContent("Verified");
    expect(screen.getByTestId("admin-user-id")).toHaveTextContent("user-1");
    expect(screen.getByTestId("role-badge")).toHaveTextContent(/admin/i);
    expect(screen.getByTestId("admin-user-status")).toHaveTextContent("Active");
    expect(screen.getByTestId("admin-user-created")).toHaveTextContent("2026");
    expect(screen.getByTestId("admin-user-updated")).toHaveTextContent("2026");
  });

  it("renders an em dash for a missing name and an Unverified badge", () => {
    renderRecord({ displayName: undefined, emailVerified: false });

    expect(screen.getByTestId("admin-user-name")).toHaveTextContent("—");
    expect(screen.getByTestId("admin-user-verified")).toHaveTextContent("Unverified");
  });

  it.each([
    ["pending", "Pending"],
    ["active", "Active"],
    ["disabled", "Disabled"],
    ["pending_deletion", "Pending deletion"],
    ["deleted", "Deleted"],
  ] as const)("renders the %s status label", (status, label) => {
    renderRecord({ status });
    expect(screen.getByTestId("admin-user-status")).toHaveTextContent(label);
  });

  it("renders no entitlement control and no financial field", () => {
    renderRecord();

    const text = screen.getByTestId("admin-user-record").textContent ?? "";
    expect(text).not.toMatch(/holding|balance|valuation|transaction|subscription|seat|billing/i);
    expect(text).not.toMatch(/wallet address|provider subject/i);
    expect(screen.queryByRole("button", { name: /entitlement/i })).not.toBeInTheDocument();
  });

  it("has no accessibility violations", async () => {
    const { container } = renderRecord();
    expect((await axe(container)).violations).toEqual([]);
  });
});
