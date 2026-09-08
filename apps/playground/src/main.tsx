import { Buffer } from "buffer";
import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource-variable/manrope";
import { App } from "./app.js";
import "./app.css";

// @solana/web3.js (used by the payment-launcher panel to build/serialize transactions) expects
// Node's Buffer global at runtime - the browser has no such global, so without this the wallet
// payment flow throws as soon as it touches a transaction. Vite doesn't polyfill Node globals by
// default, so this shim is that polyfill, scoped to just this one global.
window.Buffer = Buffer;

ReactDOM.createRoot(document.getElementById("root")!).render(<React.StrictMode><App /></React.StrictMode>);

