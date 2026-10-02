import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ALL_SCHEME_IDS, createScanner, type PaymentIntent, type Scanner } from "unipayscan";
import { CameraSession, decodePaymentQRImage } from "@universal-payment-qr/scanner";
import "./styles.css";

export interface ScannerMessages {
  idle: string;
  cameraPermission: string;
  scanning: string;
  invalid: string;
  notPayment: string;
  disabled: string;
  unsupported: string;
  upload: string;
}

const defaultMessages: ScannerMessages = {
  idle: "Paste, upload, or scan a payment QR",
  cameraPermission: "Camera access is needed only while scanning.",
  scanning: "Looking for a payment code…",
  invalid: "This payment QR is malformed.",
  notPayment: "That QR is valid, but it is not a recognized payment request.",
  disabled: "Recognized, but this payment scheme is disabled here.",
  unsupported: "This payment scheme is not supported yet.",
  upload: "Drop a QR image or choose a file",
};

type ScannerState = "idle" | "requesting" | "scanning" | "processing" | "success" | "unsupported" | "error";

/** Where a scanned payload came from. `upi_id` means the user typed a UPI ID into the UPI ID tab,
 * so any payee name on the intent is one they typed themselves, not one printed in a QR - a host
 * should label it that way. */
export type ScanSource = "camera" | "paste" | "upload" | "upi_id";

export interface ScanContext {
  source: ScanSource;
}

export interface PaymentQRScannerProps {
  enabledSchemes?: string[];
  scanner?: Scanner;
  headless?: boolean;
  messages?: Partial<ScannerMessages>;
  className?: string;
  /** Drives the paste tab from outside (e.g. a host app's "try an example" button) — set a new
   * `key` each time to force a re-scan, even if `payload` is unchanged from the last one. */
  example?: { key: string | number; payload: string } | undefined;
  /** Adds a "UPI ID" tab: the user types a UPI ID (plus an optional payee name and amount) and it
   * is scanned as the `upi://pay` link NPCI defines for it. Off by default because it is
   * India-specific, and because a bare `name@bank` can't be told apart from an email address -
   * choosing this tab is what says "this is a UPI ID", so the core never has to guess. */
  upiIdEntry?: boolean;
  onDetected?(intent: PaymentIntent, context?: ScanContext): void;
  onUnsupported?(intent: PaymentIntent, context?: ScanContext): void;
  onError?(error: Error, intent?: PaymentIntent, context?: ScanContext): void;
}

/** The `upi://pay` link for a typed UPI ID. `@` stays literal in `pa` (as in NPCI's own examples);
 * everything else is percent-encoded, so a stray `&` in a name can't inject another parameter.
 * Validation is left to the core's UPI parser, the one source of truth for what a valid payee
 * address and amount are. */
function upiPayLink(upiId: string, payeeName: string, amount: string): string {
  const params = [`pa=${encodeURIComponent(upiId).replace(/%40/g, "@")}`];
  if (payeeName) params.push(`pn=${encodeURIComponent(payeeName)}`);
  if (amount) params.push(`am=${encodeURIComponent(amount)}`);
  params.push("cu=INR");
  return `upi://pay?${params.join("&")}`;
}

export function usePaymentQRScanner(options: Pick<PaymentQRScannerProps, "enabledSchemes" | "scanner" | "onDetected" | "onUnsupported" | "onError">) {
  const scanner = useMemo(() => options.scanner ?? createScanner(options.enabledSchemes ? {
    schemes: Object.fromEntries(ALL_SCHEME_IDS.map((id) => [id, options.enabledSchemes!.includes(id)])),
  } : {}), [options.scanner, options.enabledSchemes?.join("|")]);
  const [state, setState] = useState<ScannerState>("idle");
  const [intent, setIntent] = useState<PaymentIntent>();
  const [error, setError] = useState<Error>();

  const scan = useCallback(async (payload: string, context?: ScanContext) => {
    setState("processing");
    setError(undefined);
    try {
      const next = await scanner.scan(payload);
      setIntent(next);
      if (!next.recognized || !next.validation.valid) {
        const problem = new Error(next.validation.errors[0]?.message ?? "Invalid payment QR.");
        setError(problem);
        setState("error");
        options.onError?.(problem, next, context);
      } else if (!next.supported) {
        setState("unsupported");
        options.onUnsupported?.(next, context);
      } else {
        setState("success");
        options.onDetected?.(next, context);
      }
      return next;
    } catch (cause) {
      const problem = cause instanceof Error ? cause : new Error(String(cause));
      setError(problem);
      setState("error");
      options.onError?.(problem, undefined, context);
      return undefined;
    }
  }, [scanner, options.onDetected, options.onUnsupported, options.onError]);

  const scanImage = useCallback(
    async (file: Blob) => scan(await decodePaymentQRImage(file), { source: "upload" }),
    [scan],
  );
  const reset = useCallback(() => { setIntent(undefined); setError(undefined); setState("idle"); }, []);
  return { scanner, state, setState, intent, error, scan, scanImage, reset };
}

