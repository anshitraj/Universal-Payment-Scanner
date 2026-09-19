// Ethereum (and EVM sibling chains) wallet handoff via EIP-6963 (https://eips.ethereum.org/EIPS/eip-6963)
// - the standard, accepted-since-2023 way a page discovers every installed EIP-1193 wallet
// extension (MetaMask, Coinbase Wallet, Rainbow, ...) without guessing which one last claimed
// `window.ethereum`. Same shape as Solana's Wallet Standard integration (solanaWallets.ts): no
// hardcoded per-wallet list, each wallet self-reports its own name and icon, and UniPayScan never
// sees a private key - `eth_sendTransaction` opens the wallet's own approval UI.

import type { PaymentIntent } from "unipayscan";

interface Eip1193Provider {
  request(args: { method: string; params?: unknown[] | Record<string, unknown> }): Promise<unknown>;
}

interface Eip6963ProviderInfo {
  uuid: string;
  name: string;
  icon: string;
  rdns: string;
}

interface Eip6963ProviderDetail {
  info: Eip6963ProviderInfo;
  provider: Eip1193Provider;
}

export interface EthereumWalletOption {
  name: string;
  icon: string;
  provider: Eip1193Provider;
}

/** Every EIP-6963-compliant wallet currently installed. Empty on mobile web and in any browser
 * with no wallet extension - callers should fall back to an EIP-681 `ethereum:` deep link there
 * (see genericEthereumLink below). Per spec, the app "MUST NOT remove" its listener for the
 * lifetime of the page, but for this one-shot use (list wallets right when the confirm panel
 * opens) a short collection window after the request is enough in practice - wallets announce
 * themselves synchronously in response to the request event. */
export function listEthereumWallets(): Promise<EthereumWalletOption[]> {
  if (typeof window === "undefined") return Promise.resolve([]);
  return new Promise((resolve) => {
    const found = new Map<string, Eip6963ProviderDetail>();
    const onAnnounce = (event: Event) => {
      const detail = (event as CustomEvent<Eip6963ProviderDetail>).detail;
      found.set(detail.info.uuid, detail);
    };
    window.addEventListener("eip6963:announceProvider", onAnnounce);
    window.dispatchEvent(new Event("eip6963:requestProvider"));
    window.setTimeout(() => {
      window.removeEventListener("eip6963:announceProvider", onAnnounce);
      resolve([...found.values()].map((d) => ({ name: d.info.name, icon: d.info.icon, provider: d.provider })));
    }, 150);
  });
}

// network name (from PaymentIntent.network, set by crates/core/src/schemes/ethereum.rs's
// chain_name()) -> [hex chain id (EIP-155), block explorer base URL]. "evm" (unrecognized chain
// id) has no safe entry here on purpose - switching to the wrong chain and sending funds there is
// exactly the kind of mistake this table exists to prevent, so unrecognized chains fall back to
// "open your wallet app directly" instead of a guess.
const EVM_CHAINS: Record<string, { hexId: string; explorer: string }> = {
  ethereum: { hexId: "0x1", explorer: "https://etherscan.io/tx/" },
  optimism: { hexId: "0xa", explorer: "https://optimistic.etherscan.io/tx/" },
  polygon: { hexId: "0x89", explorer: "https://polygonscan.com/tx/" },
  arbitrum: { hexId: "0xa4b1", explorer: "https://arbiscan.io/tx/" },
  base: { hexId: "0x2105", explorer: "https://basescan.org/tx/" },
};

export function isKnownEvmChain(network: string | undefined): boolean {
  return Boolean(network && network in EVM_CHAINS);
}

export class EthereumPaymentError extends Error {}

function toWeiHex(decimalEth: string): string {
  const [whole, frac = ""] = decimalEth.split(".");
  const fracPadded = (frac + "0".repeat(18)).slice(0, 18);
  const wei = BigInt(whole || "0") * 10n ** 18n + BigInt(fracPadded || "0");
  return `0x${wei.toString(16)}`;
}

/** Requests accounts, switches the wallet to the chain the QR actually specifies (never assumes
 * the wallet is already on the right one - a silent wrong-chain send is exactly the costly mistake
 * this checks for), and sends a native-asset transfer. Returns the transaction hash the wallet
 * submitted; this does not wait for confirmation. */
export async function payWithEthereumWallet(
  option: EthereumWalletOption,
  intent: PaymentIntent,
): Promise<{ txHash: string; explorerUrl: string }> {
  if (intent.asset?.contract) {
    throw new EthereumPaymentError(
      "This demo only sends the chain's native asset, not ERC-20 tokens - open this QR in your wallet app directly for a token transfer.",
    );
  }
  const network = intent.network;
  const chain = network ? EVM_CHAINS[network] : undefined;
  if (!chain) {
    throw new EthereumPaymentError(
      `${network ?? "This chain"} isn't one this demo can confirm a safe chain id for - open this QR in your wallet app directly rather than risk sending on the wrong network.`,
    );
  }
  const to = intent.recipient?.address;
  if (!to) throw new EthereumPaymentError("This payment intent has no recipient address.");
  if (!intent.amount) throw new EthereumPaymentError("This QR doesn't specify an amount to send.");

  const accounts = (await option.provider.request({ method: "eth_requestAccounts" })) as string[];
  const from = accounts[0];
  if (!from) throw new EthereumPaymentError(`${option.name} did not authorize any account.`);

  try {
    await option.provider.request({
      method: "wallet_switchEthereumChain",
      params: [{ chainId: chain.hexId }],
    });
  } catch {
    throw new EthereumPaymentError(
      `${option.name} couldn't switch to the ${network} network - add it in your wallet and try again.`,
    );
  }

  const txHash = (await option.provider.request({
    method: "eth_sendTransaction",
    params: [{ from, to, value: toWeiHex(intent.amount) }],
  })) as string;

  return { txHash, explorerUrl: `${chain.explorer}${txHash}` };
}

/** EIP-681 (https://eips.ethereum.org/EIPS/eip-681) - the same URI format
 * crates/core/src/schemes/ethereum.rs already parses, used here as the mobile fallback when no
 * EIP-6963 wallet is installed in this browser. */
export function genericEthereumLink(intent: PaymentIntent): string {
  const existing = intent.recommendedAction?.uri;
  if (existing?.startsWith("ethereum:")) return existing;
  const address = intent.recipient?.address;
  if (!address) throw new EthereumPaymentError("No recipient address to build an ethereum: link from.");
  const chain = intent.network ? EVM_CHAINS[intent.network] : undefined;
  const chainIdDecimal = chain ? Number.parseInt(chain.hexId, 16) : undefined;
  const suffix = chainIdDecimal ? `@${chainIdDecimal}` : "";
  const params = intent.amount ? `?value=${BigInt(toWeiHex(intent.amount))}` : "";
  return `ethereum:${address}${suffix}${params}`;
}
