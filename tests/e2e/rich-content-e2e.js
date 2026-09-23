#!/usr/bin/env node

/**
 * Rich Content E2E Test Suite
 *
 * Drives a real browser against a real `schemaui web` session serving
 * `examples/rich-content.schema.json`, asserting the behaviours the rich
 * content feature promises:
 *
 *   1. the session payload carries rendered assets and the content_render
 *      capability;
 *   2. option figures, overridden labels and sanitised markdown render;
 *   3. hovering a thumbnail enlarges it, viewport-aware;
 *   4. clicking opens the fullscreen viewer with copy/download, and Escape
 *      closes it;
 *   5. the Mermaid editor previews as you type and reports parse errors;
 *   6. switching the theme swaps the diagram variants.
 *
 * Self-contained: the server is spawned here on an ephemeral port and
 * terminated on exit. Usage:
 *
 *   node e2e/rich-content-e2e.js            # headless
 *   HEADLESS=false node e2e/rich-content-e2e.js
 */

const { spawn } = require("child_process");
const fs = require("fs");
const http = require("http");
const net = require("net");
const path = require("path");

const puppeteer = require("puppeteer");
const chalk = require("chalk");

const ROOT = path.resolve(__dirname, "..", "..");
const SCHEMA = path.join(ROOT, "examples", "rich-content.schema.json");
const DEFAULTS = path.join(ROOT, "examples", "rich-content-defaults.json");
const HEADLESS = process.env.HEADLESS !== "false";

function findBinary() {
  for (const dir of ["target/debug", "target/release"]) {
    const candidate = path.join(ROOT, dir, "schemaui");
    if (fs.existsSync(candidate)) return candidate;
  }
  throw new Error("schemaui binary not found; run `cargo build -p schemaui-cli` first");
}

function reservePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, "127.0.0.1", () => {
      const port = server.address().port;
      server.close(() => resolve(port));
    });
    server.on("error", reject);
  });
}

function fetchJson(url) {
  return new Promise((resolve, reject) => {
    http
      .get(url, (res) => {
        let body = "";
        res.on("data", (chunk) => (body += chunk));
        res.on("end", () => {
          try {
            resolve({ status: res.statusCode, body: JSON.parse(body) });
          } catch (error) {
            reject(error);
          }
        });
      })
      .on("error", reject);
  });
}

