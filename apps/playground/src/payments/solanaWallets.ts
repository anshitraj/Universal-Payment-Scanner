// Solana wallet handoff via the Wallet Standard (https://github.com/anza-xyz/wallet-standard) -
// the same mechanism Phantom, Solflare, Backpack, and every other modern Solana wallet extension
// use to announce themselves to a page. No hardcoded per-wallet list: whatever wallets the user
// has installed register themselves, each with its own name and icon, and this just enumerates
// them. UniPayScan never sees a private key - `signAndSendTransaction` opens the wallet's own
// approval UI, the wallet signs, and the wallet submits it.
//
// The `solana:signAndSendTransaction` feature isn't part of @wallet-standard/core (that package
// only carries the chain-agnostic `standard:*` features) - it's Solana's own extension to the
// standard. The official types live in `@solana/wallet-standard-features`, which this project
// doesn't otherwise depend on, so the minimal shape actually used here is declared locally against
// the published spec rather than pulling in a whole extra package for two type aliases.

import { getWallets } from "@wallet-standard/core";
import type { Wallet, WalletAccount } from "@wallet-standard/base";
import type { StandardConnectFeature } from "@wallet-standard/features";
import { Connection, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import bs58 from "bs58";
import type { PaymentIntent } from "unipayscan";

const SOLANA_CHAIN = "solana:mainnet";
const SOLANA_RPC = "https://api.mainnet-beta.solana.com";
const SIGN_AND_SEND_FEATURE = "solana:signAndSendTransaction";

interface SolanaSignAndSendTransactionOutput {
  readonly signature: Uint8Array;
}

interface SolanaSignAndSendTransactionFeature {
  readonly [SIGN_AND_SEND_FEATURE]: {
    readonly signAndSendTransaction: (
      ...inputs: readonly {
        account: WalletAccount;
        chain: string;
        transaction: Uint8Array;
      }[]
    ) => Promise<readonly SolanaSignAndSendTransactionOutput[]>;
  };
}

type SolanaCapableWallet = Wallet & {
  features: StandardConnectFeature & SolanaSignAndSendTransactionFeature;
};

export interface SolanaWalletOption {
  wallet: SolanaCapableWallet;
  name: string;
  icon?: string;
}

function isSolanaCapable(wallet: Wallet): wallet is SolanaCapableWallet {
  return (
    wallet.chains.some((chain) => chain.startsWith("solana:")) &&
    "standard:connect" in wallet.features &&
    SIGN_AND_SEND_FEATURE in wallet.features
  );
}

/** Every currently-installed wallet that speaks the Solana Wallet Standard. Empty on mobile web
 * and in any browser with no wallet extension installed - callers should fall back to the Solana
 * Pay `solana:` deep link in that case (see deepLinks.ts). */
export function listSolanaWallets(): SolanaWalletOption[] {
  if (typeof window === "undefined") return [];
  const wallets = getWallets().get().filter(isSolanaCapable);
  return wallets.map((wallet) => ({ wallet, name: wallet.name, icon: wallet.icon }));
}

export class SolanaPaymentError extends Error {}

/** Connects (prompting the wallet's own approval UI), builds a native SOL transfer from the
 * intent's recipient/amount, and asks the wallet to sign and submit it. Returns the base58
 * transaction signature - the same format Solana Explorer / Solscan use in their URLs - once the
 * wallet has broadcast it. This does not wait for on-chain confirmation; it only confirms the
 * wallet accepted and submitted the transaction. */
export async function payWithSolanaWallet(
  option: SolanaWalletOption,
  intent: PaymentIntent,
): Promise<{ signature: string }> {
  if (intent.asset?.contract) {
    throw new SolanaPaymentError(
      "This demo only sends native SOL, not SPL tokens - open this QR in your wallet app directly for a token transfer.",
    );
  }
  const recipientAddress = intent.recipient?.address ?? intent.recipient?.id;
  if (!recipientAddress) {
    throw new SolanaPaymentError("This payment intent has no recipient address.");
  }
  const amount = intent.amount ? Number(intent.amount) : undefined;
  if (!amount || Number.isNaN(amount) || amount <= 0) {
    throw new SolanaPaymentError("This QR doesn't specify an amount to send.");
  }

  let toPubkey: PublicKey;
  try {
    toPubkey = new PublicKey(recipientAddress);
  } catch {
    throw new SolanaPaymentError("Recipient address is not a valid Solana address.");
  }

  const connectFeature = option.wallet.features["standard:connect"];
  const { accounts } = await connectFeature.connect();
  const account = accounts[0];
  if (!account) throw new SolanaPaymentError(`${option.name} did not authorize any account.`);

  const fromPubkey = new PublicKey(account.address);
  const connection = new Connection(SOLANA_RPC, "confirmed");
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();

  const transaction = new Transaction({
    feePayer: fromPubkey,
    blockhash,
    lastValidBlockHeight,
  }).add(SystemProgram.transfer({ fromPubkey, toPubkey, lamports: Math.round(amount * 1_000_000_000) }));

  const signAndSend = option.wallet.features[SIGN_AND_SEND_FEATURE].signAndSendTransaction;
  const [result] = await signAndSend({
    account,
    chain: SOLANA_CHAIN,
    transaction: transaction.serialize({ requireAllSignatures: false, verifySignatures: false }),
  });
  if (!result) throw new SolanaPaymentError(`${option.name} did not return a signature.`);

  return { signature: bs58.encode(result.signature) };
}

export function solanaExplorerUrl(signature: string): string {
  return `https://explorer.solana.com/tx/${signature}`;
}

/** Solana Pay's own `solana:` URI (https://docs.solanapay.com/spec) - the same scheme mobile
 * wallets register as a handler for. `recommendedAction.uri` is only already one of these when the
 * scanned payload itself was a Solana Pay URI; a bare address (what most wallets' own "receive"
 * screens show) has no scheme prefix at all, so it isn't safe to use directly as a link. */
export function genericSolanaLink(intent: PaymentIntent): string {
  const existing = intent.recommendedAction?.uri;
  if (existing?.startsWith("solana:")) return existing;
  const recipient = intent.recipient?.address ?? intent.recipient?.id ?? existing;
  if (!recipient) throw new SolanaPaymentError("No recipient address to build a Solana Pay link from.");
  const params = new URLSearchParams();
  if (intent.amount) params.set("amount", intent.amount);
  if (intent.asset?.contract) params.set("spl-token", intent.asset.contract);
  if (intent.reference) params.set("reference", intent.reference);
  if (intent.description) params.set("message", intent.description);
  const query = params.toString();
  return `solana:${recipient}${query ? `?${query}` : ""}`;
}