export function PaymentQRScanner(props: PaymentQRScannerProps) {
  const messages = { ...defaultMessages, ...props.messages };
  const controller = usePaymentQRScanner(props);
  const videoRef = useRef<HTMLVideoElement>(null);
  const cameraRef = useRef<CameraSession | undefined>(undefined);
  const [mode, setMode] = useState<"paste" | "camera" | "upload" | "upi_id">("paste");
  const [payload, setPayload] = useState("");
  const [upiId, setUpiId] = useState("");
  const [upiName, setUpiName] = useState("");
  const [upiAmount, setUpiAmount] = useState("");
  const [dragging, setDragging] = useState(false);
  const [cameras, setCameras] = useState<MediaDeviceInfo[]>([]);
  const [activeCamera, setActiveCamera] = useState(0);
  const [torch, setTorch] = useState(false);

  useEffect(() => () => cameraRef.current?.stop(), []);

  useEffect(() => {
    if (!props.example) return;
    setMode("paste");
    setPayload(props.example.payload);
    void controller.scan(props.example.payload, { source: "paste" });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.example?.key]);

  const startCamera = async (deviceId?: string) => {
    setMode("camera");
    controller.setState("requesting");
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    try {
      const session = new CameraSession();
      cameraRef.current = session;
      if (!videoRef.current) throw new Error("Camera preview is unavailable.");
      await session.start(videoRef.current, {
        ...(deviceId ? { deviceId } : {}),
        onDetected: (value) => { session.stop(); void controller.scan(value, { source: "camera" }); },
        onError: (cameraError) => props.onError?.(cameraError),
      });
      setCameras(await CameraSession.listCameras());
      controller.setState("scanning");
    } catch (cause) {
      const problem = cause instanceof Error ? cause : new Error(String(cause));
      controller.setState("error");
      props.onError?.(problem);
    }
  };

  const switchCamera = async () => {
    if (cameras.length < 2) return;
    const next = (activeCamera + 1) % cameras.length;
    setActiveCamera(next);
    setTorch(false);
    await startCamera(cameras[next]?.deviceId);
  };

  const toggleTorch = async () => {
    try {
      await cameraRef.current?.switchTorch(!torch);
      setTorch((current) => !current);
    } catch (cause) {
      props.onError?.(cause instanceof Error ? cause : new Error(String(cause)));
    }
  };

  const pickFile = async (file?: File) => {
    if (!file) return;
    setMode("upload");
    await controller.scanImage(file);
  };

  const submitUpiId = () => {
    const id = upiId.trim();
    if (!id) return;
    // Someone pasting a whole UPI link into the ID box gets it scanned as-is rather than wrapped
    // in a second link.
    const link = /^upi:/i.test(id) ? id : upiPayLink(id, upiName.trim(), upiAmount.trim());
    void controller.scan(link, { source: "upi_id" });
  };

  const scanAnother = () => {
    controller.reset();
    if (mode === "camera") void startCamera(cameras[activeCamera]?.deviceId);
    else if (mode === "paste") setPayload("");
  };

  const tabs = props.upiIdEntry ? (["camera", "paste", "upload", "upi_id"] as const) : (["camera", "paste", "upload"] as const);
  const recipient = controller.intent?.recipient;
  const recipientAddress = recipient?.id ?? recipient?.address;

  if (props.headless) return null;

  const done = controller.state === "success" || controller.state === "unsupported" || controller.state === "error";

  const statusText = controller.state === "scanning" ? messages.scanning
    : controller.state === "unsupported" ? (controller.intent?.support.message ?? messages.unsupported)
    : controller.state === "error" ? (controller.intent?.recognized === false ? messages.notPayment : controller.error?.message ?? messages.invalid)
    : controller.state === "success" ? `${controller.intent?.scheme.replaceAll("_", " ")} recognized`
    : messages.idle;

  return (
    <section className={`upqr-scanner ${props.className ?? ""}`} aria-label="Payment QR scanner">
      <div className="upqr-scanner__rail" aria-hidden="true"><span>01</span><i /><span>INTENT</span></div>
      <div className="upqr-scanner__main">
        <header className="upqr-scanner__header">
          <div>
            <p className="upqr-kicker">LOCAL PAYMENT INTELLIGENCE</p>
            <h2>Read the intent.<br /><em>Never move the money.</em></h2>
          </div>
          <span className="upqr-private"><i /> ON-DEVICE</span>
        </header>

        <nav className="upqr-tabs" aria-label="Input method">
          {tabs.map((tab) => (
            <button key={tab} type="button" aria-pressed={mode === tab} onClick={() => tab === "camera" ? void startCamera() : setMode(tab)}>
              {tab === "upi_id" ? "UPI ID" : tab}
            </button>
          ))}
        </nav>

        <div className={`upqr-stage upqr-stage--${mode}`}>
          {mode === "camera" ? (
            <div className="upqr-camera">
              <video ref={videoRef} muted playsInline aria-label="Camera preview" />
              <div className="upqr-reticle" aria-hidden="true"><span /><span /><span /><span /></div>
              {controller.state === "requesting" && <p>{messages.cameraPermission}</p>}
              {controller.state === "scanning" && (
                <div className="upqr-camera__controls">
                  <button type="button" onClick={() => void toggleTorch()} aria-pressed={torch}>{torch ? "Torch off" : "Torch"}</button>
                  {cameras.length > 1 && <button type="button" onClick={() => void switchCamera()}>Switch camera</button>}
                </div>
              )}
            </div>
          ) : mode === "upload" ? (
            <label className={`upqr-dropzone ${dragging ? "is-dragging" : ""}`} onDragOver={(event) => { event.preventDefault(); setDragging(true); }} onDragLeave={() => setDragging(false)} onDrop={(event) => { event.preventDefault(); setDragging(false); void pickFile(event.dataTransfer.files[0]); }}>
              <input type="file" accept="image/png,image/jpeg,image/webp,image/gif" onChange={(event) => void pickFile(event.target.files?.[0])} />
              <span className="upqr-upload-mark" aria-hidden="true">↗</span>
              <strong>{messages.upload}</strong>
              <small>PNG, JPEG, WebP or GIF · processed locally</small>
            </label>
          ) : mode === "upi_id" ? (
            <form className="upqr-upi" onSubmit={(event) => { event.preventDefault(); submitUpiId(); }}>
              <label htmlFor="upqr-upi-id">UPI ID</label>
              <input id="upqr-upi-id" value={upiId} maxLength={255} onChange={(event) => setUpiId(event.target.value)} placeholder="name@bank" inputMode="email" autoCapitalize="none" autoComplete="off" autoCorrect="off" spellCheck={false} required />
              <label htmlFor="upqr-upi-name">Name you expect <em>optional · unverified</em></label>
              <input id="upqr-upi-name" value={upiName} maxLength={100} onChange={(event) => setUpiName(event.target.value)} placeholder="e.g. Ram Lal" autoComplete="off" />
              <label htmlFor="upqr-upi-amount">Amount (INR) <em>optional</em></label>
              <input id="upqr-upi-amount" value={upiAmount} maxLength={12} onChange={(event) => setUpiAmount(event.target.value)} placeholder="Leave empty to enter it in your UPI app" inputMode="decimal" autoComplete="off" />
              <p className="upqr-upi__note">
                The name is only what you typed. Your UPI app shows the name the bank has on record before you enter your PIN - pay only if that matches.
              </p>
              <button type="submit" disabled={!upiId.trim() || controller.state === "processing"}>
                {controller.state === "processing" ? "Checking…" : "Check UPI ID"}<span aria-hidden="true">→</span>
              </button>
            </form>
          ) : (
            <form className="upqr-paste" onSubmit={(event) => { event.preventDefault(); if (payload.trim()) void controller.scan(payload.trim(), { source: "paste" }); }}>
              <label htmlFor="upqr-payload">Raw QR payload</label>
              <textarea id="upqr-payload" value={payload} maxLength={8192} onChange={(event) => setPayload(event.target.value)} placeholder="upi://pay?pa=merchant@bank…" spellCheck={false} />
              <button type="submit" disabled={!payload.trim() || controller.state === "processing"}>
                {controller.state === "processing" ? "Parsing…" : "Parse intent"}<span aria-hidden="true">→</span>
              </button>
            </form>
          )}
        </div>

        <div className={`upqr-status upqr-status--${controller.state}`} role="status" aria-live="polite">
          <span className="upqr-status__signal" aria-hidden="true" />
          <p>{statusText}</p>
          {controller.intent && controller.state !== "error" && (
            <dl>
              <div><dt>Scheme</dt><dd>{controller.intent.scheme}</dd></div>
              <div>
                <dt>Recipient</dt>
                <dd>
                  {recipientAddress ?? recipient?.name ?? "Unspecified"}
                  {recipientAddress && recipient?.name && <small>{recipient.name} · unverified</small>}
                </dd>
              </div>
              <div><dt>Amount</dt><dd>{controller.intent.amount ? `${controller.intent.amount} ${controller.intent.currency ?? controller.intent.asset?.symbol ?? ""}` : "Open amount"}</dd></div>
            </dl>
          )}
          {done && (
            <button type="button" className="upqr-rescan" onClick={scanAnother}>Scan another →</button>
          )}
        </div>
      </div>
    </section>
  );
}
