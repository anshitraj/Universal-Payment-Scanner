import { copyFile, mkdir } from "node:fs/promises";
await mkdir("packages/react/dist", { recursive: true });
await copyFile("packages/react/src/styles.css", "packages/react/dist/styles.css");
