import { createScanner, preset } from "@universal-payment-qr/core";

const scanner = createScanner(preset("all-stable", {
  schemes: { solana_pay: false },
}));

const result = await scanner.scan(process.argv[2] ?? "upi://pay?pa=merchant%40bank&cu=INR");
process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);

