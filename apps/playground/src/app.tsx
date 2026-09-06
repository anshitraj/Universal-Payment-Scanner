import { useEffect, useMemo, useRef, useState } from "react";
import { createScanner, type PaymentIntent, type SchemeMetadata } from "unipayscan";
import { PaymentQRScanner } from "@universal-payment-qr/react";
import { REGION_ORDER, NON_PAYMENT_SCHEME_IDS, regionFor } from "./data/regions";
import { SCANNER_EXAMPLES } from "./data/examples";
import { SDK_STATUSES } from "./data/sdks";
import { SCHEME_LOGOS } from "./data/logos";

const GITHUB_URL = "https://github.com/anshitraj/Universal-Payment-Scanner";
const DOCS_URL = `${GITHUB_URL}/blob/main/docs/schema.md`;
const CONTRIBUTING_URL = `${GITHUB_URL}/blob/main/CONTRIBUTING.md`;
const NON_PAYMENT_IDS = new Set<string>(NON_PAYMENT_SCHEME_IDS);
const MATURITIES = ["stable", "beta", "community", "experimental"] as const;
const DIRECTORY_PAGE_SIZE = 10;

function MaturityBadge({ maturity }: { maturity: string }) {
  return <span className={`maturity maturity--${maturity}`}>{maturity}</span>;
}

function SchemeIcon({ id }: { id: string }) {
  const src = SCHEME_LOGOS[id];
  if (src) return <img className="scheme-icon" src={src} alt="" width={16} height={16} />;
  if (id === "upi") return <span className="scheme-icon scheme-icon--text" aria-hidden="true">U</span>;
  return null;
}

const PACKAGE_TABS = [
  { id: "npm", label: "JavaScript", logo: "/logos/nodedotjs.svg", command: "npm install unipayscan" },
  { id: "python", label: "Python", logo: "/logos/python.svg", command: "pip install unipayscan" },
  { id: "flutter", label: "Flutter", logo: "/logos/flutter.svg", command: "flutter pub add unipayscan" },
] as const;

