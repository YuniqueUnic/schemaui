#!/usr/bin/env node

/**
 * Web resilience E2E: rendering, saving and previewing against a real
 * `schemaui web` session, including the session dying underneath the page.
 *
 *   1. markdown tables render with visible cell borders;
 *   2. the preview pane shows the document in every advertised format;
 *   3. Save on an untouched document says "Draft saved";
 *   4. explicit `null` and omitted optional fields survive the round trip;
 *   5. when the server goes away, Save and Preview say the connection was
 *      lost — not the browser's bare "Failed to fetch" — and keep the form.
 *
 * Self-contained: spawns its own server on an ephemeral port. Environment:
 *
 *   SCHEMAUI_BIN           schemaui binary (default: target/{debug,release}/schemaui)
 *   SCHEMAUI_FRONTEND_DIR  serve this frontend instead of the embedded one,
 *                          e.g. a fresh `build:embedded` output
 *   HEADLESS=false         watch it run
 */

const { spawn } = require("child_process");
const fs = require("fs");
const net = require("net");
const path = require("path");

const puppeteer = require("puppeteer");
const chalk = require("chalk");

const ROOT = path.resolve(__dirname, "..", "..");
const FIXTURES = path.join(__dirname, "fixtures");
const SCHEMA = path.join(FIXTURES, "prose-save-preview.schema.json");
const CONFIG = path.join(FIXTURES, "prose-save-preview.config.json");
const HEADLESS = process.env.HEADLESS !== "false";

function findBinary() {
  if (process.env.SCHEMAUI_BIN) return process.env.SCHEMAUI_BIN;
  for (const dir of ["target/debug", "target/release"]) {
    const candidate = path.join(ROOT, dir, "schemaui");
    if (fs.existsSync(candidate)) return candidate;
  }
  throw new Error("schemaui binary not found; run `mbx build -p schemaui-cli` or set SCHEMAUI_BIN");
}

function reservePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
    server.on("error", reject);
  });
}

