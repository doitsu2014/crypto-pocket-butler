import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import AccountStatusRow from "./AccountStatusRow";
import CsvImport from "./CsvImport";
import ExchangeCredentialForm from "./ExchangeCredentialForm";
import WalletAddressForm from "./WalletAddressForm";
import WalletConnectButton from "./WalletConnectButton";
import { ApiError } from "../../api/client";
import { renderWithProviders } from "../../test/render";
import type { Account } from "../../api/types";

const account: Account = {
  id: "acc-1",
  kind: "exchange",
  label: "Binance main",
  status: "synced",
  lastSyncedAt: "2026-11-08T10:00:00Z",
};

describe("exchange credential form", () => {
  it("requires label, key and secret without storing anything", async () => {
    const addAccount = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(<ExchangeCredentialForm addAccount={addAccount} />);

    await user.click(screen.getByTestId("exchange-submit"));

    expect(screen.getByTestId("exchange-errors")).toHaveTextContent(/required/i);
    expect(addAccount).not.toHaveBeenCalled();
    expect(window.localStorage.length).toBe(0);
    expect(window.sessionStorage.length).toBe(0);
  });

  it("names the field on a 422 and keeps the label for a retry", async () => {
    const addAccount = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Validation failed",
        status: 422,
        correlationId: "corr-422",
        errors: [{ field: "apiKey", reason: "Key format is invalid" }],
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(<ExchangeCredentialForm addAccount={addAccount} />);

    await user.type(screen.getByTestId("exchange-label"), "Binance main");
    await user.type(screen.getByTestId("exchange-api-key"), "bad");
    await user.type(screen.getByTestId("exchange-api-secret"), "secret");
    await user.click(screen.getByTestId("exchange-submit"));

    expect(await screen.findByTestId("exchange-errors")).toHaveTextContent(
      "apiKey: Key format is invalid",
    );
    expect((screen.getByTestId("exchange-label") as HTMLInputElement).value).toBe(
      "Binance main",
    );
  });

  it("shows a verification failure and clears the secret fields on success only", async () => {
    const verify = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Verification failed",
        status: 422,
        correlationId: "corr-verify",
        errors: [{ field: "permissions", reason: "withdrawal permission detected" }],
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(
      <ExchangeCredentialForm
        addAccount={vi.fn().mockResolvedValue(account)}
        verify={verify}
      />,
    );

    await user.type(screen.getByTestId("exchange-label"), "Binance main");
    await user.type(screen.getByTestId("exchange-api-key"), "key");
    await user.type(screen.getByTestId("exchange-api-secret"), "secret");
    await user.click(screen.getByTestId("exchange-submit"));

    expect(await screen.findByTestId("exchange-verification-error")).toHaveTextContent(
      "withdrawal permission detected",
    );
    expect((screen.getByTestId("exchange-label") as HTMLInputElement).value).toBe(
      "Binance main",
    );
  });
});

describe("wallet address form", () => {
  it("names the supported networks and keeps a malformed address", async () => {
    const addWallet = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(<WalletAddressForm addWallet={addWallet} />);

    await user.type(screen.getByTestId("wallet-address"), "0xnotanaddress");
    await user.click(screen.getByTestId("wallet-submit"));

    const errors = screen.getByTestId("wallet-errors");
    expect(errors).toHaveTextContent("Ethereum");
    expect(errors).toHaveTextContent("BNB Smart Chain");
    expect(errors).toHaveTextContent("Solana");
    expect((screen.getByTestId("wallet-address") as HTMLInputElement).value).toBe(
      "0xnotanaddress",
    );
    expect(addWallet).not.toHaveBeenCalled();
  });

  it("submits a valid EVM address", async () => {
    const addWallet = vi.fn().mockResolvedValue({ ...account, kind: "wallet" });
    const user = userEvent.setup();
    renderWithProviders(<WalletAddressForm addWallet={addWallet} />);
    const address = `0x${"a".repeat(40)}`;

    await user.type(screen.getByTestId("wallet-address"), address);
    await user.click(screen.getByTestId("wallet-submit"));

    expect(addWallet).toHaveBeenCalledWith({
      venue: "ethereum",
      label: "My wallet",
      address,
    });
  });

  it("rejects a Solana address on the EVM network", async () => {
    const addWallet = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(<WalletAddressForm addWallet={addWallet} />);

    await user.type(screen.getByTestId("wallet-address"), "9xQeWvG816bUx9EPf6W5G5h1z4");
    await user.click(screen.getByTestId("wallet-submit"));

    expect(screen.getByTestId("wallet-errors")).toBeInTheDocument();
    expect(addWallet).not.toHaveBeenCalled();
  });
});

