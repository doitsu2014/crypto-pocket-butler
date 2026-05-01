/**
 * Example usage of the useChains hook
 * 
 * This file demonstrates how to use the useChains hook to fetch and display
 * the list of supported EVM chains.
 */

import { useChains } from "@/hooks";

export function ChainsExample() {
  const { data: chains, isLoading, error } = useChains();

  if (isLoading) {
    return <div>Loading supported chains...</div>;
  }

  if (error) {
    return <div>Error loading chains: {error.message}</div>;
  }

  return (
    <div>
      <h2>Supported EVM Chains</h2>
      <ul>
        {chains?.map((chain) => (
          <li key={chain.id}>
            <strong>{chain.name}</strong> ({chain.id}) - Native: {chain.native_symbol}
          </li>
        ))}
      </ul>
    </div>
  );
}

/**
 * Example: Using chains in a wallet account form
 */
export function WalletAccountForm() {
  const { data: chains, isLoading, error } = useChains();

  return (
    <form>
      <div>
        <label>Wallet Address:</label>
        <input type="text" name="wallet_address" />
      </div>
      
      <div>
        <label>Select Chains:</label>
        {isLoading && <p>Loading chains...</p>}
        {error && <p>Error loading chains: {error.message}</p>}
        {chains?.map((chain) => (
          <div key={chain.id}>
            <input 
              type="checkbox" 
              id={`chain-${chain.id}`} 
              value={chain.id} 
              name="enabled_chains"
            />
            <label htmlFor={`chain-${chain.id}`}>
              {chain.name} ({chain.native_symbol})
            </label>
          </div>
        ))}
      </div>
    </form>
  );
}
