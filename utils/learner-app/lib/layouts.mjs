// The layouts the renderer draws (tools/render/layouts.js), read in Node.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

export const RENDER_DIR = fileURLToPath(new URL("../../../tools/render/", import.meta.url));

export function loadLayouts(path = RENDER_DIR + "layouts.js") {
  const src = readFileSync(path, "utf8");
  const m = src.match(/window\.BADA_LAYOUTS\s*=\s*(\[.*\]);?\s*$/s);
  if (!m) throw new Error(`no BADA_LAYOUTS in ${path}`);
  // Installed layouts only: 300–999 are design drafts (spec §5.1).
  return JSON.parse(m[1]).filter(l => l.id < 300);
}