function InstallBox({ dark }: { dark?: boolean }) {
  const [tab, setTab] = useState<(typeof PACKAGE_TABS)[number]["id"]>("npm");
  const [copied, setCopied] = useState(false);
  const active = PACKAGE_TABS.find((t) => t.id === tab) ?? PACKAGE_TABS[0];
  const copy = async () => {
    await navigator.clipboard.writeText(active.command);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };
  return (
    <div className={`install${dark ? " install--dark" : ""}`}>
      <div className="install-tabs" role="tablist" aria-label="Install command by package">
        {PACKAGE_TABS.map((t) => (
          <button key={t.id} type="button" role="tab" aria-selected={t.id === tab} className={t.id === tab ? "is-active" : ""} onClick={() => { setTab(t.id); setCopied(false); }}>
            <img src={t.logo} alt="" width={16} height={16} />
            {t.label}
          </button>
        ))}
      </div>
      <div className={`install-box${dark ? " install-box--dark" : ""}`}>
        <span className="install-box__prompt" aria-hidden="true">&gt;</span>
        <code>{active.command}</code>
        <button type="button" onClick={() => void copy()}>{copied ? "Copied" : "Copy"}</button>
      </div>
    </div>
  );
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
  const [rawDirectoryPage, setRawDirectoryPage] = useState(0);
  const outputRef = useRef<HTMLElement>(null);
  const directoryRef = useRef<HTMLDivElement>(null);

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
  const flatDirectory = useMemo(() => groupByRegion(filteredDirectory).flatMap(([, list]) => list), [filteredDirectory]);
  const directoryPageCount = Math.max(1, Math.ceil(flatDirectory.length / DIRECTORY_PAGE_SIZE));
  const directoryPage = Math.min(rawDirectoryPage, directoryPageCount - 1);
  const pagedDirectory = useMemo(
    () => flatDirectory.slice(directoryPage * DIRECTORY_PAGE_SIZE, (directoryPage + 1) * DIRECTORY_PAGE_SIZE),
    [flatDirectory, directoryPage],
  );
  const groupedDirectory = useMemo(() => groupByRegion(pagedDirectory), [pagedDirectory]);

  useEffect(() => { setRawDirectoryPage(0); }, [schemeQuery, maturityFilter]);
  const goToDirectoryPage = (next: number) => {
    setRawDirectoryPage(next);
    directoryRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  };

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

  useEffect(() => {
    if (!intent) return;
    outputRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, [intent]);

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
        <div className="masthead__actions">
          <a href={GITHUB_URL} target="_blank" rel="noreferrer" className="btn btn--ghost btn--sm">GitHub ↗</a>
          <a href="#playground" className="btn btn--primary btn--sm">Try Playground</a>
        </div>
      </header>

      <section className="hero" id="top">
        <div className="hero__grid">
          <div className="hero__copy">
            <h1>Scan any payment QR.<br /><span className="accent">Get one clean intent.</span></h1>
            <p className="hero__lead">Open-source infrastructure for detecting, validating, and normalizing payment QRs, wallet addresses, and payment links across fiat and crypto. Recognize the standard. Validate the structure. Normalize the intent. Your application decides what happens next.</p>

            <InstallBox />
            <p className="hero__subinstall">Or via the CLI: <code>npx unipayscan scan "upi://pay?..."</code></p>

            <div className="hero__cta">
              <a href="#playground" className="btn btn--primary">Try Playground</a>
              <a href={GITHUB_URL} target="_blank" rel="noreferrer" className="btn btn--ghost">View on GitHub ↗</a>
            </div>
            <div className="hero__rule"><span>NO CUSTODY</span><span>NO AUTO-NAVIGATION</span><span>LOCAL BY DEFAULT</span></div>
          </div>

          <div className="terminal hero__terminal" aria-hidden="true">
            <div className="terminal__bar"><i /><i /><i /><span>unipayscan</span></div>
            <div className="terminal__body">
              <p><span className="t-prompt">&gt;</span> scanner.scan(qr)</p>
              <p className="t-dim">upi://pay?pa=merchant@bank&amp;am=499.00&amp;cu=INR</p>
              <p className="t-ok">✓ Recognized <span className="t-dim">upi</span></p>
              <p className="t-ok">✓ Valid <span className="t-dim">structurally</span></p>
              <p className="t-ok">✓ Normalized <span className="t-dim">→ PaymentIntent</span></p>
              <p className="t-json">{"{"}</p>
              <p className="t-json">&nbsp;&nbsp;"scheme": "upi",</p>
              <p className="t-json">&nbsp;&nbsp;"amount": "499.00",</p>
              <p className="t-json">&nbsp;&nbsp;"currency": "INR"</p>
              <p className="t-json">{"}"}</p>
              <p><span className="t-cursor" /></p>
            </div>
            <div className="terminal__stats">
              <div><strong>{paymentSchemes.length || 36}</strong><span>Schemes</span></div>
              <div><strong>0</strong><span>Network calls</span></div>
              <div><strong>{SDK_STATUSES.length}</strong><span>SDKs</span></div>
            </div>
          </div>
        </div>
      </section>

      <section className="compare" aria-label="What UniPayScan does">
        <div className="section-head">
          <span className="eyebrow">How it works</span>
          <h2>One input. <span className="accent">Different payment systems.</span></h2>
        </div>
        <div className="compare__grid">
          <div className="compare__old">
            <p className="compare__tag">Without UniPayScan</p>
            <ul>
              <li><SchemeIcon id="upi" />UPI parser</li>
              <li><SchemeIcon id="pix" />Pix parser</li>
              <li><SchemeIcon id="bitcoin" />Bitcoin parser</li>
              <li><SchemeIcon id="paypal" />PayPal link parser</li>
              <li><SchemeIcon id="solana_pay" />Solana Pay parser</li>
              <li className="compare__muted"><SchemeIcon id="walletconnect" />WalletConnect — not a payment</li>
            </ul>
            <p className="compare__foot">Five formats to maintain. Five places to get it wrong.</p>
          </div>
          <div className="compare__new">
            <p className="compare__tag compare__tag--accent">With UniPayScan</p>
            <h3>One normalized <code>PaymentIntent</code></h3>
            <div className="terminal terminal--mini">
              <div className="terminal__body">
                <p><span className="t-prompt">&gt;</span> scanner.scan(qr)</p>
                <p className="t-ok">✓ Recognize <span className="t-dim">→ format detected</span></p>
                <p className="t-ok">✓ Validate <span className="t-dim">→ structure checked</span></p>
                <p className="t-ok">✓ Normalize <span className="t-dim">→ PaymentIntent</span></p>
              </div>
            </div>
            <div className="compare__footstrip">
              <div><strong>Detect</strong><span>Identify the standard</span></div>
              <div><strong>Validate</strong><span>Check the structure</span></div>
              <div><strong>Normalize</strong><span>One typed contract</span></div>
            </div>
          </div>
        </div>
      </section>

      <section className="workspace" aria-label="Scanner playground" id="playground">
        <div className="workspace__scanner">
          <div className="try-examples">
            <p>Try an example:</p>
            <div className="try-examples__buttons">
              {SCANNER_EXAMPLES.map((ex) => (
                <button key={ex.key} type="button" onClick={() => setExample({ key: `${ex.key}-${Date.now()}`, payload: ex.payload })}><SchemeIcon id={ex.key} />{ex.label}</button>
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
                      <span><SchemeIcon id={s.id} /><strong>{s.displayName}</strong><MaturityBadge maturity={s.maturity} /></span>
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
                <span className="scheme-static-row__name"><SchemeIcon id={s.id} /><strong>{s.displayName}</strong></span>
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

      <section className="output" aria-label="Normalized output" ref={outputRef}>
        <div className="output__intro"><span className="eyebrow eyebrow--inverse">Normalize</span><h2>A boring contract.<br /><span className="accent">By design.</span></h2><p>The same versioned shape across banks, wallets, links, and chains. Amounts remain exact strings.</p><a className="link-arrow" href={DOCS_URL} target="_blank" rel="noreferrer">Read the schema reference ↗</a></div>
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
        <div className="schemes-directory__groups" ref={directoryRef}>
          {groupedDirectory.map(([region, list]) => (
            <div className="directory-group" key={region}>
              <p className="scheme-group__label">{region}</p>
              {list.map((s) => (
                <details className="directory-row" key={s.id}>
                  <summary>
                    <SchemeIcon id={s.id} />
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
        {flatDirectory.length > DIRECTORY_PAGE_SIZE && (
          <div className="directory-pagination">
            <button type="button" onClick={() => goToDirectoryPage(directoryPage - 1)} disabled={directoryPage === 0}>← Previous</button>
            <span>Page {directoryPage + 1} of {directoryPageCount} · {flatDirectory.length} schemes</span>
            <button type="button" onClick={() => goToDirectoryPage(directoryPage + 1)} disabled={directoryPage >= directoryPageCount - 1}>Next →</button>
          </div>
        )}
      </section>

      <section className="sdks" id="sdks" aria-label="SDKs and platform bindings">
        <div className="panel-heading"><span>05</span><div><small>EVERYWHERE</small><h2>Use UniPayScan anywhere.</h2></div></div>
        <div className="sdks__grid">
          {SDK_STATUSES.map((sdk) => (
            <div className={`sdk-card sdk-card--${sdk.status}`} key={sdk.name}>
              <div className="sdk-card__header"><img className="sdk-card__logo" src={sdk.logo} alt="" width={22} height={22} /><strong>{sdk.name}</strong><i aria-hidden="true" /></div>
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

      <section className="faq" aria-label="Frequently asked questions">
        <div className="panel-heading"><span>08</span><div><small>FAQ</small><h2>Questions, answered.</h2></div></div>
        <div className="faq__list">
          <details>
            <summary><span>What payment formats does UniPayScan support?</span><i aria-hidden="true">+</i></summary>
            <p>{paymentSchemes.length || 36} schemes across UPI, Pix, EMVCo, PayPal, Bitcoin, Ethereum, Solana Pay, and more — see the full directory above.</p>
          </details>
          <details>
            <summary><span>Does scanning make any network calls?</span><i aria-hidden="true">+</i></summary>
            <p>No. The core parser runs entirely on-device — 0 network calls, no telemetry, no custody of funds or keys.</p>
          </details>
          <details>
            <summary><span>What license is it under?</span><i aria-hidden="true">+</i></summary>
            <p>MIT. Free to use in commercial and open-source projects.</p>
          </details>
          <details>
            <summary><span>How do I add a payment format that's missing?</span><i aria-hidden="true">+</i></summary>
            <p>Open a pull request with an adapter, test vectors, and a specification reference — see the Contribution Guide below.</p>
          </details>
        </div>
      </section>

      <section className="cta-band" aria-label="Contribute">
        <div className="cta-band__copy">
          <span className="eyebrow eyebrow--inverse">Open source · MIT licensed</span>
          <h2>Payments are global.<br /><span className="accent">UniPayScan should be too.</span></h2>
          <p>Missing a payment standard from your country? Add an adapter, test vectors, and specification references.</p>
          <div className="cta-band__buttons">
            <a href={CONTRIBUTING_URL} target="_blank" rel="noreferrer" className="btn btn--primary">Add a Scheme</a>
            <a href={CONTRIBUTING_URL} target="_blank" rel="noreferrer" className="btn btn--ghost btn--inverse">Contribution Guide</a>
          </div>
        </div>
        <InstallBox dark />
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
