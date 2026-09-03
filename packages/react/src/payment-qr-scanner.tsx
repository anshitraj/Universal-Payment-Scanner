import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ALL_SCHEME_IDS, createScanner, type PaymentIntent, type Scanner } from "@universal-payment-qr/core";
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

export interface PaymentQRScannerProps {
  enabledSchemes?: string[];
  scanner?: Scanner;
  headless?: boolean;
  messages?: Partial<ScannerMessages>;
  className?: string;
  onDetected?(intent: PaymentIntent): void;
  onUnsupported?(intent: PaymentIntent): void;
  onError?(error: Error, intent?: PaymentIntent): void;
}

export function usePaymentQRScanner(options: Pick<PaymentQRScannerProps, "enabledSchemes" | "scanner" | "onDetected" | "onUnsupported" | "onError">) {
  const scanner = useMemo(() => options.scanner ?? createScanner(options.enabledSchemes ? {
    schemes: Object.fromEntries(ALL_SCHEME_IDS.map((id) => [id, options.enabledSchemes!.includes(id)])),
  } : {}), [options.scanner, options.enabledSchemes?.join("|")]);
  const [state, setState] = useState<ScannerState>("idle");
  const [intent, setIntent] = useState<PaymentIntent>();
  const [error, setError] = useState<Error>();

  const scan = useCallback(async (payload: string) => {
    setState("processing");
    setError(undefined);
    try {
      const next = await scanner.scan(payload);
      setIntent(next);
      if (!next.recognized || !next.validation.valid) {
        const problem = new Error(next.validation.errors[0]?.message ?? "Invalid payment QR.");
        setError(problem);
        setState("error");
        options.onError?.(problem, next);
      } else if (!next.supported) {
        setState("unsupported");
        options.onUnsupported?.(next);
      } else {
        setState("success");
        options.onDetected?.(next);
      }
      return next;
    } catch (cause) {
      const problem = cause instanceof Error ? cause : new Error(String(cause));
      setError(problem);
      setState("error");
      options.onError?.(problem);
      return undefined;
    }
  }, [scanner, options.onDetected, options.onUnsupported, options.onError]);

  const scanImage = useCallback(async (file: Blob) => scan(await decodePaymentQRImage(file)), [scan]);
  const reset = useCallback(() => { setIntent(undefined); setError(undefined); setState("idle"); }, []);
  return { scanner, state, setState, intent, error, scan, scanImage, reset };
}

export function PaymentQRScanner(props: PaymentQRScannerProps) {
  const messages = { ...defaultMessages, ...props.messages };
  const controller = usePaymentQRScanner(props);
  const videoRef = useRef<HTMLVideoElement>(null);
  const cameraRef = useRef<CameraSession | undefined>(undefined);
  const [mode, setMode] = useState<"paste" | "camera" | "upload">("paste");
  const [payload, setPayload] = useState("");
  const [dragging, setDragging] = useState(false);
  const [cameras, setCameras] = useState<MediaDeviceInfo[]>([]);
  const [activeCamera, setActiveCamera] = useState(0);
  const [torch, setTorch] = useState(false);

  useEffect(() => () => cameraRef.current?.stop(), []);

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
        onDetected: (value) => { session.stop(); void controller.scan(value); },
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

  if (props.headless) return null;

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
          {(["camera", "paste", "upload"] as const).map((tab) => (
            <button key={tab} type="button" aria-pressed={mode === tab} onClick={() => tab === "camera" ? void startCamera() : setMode(tab)}>
              {tab}
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
          ) : (
            <form className="upqr-paste" onSubmit={(event) => { event.preventDefault(); if (payload.trim()) void controller.scan(payload.trim()); }}>
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
              <div><dt>Recipient</dt><dd>{controller.intent.recipient?.name ?? controller.intent.recipient?.id ?? controller.intent.recipient?.address ?? "Unspecified"}</dd></div>
              <div><dt>Amount</dt><dd>{controller.intent.amount ? `${controller.intent.amount} ${controller.intent.currency ?? controller.intent.asset?.symbol ?? ""}` : "Open amount"}</dd></div>
            </dl>
          )}
        </div>
      </div>
    </section>
  );
}
