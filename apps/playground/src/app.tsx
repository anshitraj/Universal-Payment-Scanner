import { useEffect, useMemo, useState } from "react";
import { createScanner, type PaymentIntent, type SchemeMetadata } from "unipayscan";
import { PaymentQRScanner } from "@universal-payment-qr/react";
import { REGION_ORDER, NON_PAYMENT_SCHEME_IDS, regionFor } from "./data/regions";
import { SCANNER_EXAMPLES } from "./data/examples";
import { SDK_STATUSES } from "./data/sdks";

const GITHUB_URL = "https://github.com/anshitraj/Universal-Payment-Scanner";
const DOCS_URL = `${GITHUB_URL}/blob/main/docs/schema.md`;
const CONTRIBUTING_URL = `${GITHUB_URL}/blob/main/CONTRIBUTING.md`;
const NON_PAYMENT_IDS = new Set<string>(NON_PAYMENT_SCHEME_IDS);
const MATURITIES = ["stable", "beta", "community", "experimental"] as const;

function MaturityBadge({ maturity }: { maturity: string }) {
  return <span className={`maturity maturity--${maturity}`}>{maturity}</span>;
}

function groupByRegion(schemes: readonly SchemeMetadata[]) {
  const groups = new Map<string, SchemeMetadata[]>();
  for (const scheme of schemes) {
    const region = regionFor(scheme.countries);
    groups.set(region, [...(groups.get(region) ?? []), scheme]);
  }
  return REGION_ORDER.filter((region) => groups.has(region)).map((region) => [region, groups.get(region)!] as const);
}

