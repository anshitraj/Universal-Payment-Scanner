import { createWasmScanner } from "./wasm-loader.node.js";
import { buildApi, preset, type Scanner } from "./scanner-factory.js";

export * from "./types.js";
export type { Scanner };
export { preset };

const api = buildApi(createWasmScanner);
export const createScanner = api.createScanner;
export const parsePaymentQR = api.parsePaymentQR;
export const normalizePaymentQR = api.normalizePaymentQR;
export const detectPaymentQR = api.detectPaymentQR;
export const validatePaymentQR = api.validatePaymentQR;
export const getSupportedSchemes = api.getSupportedSchemes;
export const getCapabilities = api.getCapabilities;