async function startServer() {
  const port = await reservePort();
  const server = spawn(
    findBinary(),
    [
      "web",
      "--schema", SCHEMA,
      "--config", DEFAULTS,
      "--host", "127.0.0.1",
      "--port", String(port),
      "--force",
      "--timeout", "120",
    ],
    { stdio: "ignore" },
  );
  const base = `http://127.0.0.1:${port}`;
  for (let attempt = 0; attempt < 100; attempt++) {
    try {
      const { status } = await fetchJson(`${base}/api/v1/session`);
      if (status === 200) return { server, base };
    } catch {
      // not up yet
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  server.kill();
  throw new Error("server did not become ready");
}

class RichContentE2E {
  constructor(base) {
    this.base = base;
    this.results = [];
    this.browser = null;
    this.page = null;
  }

  async setup() {
    this.browser = await launchBrowser();
    this.page = await this.browser.newPage();
    await this.page.setViewport({ width: 1440, height: 900 });
    await this.page.goto(this.base, { waitUntil: "networkidle0", timeout: 30000 });
  }

  async teardown() {
    if (this.browser) await this.browser.close();
  }

  record(name, passed, detail = "") {
    this.results.push({ name, passed, detail });
    console.log(
      passed
        ? chalk.green(`  ✅ ${name}`)
        : chalk.red(`  ❌ ${name}${detail ? ` — ${detail}` : ""}`),
    );
  }

  async test(name, fn) {
    try {
      const detail = await fn();
      this.record(name, true, typeof detail === "string" ? detail : "");
    } catch (error) {
      this.record(name, false, error.message);
    }
  }

  /** Fetch a value from inside the page. */
  evaluate(pageFunction) {
    return this.page.evaluate(pageFunction);
  }

  figure(label) {
    return `[role="img"][aria-label="${label}"]`;
  }

  async run() {
    const page = this.page;

    await this.test("session ships rendered assets and the content_render capability", async () => {
      const session = await (await fetchJson(`${this.base}/api/v1/session`)).body;
      if (!session.capabilities.includes("content_render")) {
        throw new Error("content_render missing from capabilities");
      }
      const assets = session.rich ?? {};
      const kinds = Object.values(assets).map((a) => a.kind);
      const diagrams = kinds.filter((k) => k === "diagram").length;
      if (diagrams < 3) throw new Error(`expected ≥3 diagram assets, got ${diagrams}`);
      if (!kinds.includes("svg") || !kinds.includes("html")) {
        throw new Error(`svg/html assets missing: ${kinds.join(",")}`);
      }
      for (const asset of Object.values(assets)) {
        if (asset.kind === "diagram" && !(asset.light?.body && asset.dark?.body)) {
          throw new Error("diagram asset missing a theme variant");
        }
      }
      return `${assets.length} assets`;
    });

    await this.test("radio options render labels, descriptions and figure thumbnails", async () => {
      await page.waitForSelector(this.figure("Mesh"), { visible: true, timeout: 10000 });
      await page.waitForSelector(this.figure("Star"), { visible: true, timeout: 5000 });
      const text = await page.evaluate(() => document.body.innerText);
      if (!text.includes("Every node talks to every other node")) {
        throw new Error("option description not rendered");
      }
      if (!text.includes("single point of failure")) {
        throw new Error("second option description not rendered");
      }
      // Ring declares no figure: exactly two thumbnails in the radio group.
      const thumbs = await page.$$eval(
        `${this.figure("Mesh")}, ${this.figure("Star")}`,
        (els) => els.length,
      );
      if (thumbs !== 2) throw new Error(`expected 2 thumbnails, found ${thumbs}`);
      // The third option shows label only.
      if (!text.includes("Ring")) throw new Error("plain option label missing");
    });

    await this.test("figures cover section headers, nullable options and new diagram families", async () => {
      // Section-level x-content: the section header carries its own figure.
      const sectionFigure = await page.$(this.figure("More shapes a figure can take"));
      if (!sectionFigure) throw new Error("section-level figure missing");
      // Nullable enum: figures on the two real options, none on the null one.
      await page.waitForSelector(this.figure("Active–passive"), { timeout: 5000 });
      await page.waitForSelector(this.figure("Active–active"), { timeout: 5000 });
      const notConfigured = await page.$(this.figure("Not configured"));
      if (notConfigured) throw new Error("null option should not carry a figure");
      // Sequence + pie diagrams arrived as rendered assets in the session.
      const session = await (await fetchJson(`${this.base}/api/v1/session`)).body;
      const bodies = Object.values(session.rich ?? {})
        .filter((a) => a.kind === "diagram")
        .map((a) => a.light.body + a.dark.body);
      const sequence = bodies.some((b) => b.includes("Developer") && b.includes("production"));
      const pie = bodies.some((b) => /45|compute/.test(b));
      if (!sequence) throw new Error("sequence diagram asset missing");
      if (!pie) throw new Error("pie chart asset missing");
    });

    await this.test("select shows the x-options label override", async () => {
      const label = await page.evaluate(() => {
        const trigger = document.querySelector("[data-radix-popper-content-wrapper]")
          ? null
          : document.querySelector("button[role='combobox'], [role='combobox']");
        return trigger ? trigger.textContent : null;
      });
      if (!label || !label.includes("Amazon S3")) {
        throw new Error(`combobox shows ${JSON.stringify(label)}`);
      }
    });

    await this.test("markdown block renders sanitised structure (table, strikethrough)", async () => {
      const probe = await page.evaluate(() => ({
        table: Boolean(document.querySelector("table")),
        del: Boolean(document.querySelector("del")),
        headings: document.querySelectorAll("h3").length,
      }));
      if (!probe.table) throw new Error("markdown table missing");
      if (!probe.del) throw new Error("strikethrough missing");
      if (probe.headings < 1) throw new Error("markdown heading missing");
    });

    await this.test("hovering a thumbnail enlarges it within the viewport", async () => {
      const mesh = await page.$(this.figure("Mesh"));
      await mesh.scrollIntoView();
      await page.evaluate(() => new Promise((r) => requestAnimationFrame(r)));
      await page.hover(this.figure("Mesh"));
      // SHOW_DELAY_MS on the frontend is 180ms; give the transition room.
      await new Promise((resolve) => setTimeout(resolve, 700));
      const layer = await page.evaluate(() => {
        const portal = Array.from(document.body.children).find((el) =>
          String(el.className).includes("pointer-events-none"),
        );
        if (!portal) return null;
        const r = portal.getBoundingClientRect();
        return {
          hasSvg: Boolean(portal.querySelector("svg")),
          inViewport:
            r.x >= 0 && r.y >= 0 && r.right <= window.innerWidth && r.bottom <= window.innerHeight,
        };
      });
      if (!layer) throw new Error("hover preview layer did not appear");
      if (!layer.hasSvg) throw new Error("hover preview has no svg");
      if (!layer.inViewport) throw new Error("hover preview escaped the viewport");
      await page.mouse.move(10, 10);
    });

    await this.test("clicking opens the fullscreen viewer with copy/download; Escape closes", async () => {
      await page.click(this.figure("Mesh"));
      await page.waitForSelector("[role='dialog']", { visible: true, timeout: 5000 });
      const dialogText = await page.$eval("[role='dialog']", (el) => el.textContent);
      if (!dialogText.includes("Copy source")) throw new Error("Copy source button missing");
      if (!dialogText.includes("Download SVG")) throw new Error("Download SVG button missing");
      const figureSvg = await page.$$eval(
        "[role='dialog'] svg",
        (svgs) => svgs.filter((s) => (s.getAttribute("viewBox") ?? "").includes(" ")).length,
      );
      if (figureSvg < 1) throw new Error("figure not rendered in the viewer");
      await page.keyboard.press("Escape");
      await new Promise((resolve) => setTimeout(resolve, 400));
      const closed = await page.$("[role='dialog']");
      if (closed) throw new Error("Escape did not close the viewer");
    });

    await this.test("mermaid editor previews valid source and reports parse errors", async () => {
      const editor = await page.$('textarea[aria-label="Mermaid source"]');
      if (!editor) throw new Error("mermaid editor textarea missing");
      await editor.scrollIntoView();
      // Type something broken: the pane must show the renderer's message.
      await editor.click({ clickCount: 3 });
      await page.type('textarea[aria-label="Mermaid source"]', "flowchart LR; A-->");
      await new Promise((resolve) => setTimeout(resolve, 900));
      const errorShown = await page.evaluate(() => {
        const textareas = document.querySelectorAll("textarea");
        const grid = textareas[textareas.length - 1].closest("div");
        return /unexpected token|invalid|does not parse/i.test(grid.parentElement.textContent ?? "");
      });
      if (!errorShown) throw new Error("parse error not surfaced next to the editor");
      // And something valid: the preview comes back.
      await editor.click({ clickCount: 3 });
      await page.type(
        'textarea[aria-label="Mermaid source"]',
        "flowchart LR\n  A --> B",
      );
      await new Promise((resolve) => setTimeout(resolve, 1200));
      const previewSvg = await page.evaluate(() => {
        const preview = document.querySelector('[data-testid="mermaid-preview"]');
        return preview ? preview.querySelectorAll("svg").length : 0;
      });
      if (previewSvg < 1) throw new Error("live preview did not render the corrected source");
    });

    await this.test("editor pane holds its height and the last good diagram while typing", async () => {
      const preview = await page.$('[data-testid="mermaid-preview"]');
      if (!preview) throw new Error("preview pane missing");
      const before = await preview.boundingBox();
      // Mid-debounce: the previous diagram must still be on screen, and the
      // pane must not have moved — this is the "every keystroke collapses
      // the layout" regression.
      await page.type('textarea[aria-label="Mermaid source"]', "  # typing");
      const mid = await page.evaluate(() => {
        const pane = document.querySelector('[data-testid="mermaid-preview"]');
        return { svg: Boolean(pane.querySelector("svg")), height: pane.getBoundingClientRect().height };
      });
      if (!mid.svg) throw new Error("previous diagram vanished while re-rendering");
      if (Math.abs(mid.height - before.height) > 1) {
        throw new Error(`pane height jumped ${before.height} → ${mid.height}`);
      }
      await new Promise((resolve) => setTimeout(resolve, 900));
    });

    await this.test("editor layout switches between split, stacked and swapped", async () => {
      const clickLayout = (label) =>
        page.evaluate((name) => {
          const button = Array.from(document.querySelectorAll("button")).find(
            (b) => b.getAttribute("aria-label") === name,
          );
          if (!button) return false;
          button.click();
          return true;
        }, label);

      const orientation = () =>
        page.evaluate(() => {
          const preview = document.querySelector('[data-testid="mermaid-preview"]');
          const grid = preview?.closest(".grid");
          if (!grid) return null;
          const [first, second] = Array.from(grid.children);
          const col =
            first.getBoundingClientRect().y !== second.getBoundingClientRect().y;
          // `order` moves boxes visually without touching DOM order, so the
          // arrangement must be read off geometry: who sits left?
          const previewLeft = preview.getBoundingClientRect().x;
          const editorLeft = (
            grid.querySelector("textarea") ?? first
          ).getBoundingClientRect().x;
          return { stacked: col, swapped: !col && previewLeft < editorLeft };
        });

      if (!(await clickLayout("Editor above, preview below"))) throw new Error("stacked button missing");
      await new Promise((resolve) => setTimeout(resolve, 150));
      let state = await orientation();
      if (!state?.stacked) throw new Error("stacked layout did not stack the panes");

      if (!(await clickLayout("Editor left, preview right"))) throw new Error("split button missing");
      await new Promise((resolve) => setTimeout(resolve, 150));
      state = await orientation();
      if (state?.stacked || state?.swapped) throw new Error("split layout wrong");

      if (!(await clickLayout("Preview left, editor right"))) throw new Error("swap button missing");
      await new Promise((resolve) => setTimeout(resolve, 150));
      state = await orientation();
      if (state?.stacked || !state?.swapped) throw new Error("swapped layout wrong");
      // Back to the default arrangement.
      await clickLayout("Editor left, preview right");
    });

    await this.test("select options carry their own diagrams", async () => {
      // The reference-architecture select: open it, its popup must contain
      // one small figure per option that declared content.
      const opened = await page.evaluate(() => {
        const region = Array.from(document.querySelectorAll("section, [data-slot]"))
          .find((el) => el.textContent?.includes("Reference architecture"));
        if (!region) return false;
        // The combobox under the Reference architecture heading.
        const buttons = Array.from(region.querySelectorAll("button[role='combobox'], [role='combobox']"));
        buttons[0]?.click();
        return buttons.length > 0;
      });
      if (!opened) throw new Error("reference architecture select not found");
      await page.waitForSelector("[role='listbox']", { visible: true, timeout: 5000 });
      const figures = await page.$$eval(
        "[role='listbox'] [role='option'] svg",
        (svgs) => svgs.length,
      );
      if (figures < 3) throw new Error(`expected ≥3 option figures, found ${figures}`);
      await page.keyboard.press("Escape");
    });

    await this.test("the fullscreen viewer fits the figure to the stage", async () => {
      await page.click(this.figure("Mesh"));
      await page.waitForSelector("[role='dialog']", { visible: true, timeout: 5000 });
      const fit = await page.evaluate(() => {
        const dialog = document.querySelector("[role='dialog']");
        const dialogRect = dialog.getBoundingClientRect();
        // The figure svg: the largest svg inside the stage (lucide icons are small).
        const svgs = Array.from(dialog.querySelectorAll("svg"));
        const figure = svgs.reduce((best, s) => {
          const r = s.getBoundingClientRect();
          return r.width * r.height > best.width * best.height ? r : best;
        }, { width: 0, height: 0 });
        return {
          figureWidthRatio: figure.width / dialogRect.width,
          dialogWidth: dialogRect.width,
          dialogHeight: dialogRect.height,
          viewportFill: dialogRect.width / window.innerWidth,
        };
      });
      if (fit.figureWidthRatio < 0.55) {
        throw new Error(
          `figure fills only ${(fit.figureWidthRatio * 100).toFixed(0)}% of the dialog width`,
        );
      }
      if (fit.dialogWidth < 700) throw new Error(`viewer dialog too small: ${fit.dialogWidth}px`);
      if (fit.viewportFill < 0.6) throw new Error("viewer dialog does not use the screen");
      await page.keyboard.press("Escape");
      await new Promise((resolve) => setTimeout(resolve, 400));
    });

    await this.test("node-level figures (block) hover and open the viewer like options do", async () => {
      const block = await page.$(this.figure("Mermaid figure"));
      await block.scrollIntoView();
      await page.hover(this.figure("Mermaid figure"));
      await new Promise((resolve) => setTimeout(resolve, 700));
      const layer = await page.evaluate(() => {
        const portal = Array.from(document.body.children).find((el) =>
          String(el.className).includes("pointer-events-none"),
        );
        return portal ? Boolean(portal.querySelector("svg")) : false;
      });
      if (!layer) throw new Error("hover preview did not appear for a block figure");
      await page.click(this.figure("Mermaid figure"));
      await page.waitForSelector("[role='dialog']", { visible: true, timeout: 5000 });
      await page.keyboard.press("Escape");
      await new Promise((resolve) => setTimeout(resolve, 400));
      const closed = await page.$("[role='dialog']");
      if (closed) throw new Error("viewer did not close for a block figure");
    });

    await this.test("switching the theme swaps the diagram variant", async () => {
      const fingerprint = () =>
        page.evaluate(() => {
          const fig = Array.from(document.querySelectorAll('[role="img"]')).find(
            (el) => el.getAttribute("aria-label") === "Mermaid figure",
          );
          const node = fig?.querySelector("rect, path, circle");
          return node?.getAttribute("fill") ?? null;
        });
      const before = await fingerprint();
      const toggled = await page.evaluate(() => {
        const button = Array.from(document.querySelectorAll("button")).find((b) =>
          /^(Dark|Light)$/.test(b.textContent?.trim() ?? ""),
        );
        if (!button) return false;
        button.click();
        return true;
      });
      if (!toggled) throw new Error("theme toggle button not found");
      await new Promise((resolve) => setTimeout(resolve, 400));
      const after = await fingerprint();
      if (!before || !after) throw new Error("figure svg node not found");
      if (before === after) {
        throw new Error(`diagram did not switch variants (${before} → ${after})`);
      }
      // Toggle back for a clean teardown state.
      await page.evaluate(() => {
        const button = Array.from(document.querySelectorAll("button")).find((b) =>
          /^(Dark|Light)$/.test(b.textContent?.trim() ?? ""),
        );
        button?.click();
      });
    });
  }
}

/**
 * Launch a browser for the suite. Prefers the installed Google Chrome
 * (`channel: "chrome"`); the puppeteer-downloaded Chrome for Testing fails to
 * start on some macOS setups, and system Chrome is the pragmatic default.
 */
async function launchBrowser() {
  const common = {
    headless: HEADLESS ? "new" : false,
    args: ["--no-sandbox", "--disable-setuid-sandbox", "--disable-gpu"],
  };
  try {
    return await puppeteer.launch({ ...common, channel: "chrome" });
  } catch (error) {
    console.log(chalk.yellow("system Chrome unavailable, using bundled Chromium"));
    return puppeteer.launch(common);
  }
}

async function main() {
  console.log(chalk.blue("🧪 Rich Content E2E Suite"));
  const { server, base } = await startServer();  const suite = new RichContentE2E(base);
  try {
    await suite.setup();
    await suite.run();
  } catch (error) {
    suite.record("suite setup", false, error.message);
  } finally {
    await suite.teardown();
    server.kill("SIGKILL");
  }

  const passed = suite.results.filter((r) => r.passed).length;
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