export function App() {
  const [capabilities, setCapabilities] = useState<SchemeMetadata[]>([]);
  const [enabled, setEnabled] = useState<string[]>([]);
  const [intent, setIntent] = useState<PaymentIntent>();
  const [copied, setCopied] = useState(false);
  const [configCopied, setConfigCopied] = useState(false);
  const [example, setExample] = useState<{ key: string | number; payload: string }>();
  const [schemeQuery, setSchemeQuery] = useState("");
  const [maturityFilter, setMaturityFilter] = useState<"all" | (typeof MATURITIES)[number]>("all");

  useEffect(() => {
    let cancelled = false;
    void createScanner().getCapabilities().then((caps) => {
      if (cancelled) return;
      setCapabilities(caps);
      setEnabled(caps.filter((s) => !NON_PAYMENT_IDS.has(s.id)).map((s) => s.id));
    });
    return () => { cancelled = true; };
  }, []);

  const scanner = useMemo(() => createScanner({
    schemes: Object.fromEntries(capabilities.map((s) => [s.id, NON_PAYMENT_IDS.has(s.id) ? true : enabled.includes(s.id)])),
  }), [capabilities, enabled.join("|")]);

  const paymentSchemes = useMemo(() => capabilities.filter((s) => !NON_PAYMENT_IDS.has(s.id)), [capabilities]);
  const nonPaymentSchemes = useMemo(() => capabilities.filter((s) => NON_PAYMENT_IDS.has(s.id)), [capabilities]);
  const groupedPayment = useMemo(() => groupByRegion(paymentSchemes), [paymentSchemes]);

  const filteredDirectory = useMemo(() => {
    const query = schemeQuery.trim().toLowerCase();
    return paymentSchemes.filter((s) => {
      if (maturityFilter !== "all" && s.maturity !== maturityFilter) return false;
      if (!query) return true;
      return [s.displayName, s.id, s.standard, ...s.countries].join(" ").toLowerCase().includes(query);
    });
  }, [paymentSchemes, schemeQuery, maturityFilter]);
  const groupedDirectory = useMemo(() => groupByRegion(filteredDirectory), [filteredDirectory]);

  const turnedOff = paymentSchemes.filter((s) => !enabled.includes(s.id)).map((s) => s.id);
  const configCode = turnedOff.length === 0
    ? "const scanner = createScanner();\n// all schemes enabled by default"
    : `const scanner = createScanner({\n  schemes: {\n${turnedOff.map((id) => `    ${id}: false,`).join("\n")}\n  },\n});`;

  const receive = (next: PaymentIntent) => { setIntent(next); setCopied(false); };
  const copy = async () => {
    if (!intent) return;
    await navigator.clipboard.writeText(JSON.stringify(intent, null, 2));
    setCopied(true);
  };
  const copyConfig = async () => {
    await navigator.clipboard.writeText(configCode);
    setConfigCopied(true);
    setTimeout(() => setConfigCopied(false), 1500);
  };

  const intentMeta = intent ? capabilities.find((s) => s.id === intent.scheme) : undefined;

  return (
    <main>
      <header className="masthead">
        <a className="wordmark" href="#top" aria-label="UniPayScan home"><span>U</span>UniPayScan</a>
        <nav className="masthead__links" aria-label="Primary">
          <a href="#playground">Playground</a>
          <a href="#schemes">Schemes</a>
          <a href="#sdks">SDKs</a>
          <a href={DOCS_URL} target="_blank" rel="noreferrer">Docs</a>
        </nav>
        <a href={GITHUB_URL} target="_blank" rel="noreferrer">GitHub ↗</a>
      </header>

      <section className="hero" id="top">
        <div className="hero__index">01 / PARSE</div>
        <h1>One scanner for<br /><em>every payment QR.</em></h1>
        <p>Open-source infrastructure for detecting, validating, and normalizing payment QRs, wallet addresses, and payment links across fiat and crypto. Recognize the standard. Validate the structure. Normalize the intent. Your application decides what happens next.</p>
        <div className="hero__cta">
          <a href="#playground" className="btn btn--primary">Try Playground</a>
          <a href={GITHUB_URL} target="_blank" rel="noreferrer" className="btn btn--ghost">View on GitHub ↗</a>
        </div>
        <p className="hero__install">Packages publishing soon · Use from GitHub today</p>
        <div className="hero__rule"><span>NO CUSTODY</span><span>NO AUTO-NAVIGATION</span><span>LOCAL BY DEFAULT</span></div>
      </section>

      <section className="problem" aria-label="What UniPayScan does">
        <div className="problem__intro"><span className="section-index">01a</span><h2>One input.<br /><em>Different payment systems.</em></h2></div>
        <div className="problem__diagram">
          <div className="problem__node problem__node--in">SCAN QR</div>
          <div className="problem__arrow" aria-hidden="true">↓</div>
          <div className="problem__node problem__node--core">UniPayScan</div>
          <div className="problem__arrow" aria-hidden="true">↓</div>
          <div className="problem__outputs">
            <div><strong>UPI</strong><span>→ payment intent</span></div>
            <div><strong>Pix</strong><span>→ payment intent</span></div>
            <div><strong>PayPal</strong><span>→ payment link</span></div>
            <div><strong>Bitcoin</strong><span>→ payment intent</span></div>
            <div><strong>Solana Pay</strong><span>→ payment intent</span></div>
            <div className="problem__outputs--muted"><strong>WalletConnect</strong><span>→ not a payment</span></div>
          </div>
        </div>
        <p className="problem__outro">Your application gets one normalized interface.</p>
      </section>

      <section className="workspace" aria-label="Scanner playground" id="playground">
        <div className="workspace__scanner">
          <div className="try-examples">
            <p>Try an example:</p>
            <div className="try-examples__buttons">
              {SCANNER_EXAMPLES.map((ex) => (
                <button key={ex.key} type="button" onClick={() => setExample({ key: `${ex.key}-${Date.now()}`, payload: ex.payload })}>{ex.label}</button>
              ))}
            </div>
          </div>
          <PaymentQRScanner scanner={scanner} example={example} onDetected={receive} onUnsupported={receive} onError={(_error, next) => next && receive(next)} />
        </div>
        <aside className="control-panel">
          <div className="panel-heading"><span>02</span><div><small>APPLICATION POLICY</small><h2>Payment methods your app accepts</h2></div></div>
          <p className="panel-copy">Detection stays global. These switches control only what this application accepts.</p>
          <div className="scheme-list">
            {groupedPayment.map(([region, list]) => (
              <div className="scheme-group" key={region}>
                <p className="scheme-group__label">{region}</p>
                {list.map((s) => {
                  const checked = enabled.includes(s.id);
                  return (
                    <label key={s.id}>
                      <span><strong>{s.displayName}</strong><MaturityBadge maturity={s.maturity} /></span>
                      <input type="checkbox" checked={checked} onChange={() => setEnabled((current) => checked ? current.filter((value) => value !== s.id) : [...current, s.id])} />
                      <i aria-hidden="true" />
                    </label>
                  );
                })}
              </div>
            ))}
          </div>

          <div className="panel-heading panel-heading--secondary"><div><small>ALSO RECOGNIZED</small><h2>Other QR types UniPayScan recognizes</h2></div></div>
          <p className="panel-copy">Structurally valid, always identified by name — never toggled, because these were never payment methods to begin with.</p>
          <div className="scheme-list scheme-list--static">
            {nonPaymentSchemes.map((s) => (
              <div key={s.id} className="scheme-static-row">
                <strong>{s.displayName}</strong>
                <span>Recognized · Not payment</span>
              </div>
            ))}
          </div>

          <div className="config-gen">
            <header><span>Integration config</span><button type="button" onClick={() => void copyConfig()}>{configCopied ? "Copied" : "Copy configuration"}</button></header>
            <pre>{configCode}</pre>
          </div>
        </aside>
      </section>

      <section className="output" aria-label="Normalized output">
        <div className="output__intro"><span>03 / NORMALIZE</span><h2>A boring contract.<br /><em>By design.</em></h2><p>The same versioned shape across banks, wallets, links, and chains. Amounts remain exact strings.</p></div>
        <div className="output__panels">
          {intent && (
            <div className="result-card">
              <p className="result-card__title">{intent.scheme.replaceAll("_", " ").toUpperCase()}</p>
              <dl>
                <div><dt>Status</dt><dd>{intent.recognized ? "Recognized" : "Not recognized"}</dd></div>
                <div><dt>Format</dt><dd>{intent.validation.valid ? "Structurally valid" : "Invalid"}</dd></div>
                <div><dt>App support</dt><dd>{intent.support.enabled ? "Enabled" : "Disabled"}</dd></div>
                <div><dt>Maturity</dt><dd>{intentMeta ? <MaturityBadge maturity={intentMeta.maturity} /> : "—"}</dd></div>
                <div><dt>Recipient</dt><dd>{intent.recipient?.name ?? intent.recipient?.id ?? intent.recipient?.address ?? "Unspecified"}</dd></div>
                <div><dt>Amount</dt><dd>{intent.amount ? `${intent.amount} ${intent.currency ?? intent.asset?.symbol ?? ""}`.trim() : "Open amount"}</dd></div>
              </dl>
              <p className="result-card__trust">Recipient trust <strong>UNVERIFIED</strong></p>
            </div>
          )}
          <div className="json-panel">
            <header><span><i /> normalized.intent.json</span><button type="button" onClick={() => void copy()} disabled={!intent}>{copied ? "Copied" : "Copy JSON"}</button></header>
            <pre>{intent ? JSON.stringify(intent, null, 2) : "// Scan or paste a payment QR to inspect its normalized intent."}</pre>
          </div>
        </div>
      </section>

      <section className="schemes-directory" id="schemes" aria-label="Supported schemes directory">
        <div className="panel-heading"><span>04</span><div><small>COVERAGE</small><h2>{paymentSchemes.length || 36} payment formats. One interface.</h2></div></div>
        <div className="schemes-directory__controls">
          <input type="search" placeholder="Search schemes…" value={schemeQuery} onChange={(event) => setSchemeQuery(event.target.value)} aria-label="Search schemes" />
          <div className="chips">
            <button type="button" className={maturityFilter === "all" ? "is-active" : ""} onClick={() => setMaturityFilter("all")}>All</button>
            {MATURITIES.map((m) => (
              <button key={m} type="button" className={maturityFilter === m ? "is-active" : ""} onClick={() => setMaturityFilter(m)}>{m}</button>
            ))}
          </div>
        </div>
        <div className="schemes-directory__groups">
          {groupedDirectory.map(([region, list]) => (
            <div className="directory-group" key={region}>
              <p className="scheme-group__label">{region}</p>
              {list.map((s) => (
                <details className="directory-row" key={s.id}>
                  <summary>
                    <span className="directory-row__name">{s.displayName}</span>
                    <MaturityBadge maturity={s.maturity} />
                  </summary>
                  <dl>
                    <div><dt>Country</dt><dd>{s.countries.length ? s.countries.join(", ") : "Global"}</dd></div>
                    <div><dt>Standard</dt><dd>{s.standard}</dd></div>
                    <div><dt>Category</dt><dd>{s.category.replaceAll("_", " ")}</dd></div>
                    <div><dt>Static amount</dt><dd>{s.staticSupported ? "Supported" : "Not supported"}</dd></div>
                    <div><dt>Dynamic QR</dt><dd>{s.dynamicSupported ? "Supported" : "Not supported"}</dd></div>
                    <div><dt>Network calls</dt><dd>0 · local-only</dd></div>
                    {s.references[0] && <div><dt>Reference</dt><dd><a href={s.references[0]} target="_blank" rel="noreferrer">Specification ↗</a></dd></div>}
                  </dl>
                </details>
              ))}
            </div>
          ))}
          {groupedDirectory.length === 0 && <p className="schemes-directory__empty">No schemes match "{schemeQuery}".</p>}
        </div>
      </section>

      <section className="sdks" id="sdks" aria-label="SDKs and platform bindings">
        <div className="panel-heading"><span>05</span><div><small>EVERYWHERE</small><h2>Use UniPayScan anywhere.</h2></div></div>
        <div className="sdks__grid">
          {SDK_STATUSES.map((sdk) => (
            <div className={`sdk-card sdk-card--${sdk.status}`} key={sdk.name}>
              <div className="sdk-card__header"><i aria-hidden="true" /><strong>{sdk.name}</strong></div>
              <p className="sdk-card__status">{sdk.status === "verified" ? "Verified" : sdk.status === "partial" ? "Partially verified" : "Unverified"}</p>
              <p className="sdk-card__note">{sdk.note}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="why" aria-label="Why UniPayScan">
        <div className="panel-heading"><span>06</span><div><small>RATIONALE</small><h2>Why UniPayScan?</h2></div></div>
        <div className="why__columns">
          <div>
            <h3>One integration</h3>
            <p>Don't maintain separate UPI, Pix, crypto, and payment-link parsers. Ship one dependency instead of five.</p>
          </div>
          <div>
            <h3>One contract</h3>
            <p>Every supported payment format becomes the same normalized <code>PaymentIntent</code>, regardless of which rail it came from.</p>
          </div>
          <div>
            <h3>Your rules</h3>
            <p>UniPayScan can recognize a format your application doesn't support and return <strong>"Recognized · Unsupported by this application"</strong> instead of a flat "Invalid QR".</p>
          </div>
        </div>
      </section>

      <section className="security" id="security" aria-label="Security and privacy">
        <div className="panel-heading"><span>07</span><div><small>LOCAL-FIRST</small><h2>Security &amp; privacy</h2></div></div>
        <div className="security__body">
          <div className="security__diagram" aria-hidden="true">
            <span>QR</span><i>↓</i><span>UniPayScan</span><i>↓</i><span>PaymentIntent</span>
            <small>No server required.</small>
          </div>
          <ul>
            <li>0 network calls in the core parser</li>
            <li>No custody</li>
            <li>No transaction signing</li>
            <li>No automatic navigation</li>
            <li>No recipient-trust claims</li>
            <li>Amounts remain exact strings</li>
          </ul>
        </div>
      </section>

      <section className="contribute" aria-label="Contribute">
        <div className="panel-heading"><span>08</span><div><small>OPEN SOURCE</small><h2>Payments are global. UniPayScan should be too.</h2></div></div>
        <p className="contribute__copy">Missing a payment standard from your country? Add an adapter, test vectors, and specification references.</p>
        <div className="contribute__cta">
          <a href={CONTRIBUTING_URL} target="_blank" rel="noreferrer" className="btn btn--primary">Add a Scheme</a>
          <a href={CONTRIBUTING_URL} target="_blank" rel="noreferrer" className="btn btn--ghost">Contribution Guide</a>
        </div>
      </section>

      <footer>
        <div className="footer__top">
          <div>
            <p className="footer__brand">UniPayScan</p>
            <p className="footer__tagline">Open-source Universal Payment Scanner.</p>
          </div>
          <nav aria-label="Footer">
            <a href={GITHUB_URL} target="_blank" rel="noreferrer">GitHub</a>
            <a href={DOCS_URL} target="_blank" rel="noreferrer">Docs</a>
            <a href="#security">Security</a>
            <a href={CONTRIBUTING_URL} target="_blank" rel="noreferrer">Contributing</a>
            <span>MIT License</span>
          </nav>
        </div>
        <p>Parsing proves structure—not identity, safety, or payment completion.</p>
      </footer>
    </main>
  );
}
