import { useState } from "react";
import type { PaymentIntent } from "@universal-payment-qr/core";
import { PaymentQRScanner } from "@universal-payment-qr/react";

type SchemeEntry = readonly [id: string, name: string, country: string];
type SchemeGroup = readonly [label: string, entries: readonly SchemeEntry[]];

const schemeGroups: readonly SchemeGroup[] = [
  ["Global", [
    ["emvco_mpm", "EMVCo", "GLOBAL"],
    ["paypal", "PayPal", "GLOBAL"],
    ["bitcoin", "Bitcoin", "GLOBAL"],
    ["ethereum", "Ethereum", "GLOBAL"],
    ["solana_pay", "Solana Pay", "GLOBAL"],
    ["tron", "TRON", "GLOBAL"],
    ["ton", "TON", "GLOBAL"],
    ["xrp", "XRP Ledger", "GLOBAL"],
    ["stellar", "Stellar", "GLOBAL"],
  ]],
  ["South Asia", [
    ["upi", "UPI", "IN"],
    ["nepalpay_qr", "NepalPay QR", "NP"],
    ["lankaqr", "LankaQR", "LK"],
    ["bangla_qr", "Bangla QR", "BD"],
    ["raast_qr", "Raast QR", "PK"],
  ]],
  ["Southeast Asia", [
    ["promptpay", "PromptPay", "TH"],
    ["vietqr", "VietQR", "VN"],
    ["paynow", "PayNow", "SG"],
    ["duitnow_qr", "DuitNow QR", "MY"],
    ["qris", "QRIS", "ID"],
    ["qr_ph", "QR Ph", "PH"],
    ["khqr", "KHQR", "KH"],
    ["lao_qr", "Lao QR", "LA"],
    ["mmqr", "MMQR", "MM"],
  ]],
  ["East Asia", [
    ["hkqr", "HKQR", "HK"],
    ["jpqr", "JPQR", "JP"],
    ["twqr", "TWQR", "TW"],
    ["zeropay", "ZeroPay", "KR"],
    ["alipay", "Alipay", "CN"],
    ["wechat_pay", "WeChat Pay", "CN"],
  ]],
  ["North America", [
    ["venmo", "Venmo", "US"],
    ["cash_app", "Cash App", "US"],
  ]],
  ["Latin America", [
    ["pix", "Pix", "BR"],
    ["mercado_pago", "Mercado Pago", "AR"],
  ]],
  ["Europe", [
    ["epc_qr", "SEPA / EPC (Girocode)", "EU"],
    ["swiss_qr_bill", "Swiss QR-bill", "CH"],
  ]],
  ["Other QR types", [
    ["walletconnect", "WalletConnect", "GLOBAL"],
    ["otp_setup", "Authenticator setup", "GLOBAL"],
    ["url", "Generic URL", "GLOBAL"],
  ]],
];

const schemes: readonly SchemeEntry[] = schemeGroups.flatMap(([, list]) => list);

export function App() {
  const [enabled, setEnabled] = useState<string[]>(schemes.map(([id]) => id));
  const [intent, setIntent] = useState<PaymentIntent>();
  const [copied, setCopied] = useState(false);

  const receive = (next: PaymentIntent) => { setIntent(next); setCopied(false); };
  const copy = async () => {
    if (!intent) return;
    await navigator.clipboard.writeText(JSON.stringify(intent, null, 2));
    setCopied(true);
  };

  return (
    <main>
      <header className="masthead">
        <a className="wordmark" href="#top" aria-label="Universal Payment QR home"><span>U</span> PAYMENT QR</a>
        <p>Open infrastructure · v0.1 preview</p>
        <a href="https://github.com/" target="_blank" rel="noreferrer">Source ↗</a>
      </header>

      <section className="hero" id="top">
        <div className="hero__index">01 / PARSE</div>
        <h1>One scanner for<br /><em>every payment QR.</em></h1>
        <p>Recognize the standard. Validate the structure. Normalize the intent. Your application decides what happens next.</p>
        <div className="hero__rule"><span>NO CUSTODY</span><span>NO AUTO-NAVIGATION</span><span>LOCAL BY DEFAULT</span></div>
      </section>

      <section className="workspace" aria-label="Scanner playground">
        <div className="workspace__scanner">
          <PaymentQRScanner enabledSchemes={enabled} onDetected={receive} onUnsupported={receive} onError={(_error, next) => next && receive(next)} />
        </div>
        <aside className="control-panel">
          <div className="panel-heading"><span>02</span><div><small>APPLICATION POLICY</small><h2>Acceptance surface</h2></div></div>
          <p className="panel-copy">Detection stays global. These switches control only what this application accepts.</p>
          <div className="scheme-list">
            {schemeGroups.map(([group, list]) => (
              <div className="scheme-group" key={group}>
                <p className="scheme-group__label">{group}</p>
                {list.map(([id, name, country]) => {
                  const checked = enabled.includes(id);
                  return <label key={id}><span><strong>{name}</strong><small>{country}</small></span><input type="checkbox" checked={checked} onChange={() => setEnabled((current) => checked ? current.filter((value) => value !== id) : [...current, id])} /><i aria-hidden="true" /></label>;
                })}
              </div>
            ))}
          </div>
        </aside>
      </section>

      <section className="output" aria-label="Normalized output">
        <div className="output__intro"><span>03 / NORMALIZE</span><h2>A boring contract.<br /><em>By design.</em></h2><p>The same versioned shape across banks, wallets, links, and chains. Amounts remain exact strings.</p></div>
        <div className="json-panel">
          <header><span><i /> normalized.intent.json</span><button type="button" onClick={() => void copy()} disabled={!intent}>{copied ? "Copied" : "Copy JSON"}</button></header>
          <pre>{intent ? JSON.stringify(intent, null, 2) : "// Scan or paste a payment QR to inspect its normalized intent."}</pre>
        </div>
      </section>

      <footer><p>Parsing proves structure—not identity, safety, or payment completion.</p><span>MIT LICENSE</span></footer>
    </main>
  );
}