describe("WalletConnect", () => {
  it("offers no signing action and attaches a read-only session", async () => {
    const attach = vi.fn().mockResolvedValue({ ...account, kind: "walletconnect" });
    const user = userEvent.setup();
    renderWithProviders(
      <WalletConnectButton
        connect={vi.fn().mockResolvedValue({ address: "0xabc", topic: "t1", chain: "eip155:1" })}
        attach={attach}
      />,
    );

    expect(screen.getByTestId("walletconnect")).toHaveTextContent(/read-only/i);
    expect(screen.getByTestId("walletconnect")).not.toHaveTextContent(/sign|transfer/i);

    await user.click(screen.getByTestId("walletconnect-button"));

    expect(await screen.findByTestId("walletconnect-connected")).toBeInTheDocument();
    expect(attach).toHaveBeenCalledWith({
      venue: "eip155:1",
      label: "WalletConnect",
      address: "0xabc",
      topic: "t1",
    });
  });

  it("keeps the connect action available after a rejected or expired session", async () => {
    const connect = vi
      .fn()
      .mockRejectedValueOnce(new Error("Session rejected"))
      .mockResolvedValueOnce({ address: "0xabc", topic: "t2", chain: "eip155:1" });
    const user = userEvent.setup();
    renderWithProviders(
      <WalletConnectButton connect={connect} attach={vi.fn().mockResolvedValue(account)} />,
    );

    await user.click(screen.getByTestId("walletconnect-button"));
    expect(await screen.findByTestId("walletconnect-error")).toHaveTextContent(
      "Session rejected",
    );

    await user.click(screen.getByTestId("walletconnect-button"));
    expect(await screen.findByTestId("walletconnect-connected")).toBeInTheDocument();
  });
});

describe("CSV import", () => {
  it("shows accepted and skipped counts with per-line errors", async () => {
    const importFile = vi.fn().mockResolvedValue({
      id: "import-1",
      status: "complete",
      accepted: 8,
      skipped: 2,
      errors: [{ line: 4, reason: "unknown asset" }],
    });
    const user = userEvent.setup();
    renderWithProviders(<CsvImport importFile={importFile} pollInterval={0} />);

    await user.upload(
      screen.getByTestId("csv-file"),
      new File(["asset,quantity\nBTC,1\n"], "transactions.csv", { type: "text/csv" }),
    );
    await user.click(screen.getByTestId("csv-submit"));

    const result = await screen.findByTestId("csv-result");
    expect(result).toHaveTextContent("Accepted: 8");
    expect(result).toHaveTextContent("Skipped: 2");
    expect(screen.getByTestId("csv-line-errors")).toHaveTextContent("4: unknown asset");
  });

  it("refuses a non-CSV file before upload", async () => {
    const importFile = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(<CsvImport importFile={importFile} pollInterval={0} />);

    await user.upload(
      screen.getByTestId("csv-file"),
      new File(["%PDF"], "statement.pdf", { type: "application/pdf" }),
    );

    expect(screen.getByTestId("csv-errors")).toHaveTextContent(".csv");
    expect(importFile).not.toHaveBeenCalled();
  });

  it("shows the gateway's per-line errors on a 422", async () => {
    const importFile = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Validation failed",
        status: 422,
        correlationId: "corr-422",
        detail: "line 3: malformed quantity",
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(<CsvImport importFile={importFile} pollInterval={0} />);

    await user.upload(
      screen.getByTestId("csv-file"),
      new File(["bad"], "transactions.csv", { type: "text/csv" }),
    );
    await user.click(screen.getByTestId("csv-submit"));

    expect(await screen.findByTestId("csv-errors")).toHaveTextContent(
      "line 3: malformed quantity",
    );
  });
});

describe("account status", () => {
  it("shows the status label, shape and last-synced time", () => {
    renderWithProviders(<AccountStatusRow account={account} />);
    const row = screen.getByTestId("account-status");
    expect(row).toHaveTextContent("Binance main");
    expect(row).toHaveTextContent("Synced");
    expect(row).toHaveTextContent("2026");
  });

  it("keeps the last-synced time and shows the reason when a re-sync fails", async () => {
    const onResync = vi.fn().mockRejectedValue(new Error("provider timeout"));
    const user = userEvent.setup();
    renderWithProviders(
      <AccountStatusRow account={{ ...account, status: "failed" }} onResync={onResync} />,
    );

    await user.click(screen.getByTestId("account-resync"));

    expect(await screen.findByTestId("account-failure")).toHaveTextContent(
      "provider timeout",
    );
    expect(screen.getByTestId("account-status")).toHaveTextContent("2026");
  });
});
