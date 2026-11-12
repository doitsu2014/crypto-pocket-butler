import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import { ApiError } from "../../api/client";
import { AdminUserLookup } from "./AdminUserLookup";

function problem(status: number, title: string, correlationId = "corr-lookup") {
  return new ApiError({
    type: `https://errors.cpb/identity/${status}`,
    title,
    status,
    correlationId,
  });
}

describe("AdminUserLookup", () => {
  it("submits the typed exact email", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<AdminUserLookup onSubmit={onSubmit} loading={false} />);

    await user.type(screen.getByTestId("admin-lookup-email"), " user@example.com ");
    await user.click(screen.getByTestId("admin-lookup-submit"));

    expect(onSubmit).toHaveBeenCalledWith("user@example.com");
  });

  it("refuses an empty address before any request", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<AdminUserLookup onSubmit={onSubmit} loading={false} />);

    await user.click(screen.getByTestId("admin-lookup-submit"));

    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByTestId("admin-lookup-email-error")).toHaveTextContent(
      "Enter the user's email address.",
    );
  });

  it("refuses a malformed address before any request", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<AdminUserLookup onSubmit={onSubmit} loading={false} />);

    await user.type(screen.getByTestId("admin-lookup-email"), "not-an-address");
    await user.click(screen.getByTestId("admin-lookup-submit"));

    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByTestId("admin-lookup-email-error")).toHaveTextContent(
      "Enter a valid email address.",
    );
  });

  it("renders the no-such-address sentence for a 404 and no record", () => {
    render(<AdminUserLookup onSubmit={vi.fn()} loading={false} notFound />);

    expect(screen.getByTestId("admin-lookup-not-found")).toHaveTextContent(
      "No user has that address.",
    );
    expect(screen.queryByTestId("admin-user-record")).not.toBeInTheDocument();
  });

  it("renders the not-an-administrator line with the correlation id for a 403", () => {
    render(
      <AdminUserLookup
        onSubmit={vi.fn()}
        loading={false}
        error={problem(403, "Forbidden", "corr-403")}
      />,
    );

    const alert = screen.getByTestId("admin-lookup-error");
    expect(alert).toHaveTextContent("Your session is not an administrator.");
    expect(alert).toHaveTextContent("corr-403");
  });

  it("renders the provider-failure sentence with a retry for a 503", async () => {
    const user = userEvent.setup();
    const onRetry = vi.fn();
    render(
      <AdminUserLookup
        onSubmit={vi.fn()}
        loading={false}
        error={problem(503, "Service unavailable", "corr-503")}
        onRetry={onRetry}
      />,
    );

    const alert = screen.getByTestId("admin-lookup-error");
    expect(alert).toHaveTextContent("This user cannot be looked up right now.");
    expect(alert).toHaveTextContent("corr-503");

    await user.click(screen.getByTestId("admin-lookup-retry"));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it("disables the form while a lookup is in flight", () => {
    render(<AdminUserLookup onSubmit={vi.fn()} loading />);

    expect(screen.getByTestId("admin-lookup-submit")).toBeDisabled();
    expect(screen.getByText("Looking up…")).toBeInTheDocument();
  });

  it("has no accessibility violations", async () => {
    const { container } = render(
      <AdminUserLookup
        onSubmit={vi.fn()}
        loading={false}
        error={problem(403, "Forbidden", "corr-403")}
      />,
    );
    expect((await axe(container)).violations).toEqual([]);
  });
});

describe("AdminUserLookup submission state", () => {
  it("keeps the typed address after a server refusal", async () => {
    const user = userEvent.setup();
    render(<AdminUserLookup onSubmit={vi.fn()} loading={false} />);

    await user.type(screen.getByTestId("admin-lookup-email"), "user@example.com");
    await user.click(screen.getByTestId("admin-lookup-submit"));

    await waitFor(() =>
      expect(screen.getByTestId("admin-lookup-email")).toHaveValue("user@example.com"),
    );
  });
});
