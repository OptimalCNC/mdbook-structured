import { spawn } from "node:child_process";
import fs from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import TOML from "@iarna/toml";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const fixture = resolve(
  repositoryRoot,
  "crates/mdbook-structured/tests/fixtures/integration-book",
);
const playwrightBook = resolve(repositoryRoot, "target/playwright-book");
const binary = resolve(repositoryRoot, "target/debug/mdbook-structured");

function run(command, args, options = {}) {
  return new Promise((resolveRun, reject) => {
    const child = spawn(command, args, {
      cwd: repositoryRoot,
      stdio: "inherit",
      ...options,
    });
    child.once("error", reject);
    child.once("close", (code, signal) => {
      if (code === 0) {
        resolveRun();
      } else {
        reject(
          new Error(
            `${command} ${args.join(" ")} failed with ${
              signal === null ? `exit code ${code}` : `signal ${signal}`
            }`,
          ),
        );
      }
    });
  });
}

export default async function prepareFixture() {
  await run("cargo", ["build", "-p", "mdbook-structured"]);

  await fs.rm(playwrightBook, { recursive: true, force: true });
  await fs.cp(fixture, playwrightBook, { recursive: true });

  const bookToml = resolve(playwrightBook, "book.toml");
  const configuration = TOML.parse(await fs.readFile(bookToml, "utf8"));
  const quotedBinary = JSON.stringify(binary);
  configuration.preprocessor.structured.command = `${quotedBinary} render`;
  configuration.preprocessor["structured-links"].command =
    `${quotedBinary} rewrite-links`;
  await fs.writeFile(bookToml, TOML.stringify(configuration));

  await run(binary, ["install"], { cwd: playwrightBook });
  await run("mdbook", ["build", playwrightBook]);
}
