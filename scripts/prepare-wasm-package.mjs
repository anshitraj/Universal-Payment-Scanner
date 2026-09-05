import { rm } from "node:fs/promises";

for (const path of ["packages/core/generated/.gitignore", "packages/core/generated-node/.gitignore"]) {
  await rm(path, { force: true });
}

