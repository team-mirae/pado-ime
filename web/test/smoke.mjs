// Loads the built wasm with the shipped data and types a few words: the engine works in WebAssembly.
//   node web/test/smoke.mjs   (after web/build.mjs, or with the wasm built in web/wasm/target)
import { readFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { Engine, DATA_FILES } from "../src/engine.js";
import assert from "node:assert/strict";

const root = fileURLToPath(new URL("../../", import.meta.url));
const wasm = readFileSync(root + "web/wasm/target/wasm32-unknown-unknown/release/bada_web.wasm");
const engine = await Engine.load(wasm);
let t = performance.now();
DATA_FILES.forEach((f, i) => existsSync(root + "engine/data/" + f) && engine.put(i, readFileSync(root + "engine/data/" + f)));
engine.init();
console.log(`init ${(performance.now() - t).toFixed(0)} ms`);

const type = (keys) => {
  let text = "", last;
  for (const k of keys) {
    const [code, shift] = k.startsWith("+") ? [k.slice(1), true] : [k, false];
    last = engine.key(code, { shift });
    text += last.commit;
  }
  return { text, last };
};
const letters = s => [...s].map(c => c === " " ? "Space" : "Key" + c.toUpperCase());

// 2-set (10): 한글
engine.applySettings({ enabled: [10] });
engine.setMode("henkan");
let r = type(letters("gksrmf"));
assert.equal(r.last.preedit, "한글");
r = type(["Space"]);   // commits, then the app gets Space (spec §6.3)
assert.equal(r.text, "한글");
assert.equal(r.last.consumed, false);
// Japanese romaji (21): kanji conversions in the menu
engine.applySettings({ enabled: [21] });
r = type(letters("nihongo"));
console.log("romaji preedit:", r.last.preedit, "menu:", r.last.menu.slice(0, 5).join(" "));
assert.ok(r.last.menu.includes("日本語"));
engine.flush();
// Muhenkan: keys pass through; ろ types \ and _
engine.setMode("muhenkan");
assert.equal(engine.key("KeyA").consumed, false);
assert.equal(engine.key("IntlRo").commit, "\\");
assert.equal(engine.key("IntlRo", { shift: true }).commit, "_");
// bada roman (15): a romaji shortcut mixed with Korean spelling ranks first (seigong 성공)
engine.applySettings({ enabled: [15] });
engine.setMode("henkan");
r = type(letters("seigong"));
assert.equal(r.last.preedit, "성공");
engine.flush();
// Catalog
const cat = engine.catalog();
assert.ok(cat.layouts.some(l => l.id === 13) && cat.actions.length > 5);
console.log("catalog:", cat.layouts.map(l => l.id).join(","));
console.log("ok");
