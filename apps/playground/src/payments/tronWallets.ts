// TRON wallet handoff via TIP-6963 (https://github.com/tronprotocol/tips/blob/master/tip-6963.md)
// - TRON's own adoption of the EIP-6963 pattern (same event shape, "TIP6963" instead of "eip6963"
// in the event names - verified against the TIP's own source, not assumed from the Ethereum
// version). Same reasoning as the Solana/Ethereum integrations: no hardcoded per-wallet list, each
// wallet self-announces its own name and icon, and UniPayScan never sees a private key -
// tronWeb.trx.sign() opens the wallet's own approval UI.
//
// No extra dependency needed: the injected provider carries a full, ready-to-use `tronWeb`
// instance once authorized (confirmed against TronLink's own integration docs), so this doesn't
// need the standalone `tronweb` package the way the Ethereum integration needs `@solana/web3.js`.

import type { PaymentIntent } from "unipayscan";

interface Tip1193Provider {
  request(args: { method: string; params?: unknown }): Promise<unknown>;
  /** Populated (non-false) only after the origin is authorized via eth_requestAccounts. */
  tronWeb:
    | false
    | {
        transactionBuilder: {
          sendTrx(to: string, amountSun: number, from: string): Promise<unknown>;
        };
        trx: {
          sign(tx: unknown): Promise<unknown>;
          sendRawTransaction(signedTx: unknown): Promise<{ txid?: string; transaction?: { txID?: string } }>;
        };
      };
}

interface Tip6963ProviderDetail {
  info: { uuid: string; name: string; icon: string; rdns: string };
  provider: Tip1193Provider;
}

export interface TronWalletOption {
  name: string;
  icon: string;
  provider: Tip1193Provider;
}

/** Every currently-installed TIP-6963 wallet. Empty on mobile web and in any browser with no TRON
 * wallet extension - callers should fall back to a bare-address "copy" flow there, the same as
 * every other scheme with no confirmed app-launch URI convention (TRON has none). */
export function listTronWallets(): Promise<TronWalletOption[]> {
  if (typeof window === "undefined") return Promise.resolve([]);
  return new Promise((resolve) => {
    const found = new Map<string, Tip6963ProviderDetail>();
    const onAnnounce = (event: Event) => {
      const detail = (event as CustomEvent<Tip6963ProviderDetail>).detail;
      found.set(detail.info.uuid, detail);
    };
    window.addEventListener("TIP6963:announceProvider", onAnnounce);
    window.dispatchEvent(new Event("TIP6963:requestProvider"));
    window.setTimeout(() => {
      window.removeEventListener("TIP6963:announceProvider", onAnnounce);
      resolve([...found.values()].map((d) => ({ name: d.info.name, icon: d.info.icon, provider: d.provider })));
    }, 150);
  });
}

export class TronPaymentError extends Error {}

/** Authorizes the origin, then builds -> signs -> broadcasts a native TRX transfer using the
 * wallet's own attached tronWeb instance (TronLink's documented build -> sign -> broadcast
 * sequence: transactionBuilder.sendTrx -> trx.sign -> trx.sendRawTransaction). Amount is
 * converted using the core's own asset.decimals (6 for TRX - TRON's "sun" unit), not a hardcoded
 * assumption. */
export async function payWithTronWallet(
  option: TronWalletOption,
  intent: PaymentIntent,
): Promise<{ txId: string }> {
  if (intent.asset?.contract) {
    throw new TronPaymentError(
      "This demo only sends native TRX, not TRC-20 tokens - open this QR in your wallet app directly for a token transfer.",
    );
  }
  const to = intent.recipient?.address ?? intent.recipient?.id;
  if (!to) throw new TronPaymentError("This payment intent has no recipient address.");
  if (!intent.amount) throw new TronPaymentError("This QR doesn't specify an amount to send.");

  const accounts = (await option.provider.request({ method: "eth_requestAccounts" })) as string[];
  const from = accounts[0];
  if (!from) throw new TronPaymentError(`${option.name} did not authorize any account.`);

  const tronWeb = option.provider.tronWeb;
  if (!tronWeb) throw new TronPaymentError(`${option.name} authorized but didn't expose tronWeb.`);

  const decimals = intent.asset?.decimals ?? 6;
  const amountSun = Math.round(Number(intent.amount) * 10 ** decimals);

  const unsignedTx = await tronWeb.transactionBuilder.sendTrx(to, amountSun, from);
  const signedTx = await tronWeb.trx.sign(unsignedTx);
  const result = await tronWeb.trx.sendRawTransaction(signedTx);
  const txId = result.txid ?? result.transaction?.txID;
  if (!txId) throw new TronPaymentError("The wallet didn't return a transaction id after broadcasting.");

  return { txId };
}

export function tronExplorerUrl(txId: string): string {
  return `https://tronscan.org/#/transaction/${txId}`;
}
