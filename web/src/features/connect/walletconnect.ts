/**
 * WalletConnect C14 read-only session.
 *
 * The provider is imported lazily on the connect route; the session only reads
 * accounts. No signing or transfer action is exposed (BR1.2, C14).
 */

export interface WalletConnectSession {
  address: string;
  topic: string;
  chain: string;
}

export async function connectWalletConnect(): Promise<WalletConnectSession> {
  const projectId = import.meta.env.VITE_WALLETCONNECT_PROJECT_ID as
    | string
    | undefined;
  if (!projectId) {
    throw new Error("WalletConnect project id is not configured");
  }
  const { EthereumProvider } = await import("@walletconnect/ethereum-provider");
  const provider = await EthereumProvider.init({
    projectId,
    chains: [1],
    optionalChains: [1, 56, 137],
    showQrModal: true,
  });
  await provider.connect();
  const address = provider.accounts[0] ?? "";
  const topic = provider.session?.topic ?? "";
  return { address, topic, chain: "eip155:1" };
}
