import { rm } from "node:fs/promises";

for (const path of ["packages/wasm/generated/.gitignore", "packages/wasm/generated-node/.gitignore"]) {
  await rm(path, { force: true });
}

