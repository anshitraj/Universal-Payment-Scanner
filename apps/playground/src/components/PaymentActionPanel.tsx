import { useEffect, useMemo, useState } from "react";
import type { PaymentIntent } from "unipayscan";
import { WALLET_LOGOS } from "../data/logos";
import {
  UPI_APPS,
  attemptAppLaunch,
  genericUpiLink,
  isAndroid,
  upiAndroidIntentLink,
  venmoLinks,
} from "../payments/deepLinks";
import {
  genericSolanaLink,
  listSolanaWallets,
  payWithSolanaWallet,
  solanaExplorerUrl,
  type SolanaWalletOption,
} from "../payments/solanaWallets";
import { listPaymentAttempts, recordPaymentAttempt, type PaymentAttempt } from "../payments/transactionLog";

type ButtonState = { status: "idle" } | { status: "busy" } | { status: "warning"; text: string } | { status: "success"; text: string; link?: string } | { status: "error"; text: string };

function formatAmount(intent: PaymentIntent): string {
  if (!intent.amount) return "an amount you'll be asked to enter";
  const unit = intent.currency ?? intent.asset?.symbol ?? "";
  return `${intent.amount}${unit ? ` ${unit}` : ""}`;
}

function recipientLabel(intent: PaymentIntent): string {
  return intent.recipient?.name ?? intent.recipient?.id ?? intent.recipient?.address ?? "an unspecified recipient";
}

// A bare crypto address (no `bitcoin:`/`ethereum:` prefix - what most wallets' own "receive"
// screens show) isn't a launchable URI. Only offer the "open in wallet app" button when the
// recommended action's uri genuinely starts with a URI scheme (RFC 3986) - otherwise copying the
// address is the only honest option, since this project doesn't assume every network has an
// app-launch URI convention (several don't have one in wide use at all).
function hasUriScheme(uri: string | undefined): uri is string {
  return Boolean(uri && /^[a-z][a-z0-9+.-]*:/i.test(uri));
}

function copyableRecipient(intent: PaymentIntent): string | undefined {
  return intent.recipient?.address ?? intent.recipient?.id;
}

