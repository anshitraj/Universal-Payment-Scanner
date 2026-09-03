import type { IScannerControls } from "@zxing/browser";

export async function decodePaymentQRImage(source: Blob | HTMLImageElement | string): Promise<string> {
  const { BrowserQRCodeReader } = await import("@zxing/browser");
  const reader = new BrowserQRCodeReader(undefined, { delayBetweenScanAttempts: 100 });
  let objectUrl: string | undefined;
  try {
    const url = source instanceof Blob ? (objectUrl = URL.createObjectURL(source)) : source;
    const result = typeof url === "string" ? await reader.decodeFromImageUrl(url) : await reader.decodeFromImageElement(url);
    return result.getText();
  } finally {
    if (objectUrl) URL.revokeObjectURL(objectUrl);
  }
}

export interface CameraSessionOptions {
  deviceId?: string;
  onDetected(payload: string): void;
  onError?(error: Error): void;
}

export class CameraSession {
  #controls: IScannerControls | undefined;

  async start(video: HTMLVideoElement, options: CameraSessionOptions): Promise<void> {
    const { BrowserQRCodeReader } = await import("@zxing/browser");
    const reader = new BrowserQRCodeReader(undefined, { delayBetweenScanAttempts: 150 });
    this.stop();
    const constraints: MediaStreamConstraints = {
      audio: false,
      video: options.deviceId ? { deviceId: { exact: options.deviceId } } : { facingMode: { ideal: "environment" } },
    };
    this.#controls = await reader.decodeFromConstraints(constraints, video, (result, error) => {
      if (result) options.onDetected(result.getText());
      if (error && error.name !== "NotFoundException") options.onError?.(error);
    });
  }

  stop(): void {
    this.#controls?.stop();
    this.#controls = undefined;
  }

  async switchTorch(enabled: boolean): Promise<void> {
    if (!this.#controls) throw new Error("Camera is not active.");
    if (!this.#controls.switchTorch) throw new Error("Torch control is not supported by this camera.");
    await this.#controls.switchTorch(enabled);
  }

  static async listCameras(): Promise<MediaDeviceInfo[]> {
    const { BrowserQRCodeReader } = await import("@zxing/browser");
    return BrowserQRCodeReader.listVideoInputDevices();
  }
}