async function startServer() {
  const port = await reservePort();
  const args = [
    "web",
    "--schema", SCHEMA,
    "--config", CONFIG,
    "--host", "127.0.0.1",
    "--port", String(port),
    "--force",
    "--timeout", "180",
  ];
  if (process.env.SCHEMAUI_FRONTEND_DIR) {
    args.push("--frontend", process.env.SCHEMAUI_FRONTEND_DIR);
  }
  const server = spawn(findBinary(), args, { stdio: "ignore" });
  const base = `http://127.0.0.1:${port}`;
  for (let attempt = 0; attempt < 100; attempt++) {
    try {
      const response = await fetch(`${base}/api/v1/session`);
      if (response.ok) return { server, base };
    } catch {
      // not up yet
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  server.kill("SIGKILL");
  throw new Error("server did not become ready");
}

async function launchBrowser() {
  const common = {
    headless: HEADLESS ? "new" : false,
    args: ["--no-sandbox", "--disable-setuid-sandbox", "--disable-gpu"],
  };
  try {
    return await puppeteer.launch({ ...common, channel: "chrome" });
  } catch {
    console.log(chalk.yellow("system Chrome unavailable, using bundled Chromium"));
    return puppeteer.launch(common);
  }
}

class ResilienceE2E {
  constructor(base, server) {
    this.base = base;
    this.server = server;
    this.results = [];
  }

  async setup() {
    this.browser = await launchBrowser();
    this.page = await this.browser.newPage();
    await this.page.setViewport({ width: 1440, height: 900 });
    await this.page.goto(this.base, { waitUntil: "networkidle0", timeout: 30000 });
    await this.page.waitForSelector("table", { timeout: 10000 });
  }

  async teardown() {
    if (this.browser) await this.browser.close();
  }

  record(name, passed, detail = "") {
    this.results.push({ name, passed });
    console.log(
      passed
        ? chalk.green(`  ✅ ${name}`)
        : chalk.red(`  ❌ ${name}${detail ? ` — ${detail}` : ""}`),
    );
  }

  async test(name, fn) {
    try {
      await fn();
      this.record(name, true);
    } catch (error) {
      this.record(name, false, error.message);
    }
  }

  bodyText() {
    return this.page.evaluate(() => document.body.innerText);
  }

  /** The text of the preview payload, or null when the pane shows an error. */
  previewPayload() {
    return this.page.evaluate(() => {
      const code = document.querySelector("pre code[class^='language-']");
      return code ? code.textContent : null;
    });
  }

  async chooseFormat(format) {
    const clicked = await this.page.evaluate((wanted) => {
      const button = Array.from(document.querySelectorAll("button")).find(
        (candidate) => candidate.textContent.trim().toLowerCase() === wanted,
      );
      if (!button) return false;
      button.click();
      return true;
    }, format);
    if (!clicked) throw new Error(`no "${format}" preview button`);
  }

  async pressSave() {
    await this.page.click("button[title^='Save']");
  }

  async waitForToast(pattern, timeout = 8000) {
    await this.page.waitForFunction(
      (source) => {
        const toasts = Array.from(document.querySelectorAll("[data-sonner-toast]"));
        return toasts.some((toast) => new RegExp(source, "i").test(toast.innerText));
      },
      { timeout },
      pattern.source,
    );
  }

  async run() {
    const page = this.page;

    await this.test("markdown table cells have visible borders", async () => {
      const probe = await page.evaluate(() => {
        const cell = document.querySelector(".rich-prose td");
        const head = document.querySelector(".rich-prose th");
        if (!cell || !head) return null;
        const cellStyle = getComputedStyle(cell);
        const headStyle = getComputedStyle(head);
        return {
          cellBorder: parseFloat(cellStyle.borderTopWidth),
          cellBorderStyle: cellStyle.borderTopStyle,
          headBackground: headStyle.backgroundColor,
          bodyBackground: getComputedStyle(document.body).backgroundColor,
          collapse: getComputedStyle(cell.closest("table")).borderCollapse,
        };
      });
      if (!probe) throw new Error("table cells are not inside .rich-prose");
      if (!(probe.cellBorder > 0) || probe.cellBorderStyle === "none") {
        throw new Error(`cells have no border: ${JSON.stringify(probe)}`);
      }
      if (probe.collapse !== "collapse") throw new Error("borders are not collapsed");
      if (probe.headBackground === probe.bodyBackground) {
        throw new Error("header row is indistinguishable from the page");
      }
    });

    await this.test("other markdown elements are styled too", async () => {
      const probe = await page.evaluate(() => {
        const scope = document.querySelector(".rich-prose");
        const code = scope.querySelector("code");
        const link = scope.querySelector("a");
        const del = scope.querySelector("del");
        return {
          hasCode: Boolean(code),
          codeBackground: code ? getComputedStyle(code).backgroundColor : "",
          linkUnderline: link ? getComputedStyle(link).textDecorationLine : "",
          hasDel: Boolean(del),
        };
      });
      if (!probe.hasCode || probe.codeBackground === "rgba(0, 0, 0, 0)") {
        throw new Error(`inline code is unstyled: ${JSON.stringify(probe)}`);
      }
      if (probe.linkUnderline !== "underline") throw new Error("links are not underlined");
      if (!probe.hasDel) throw new Error("strikethrough missing");
    });

    await this.test("preview shows the document with explicit null kept and untouched optional omitted", async () => {
      const payload = await this.previewPayload();
      if (payload === null) throw new Error(`preview pane shows an error: ${(await this.bodyText()).slice(0, 300)}`);
      const document = JSON.parse(payload);
      if (document.name !== "orders") throw new Error(`name changed: ${payload}`);
      if (!("note" in document) || document.note !== null) throw new Error(`null was not kept: ${payload}`);
      if ("region" in document) throw new Error(`untouched optional was injected: ${payload}`);
    });

    for (const format of ["yaml", "toml"]) {
      await this.test(`preview renders ${format} without a failure`, async () => {
        await this.chooseFormat(format);
        await page.waitForFunction(
          (wanted) => {
            const code = document.querySelector("pre code[class^='language-']");
            return code && code.className.includes(wanted);
          },
          { timeout: 5000 },
          format,
        ).catch(async () => {
          throw new Error(`no ${format} payload: ${(await this.bodyText()).slice(0, 300)}`);
        });
      });
    }

    await this.test("Save on an untouched document says Draft saved", async () => {
      await this.chooseFormat("json");
      await this.pressSave();
      await this.waitForToast(/draft saved/);
      const text = await this.bodyText();
      if (/failed to save/i.test(text)) throw new Error("a failure toast appeared as well");
    });

    await this.test("Save twice in a row is not an error", async () => {
      await this.pressSave();
      await this.pressSave();
      await new Promise((resolve) => setTimeout(resolve, 600));
      const text = await this.bodyText();
      if (/failed to save/i.test(text)) throw new Error("second save reported a failure");
    });

    // From here on the server is gone.
    this.server.kill("SIGKILL");
    await new Promise((resolve) => setTimeout(resolve, 500));

    await this.test("Save after the server died names the lost connection", async () => {
      await this.pressSave();
      await this.waitForToast(/lost connection/);
      const text = await this.bodyText();
      if (/^failed to fetch$/im.test(text)) throw new Error("the raw browser message leaked through");
    });

    await this.test("Preview after the server died names the lost connection", async () => {
      await this.chooseFormat("yaml");
      await page.waitForFunction(
        () => /lost connection/i.test(document.body.innerText) && /preview failed/i.test(document.body.innerText),
        { timeout: 5000 },
      );
    });

    await this.test("the form is still usable after the server died", async () => {
      const inputs = await page.$$("input[type='text'], input:not([type]), textarea");
      if (inputs.length === 0) throw new Error("the form disappeared");
      await inputs[0].click({ clickCount: 3 });
      await inputs[0].type("kept-editing");
      const value = await page.evaluate((el) => el.value, inputs[0]);
      if (!value.includes("kept-editing")) throw new Error("typing was lost");
    });
  }
}

async function main() {
  console.log(chalk.blue("🧪 Web resilience E2E"));
  const { server, base } = await startServer();
  const suite = new ResilienceE2E(base, server);
  try {
    await suite.setup();
    await suite.run();
  } catch (error) {
    suite.record("suite setup", false, error.message);
  } finally {
    await suite.teardown();
    server.kill("SIGKILL");
  }

  const passed = suite.results.filter((result) => result.passed).length;
  const failed = suite.results.length - passed;
  console.log(
    failed === 0
      ? chalk.green(`\n✅ ${passed}/${suite.results.length} tests passed`)
      : chalk.red(`\n❌ ${failed}/${suite.results.length} tests failed`),
  );
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((error) => {
  console.error(chalk.red(error));
  process.exit(1);
});