export function PaymentActionPanel({ intent }: { intent: PaymentIntent }) {
  const [open, setOpen] = useState(false);
  const [solanaWallets, setSolanaWallets] = useState<SolanaWalletOption[]>([]);
  const [buttonStates, setButtonStates] = useState<Record<string, ButtonState>>({});
  const [log, setLog] = useState<PaymentAttempt[]>([]);

  const action = intent.recommendedAction;
  const launchable = Boolean(
    intent.recognized && intent.supported && action && action.type !== "unsupported" && action.type !== "display_only",
  );

  useEffect(() => {
    setOpen(false);
    setButtonStates({});
  }, [intent]);

  useEffect(() => {
    if (open && action?.type === "wallet" && action.network === "solana") {
      setSolanaWallets(listSolanaWallets());
    }
  }, [open, action]);

  useEffect(() => {
    if (open) setLog(listPaymentAttempts());
  }, [open, buttonStates]);

  if (!launchable || !action) return null;

  const setState = (key: string, state: ButtonState) => setButtonStates((prev) => ({ ...prev, [key]: state }));

  const log_ = (entry: Omit<PaymentAttempt, "id" | "at">) => {
    recordPaymentAttempt(entry);
    setLog(listPaymentAttempts());
  };

  const launchDeepLink = (walletName: string, uri: string, webFallback?: string) => {
    setState(walletName, { status: "busy" });
    attemptAppLaunch(uri, () => {
      if (webFallback) {
        window.location.href = webFallback;
        setState(walletName, { status: "success", text: `Opened ${walletName} on the web instead.` });
        return;
      }
      setState(walletName, { status: "warning", text: `Doesn't look like ${walletName} is installed.` });
    });
    log_({ scheme: intent.scheme, wallet: walletName, amount: intent.amount, currency: intent.currency, recipient: recipientLabel(intent), outcome: "opened" });
    setTimeout(() => {
      setButtonStates((prev) => (prev[walletName]?.status === "busy" ? { ...prev, [walletName]: { status: "idle" } } : prev));
    }, 2000);
  };

  const paySolana = async (option: SolanaWalletOption) => {
    setState(option.name, { status: "busy" });
    try {
      const { signature } = await payWithSolanaWallet(option, intent);
      setState(option.name, { status: "success", text: "Signed and submitted.", link: solanaExplorerUrl(signature) });
      log_({ scheme: intent.scheme, wallet: option.name, amount: intent.amount, currency: intent.currency ?? intent.asset?.symbol, recipient: recipientLabel(intent), outcome: "signed", detail: signature });
    } catch (error) {
      const message = error instanceof Error ? error.message : "The wallet rejected or failed to sign this transaction.";
      setState(option.name, { status: "error", text: message });
      log_({ scheme: intent.scheme, wallet: option.name, amount: intent.amount, currency: intent.currency ?? intent.asset?.symbol, recipient: recipientLabel(intent), outcome: "failed", detail: message });
    }
  };

  const venmo = action.type === "redirect" && action.provider === "venmo" ? venmoLinks(intent) : null;
  const upiApps = action.type === "deeplink" && action.scheme === "upi" ? UPI_APPS : [];
  const genericLink = action.uri ?? undefined;
  const solanaLink = useMemo(() => {
    if (action.type !== "wallet" || action.network !== "solana") return undefined;
    try {
      return genericSolanaLink(intent);
    } catch {
      return undefined;
    }
  }, [action, intent]);

  return (
    <div className="pay-panel">
      {!open ? (
        <button type="button" className="pay-panel__trigger" onClick={() => setOpen(true)}>
          Make payment
        </button>
      ) : (
        <div className="pay-panel__confirm">
          <button type="button" className="pay-panel__close" onClick={() => setOpen(false)} aria-label="Close">
            ×
          </button>
          <p className="pay-panel__summary">
            Pay <strong>{formatAmount(intent)}</strong> to <strong>{recipientLabel(intent)}</strong>
          </p>
          <p className="pay-panel__warning">
            This opens a <strong>real</strong> wallet or payment app with these exact details - UniPayScan never
            touches your funds, your keys, or your PIN, and can't reverse anything once you approve it there. Only
            continue if you recognize this recipient and intend to pay them.
          </p>

          <div className="pay-panel__wallets">
            {action.type === "wallet" && action.network === "solana" && (
              <>
                {solanaWallets.length === 0 && (
                  <p className="pay-panel__hint">
                    No Solana wallet extension detected in this browser.{" "}
                    {solanaLink && (
                      <a href={solanaLink} onClick={() => log_({ scheme: intent.scheme, wallet: "Solana Pay (generic)", amount: intent.amount, currency: intent.asset?.symbol, recipient: recipientLabel(intent), outcome: "opened" })}>
                        Open in a Solana Pay-compatible app
                      </a>
                    )}
                  </p>
                )}
                {solanaWallets.map((w) => (
                  <WalletButton key={w.name} name={w.name} icon={w.icon} state={buttonStates[w.name]} onClick={() => void paySolana(w)} />
                ))}
              </>
            )}

            {upiApps.length > 0 && (
              <>
                {upiApps.map((app) => (
                  <WalletButton
                    key={app.id}
                    name={app.name}
                    logo={WALLET_LOGOS[app.id]}
                    disabledHint={isAndroid() ? undefined : "Needs Android to open this specific app"}
                    state={buttonStates[app.name]}
                    onClick={() =>
                      launchDeepLink(
                        app.name,
                        isAndroid() ? upiAndroidIntentLink(app, intent) : genericUpiLink(intent),
                      )
                    }
                  />
                ))}
                <WalletButton
                  name="Other UPI app"
                  state={buttonStates["Other UPI app"]}
                  onClick={() => launchDeepLink("Other UPI app", genericUpiLink(intent))}
                />
              </>
            )}

            {venmo && (
              <WalletButton
                name="Venmo"
                logo={WALLET_LOGOS.venmo}
                state={buttonStates.Venmo}
                onClick={() => launchDeepLink("Venmo", venmo.app, venmo.web)}
              />
            )}

            {action.type === "redirect" && action.provider !== "venmo" && genericLink && (
              <WalletButton
                name={action.provider ?? "Open link"}
                state={buttonStates[action.provider ?? "link"]}
                onClick={() => launchDeepLink(action.provider ?? "link", genericLink)}
              />
            )}

            {action.type === "wallet" && action.network !== "solana" && (
              <>
                {hasUriScheme(genericLink) && (
                  <WalletButton
                    name={`Open in ${action.network ?? "wallet"} app`}
                    state={buttonStates["generic-wallet"]}
                    onClick={() => launchDeepLink("generic-wallet", genericLink)}
                  />
                )}
                {copyableRecipient(intent) && (
                  <CopyButton value={copyableRecipient(intent)!} label="Copy address" />
                )}
              </>
            )}

            {action.type === "handoff" && !genericLink && (
              <p className="pay-panel__hint">
                {intent.scheme.replaceAll("_", " ")} has no single app-launch link - open your banking or payment
                app and enter these details yourself.{" "}
                {copyableRecipient(intent) && <CopyButton value={copyableRecipient(intent)!} label="Copy recipient" inline />}
              </p>
            )}
          </div>

          {log.length > 0 && (
            <details className="pay-panel__log">
              <summary>Recent activity ({log.length})</summary>
              <ul>
                {log.map((entry) => (
                  <li key={entry.id}>
                    <span>{new Date(entry.at).toLocaleTimeString()}</span> — {entry.wallet}: <strong>{entry.outcome.replace("_", " ")}</strong>
                    {entry.detail && entry.outcome === "signed" && (
                      <>
                        {" "}
                        (<a href={solanaExplorerUrl(entry.detail)} target="_blank" rel="noreferrer">view</a>)
                      </>
                    )}
                  </li>
                ))}
              </ul>
            </details>
          )}
        </div>
      )}
    </div>
  );
}

