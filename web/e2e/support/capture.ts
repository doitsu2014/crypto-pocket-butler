import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { Locator, Page, TestInfo } from "@playwright/test";

/**
 * Screenshots that are part of the evidence, not just failure debris.
 *
 * Each one is attached to the HTML report (open it and the image is inline with
 * the step that produced it) and written to `e2e/screenshots/<journey>/` so a
 * reviewer can page through a run without opening the report at all.
 */
const HERE = path.dirname(fileURLToPath(import.meta.url));
const SCREENSHOT_DIR = path.resolve(HERE, "..", "screenshots");

export interface CaptureOptions {
  /** Elements whose content is genuinely unstable (a clock, an id) and should be blanked. */
  mask?: Locator[];
  /** Capture the viewport instead of the whole page. */
  viewportOnly?: boolean;
}

function slug(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/**
 * Waits for the page to settle (fonts loaded, no in-flight animation), then
 * records the screenshot under the given name.
 */
export async function capture(
  page: Page,
  testInfo: TestInfo,
  name: string,
  options: CaptureOptions = {},
): Promise<void> {
  await page.waitForLoadState("networkidle", { timeout: 3_000 }).catch(() => {
    // A long-polling request or the dev server's HMR socket must not fail the
    // journey; the assertions already decided the page is ready.
  });
  // Await inside the page: returning the FontFaceSet itself does not serialize.
  await page.evaluate(async () => {
    await document.fonts.ready;
  });

  const png = await page.screenshot({
    fullPage: !options.viewportOnly,
    animations: "disabled",
    caret: "hide",
    mask: options.mask,
  });

  await testInfo.attach(`${name}.png`, {
    body: png,
    contentType: "image/png",
  });

  const journey =
    slug(`${path.basename(testInfo.file).replace(/\.spec\.ts$/, "")}-${testInfo.title}`) ||
    "journey";
  const folder = path.join(SCREENSHOT_DIR, journey);
  await mkdir(folder, { recursive: true });
  await writeFile(path.join(folder, `${name}.png`), png);
}