function CopyButton({ value, label, inline }: { value: string; label: string; inline?: boolean }) {
  const [copied, setCopied] = useState(false);
  const click = async () => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard permission denied - nothing more we can do here.
    }
  };
  if (inline) {
    return (
      <button type="button" className="pay-panel__inline-copy" onClick={() => void click()}>
        {copied ? "Copied" : label}
      </button>
    );
  }
  return (
    <div className="wallet-button-wrap">
      <button type="button" className="wallet-button" onClick={() => void click()}>
        <span className="wallet-button__fallback">⧉</span>
        <span>{copied ? "Copied" : label}</span>
      </button>
    </div>
  );
}

function WalletButton({
  name,
  logo,
  icon,
  state,
  disabledHint,
  onClick,
}: {
  name: string;
  logo?: string | undefined;
  icon?: string | undefined;
  state?: ButtonState | undefined;
  disabledHint?: string | undefined;
  onClick: () => void;
}) {
  const status = state?.status ?? "idle";
  return (
    <div className="wallet-button-wrap">
      <button type="button" className={`wallet-button wallet-button--${status}`} onClick={onClick} disabled={status === "busy"} title={disabledHint}>
        {icon ? <img src={icon} alt="" /> : logo ? <img src={logo} alt="" /> : <span className="wallet-button__fallback">{name[0]}</span>}
        <span>{name}</span>
        {status === "busy" && <em>Opening…</em>}
      </button>
      {(status === "warning" || status === "error") && (state as { text: string }).text && (
        <p className="wallet-button__note wallet-button__note--warn">{(state as { text: string }).text}</p>
      )}
      {status === "success" && (
        <p className="wallet-button__note wallet-button__note--ok">
          {(state as { text: string; link?: string }).text}{" "}
          {(state as { link?: string }).link && (
            <a href={(state as { link?: string }).link} target="_blank" rel="noreferrer">
              View transaction
            </a>
          )}
        </p>
      )}
    </div>
  );
}
