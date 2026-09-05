// 段H の秤 —— host に駆動される wasm が、**kokkos の機械が出した真値の軌跡**と一致するか。
//
// (1) node: step() を軌跡どおり呼び、**毎歩** total / wraps を突き合わせる(最終値だけ見ない)。
// (2) 実 Chromium: 本物の DOM クリックで同じ event 列を送り、頁が描いた文字を読む。
//     ⚠️ 見た目を信じない —— 頁が出した値を読んで真値と比べる(#5「黄色バグ」の教訓)。
//
//   node engine_verify.mjs      # lab/ で(先に engine_emit.py と floor_ladder --engine)

import { readFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const { evs, trace, final } = JSON.parse(readFileSync(join(here, "engine_trace.json"), "utf8"));
let bad = 0;

// ---- (1) node で毎歩突き合わせる ---------------------------------------------
const bin = readFileSync(join(here, "engine.wasm"));
const { instance } = await WebAssembly.instantiate(bin, {});
const mem = new BigInt64Array(instance.exports.memory.buffer);
let mismatch = null;
for (let i = 0; i < evs.length; i++) {
  instance.exports.step(BigInt(evs[i]));
  const got = { total: Number(mem[0]), wraps: Number(mem[1]) };
  const want = trace[i];
  if (got.total !== want.total || got.wraps !== want.wraps) {
    mismatch = { i, ev: evs[i], got, want }; break;
  }
}
if (mismatch) { bad++; console.log("node    : ✗ 不一致", JSON.stringify(mismatch)); }
else console.log(`node    : ${evs.length} 歩すべて真値と一致 ✓（最終 total=${final.total} wraps=${final.wraps}, wasm ${bin.length} B）`);

// ---- (2) 実 Chromium を本物のクリックで駆動 -----------------------------------
const htmlPath = join(here, "demo-engine.html");
let pw = null;
try {
  const { createRequire } = await import("node:module");
  const req = createRequire(import.meta.url);
  for (const p of ["playwright", "/opt/node22/lib/node_modules/playwright"]) {
    try { pw = req(p); break; } catch { /* 次を試す */ }
  }
} catch { /* 無ければ skip */ }

if (!pw || !existsSync(htmlPath)) {
  console.log("chromium: playwright か HTML が無い —— skip");
} else {
  const browser = await pw.chromium.launch();
  const page = await browser.newPage();
  const errs = [];
  page.on("pageerror", (e) => errs.push(String(e)));
  page.on("console", (m) => { if (m.type() === "error") errs.push(m.text()); });
  await page.goto("file://" + resolve(htmlPath));
  await page.waitForFunction(() => typeof window.__send === "function", null, { timeout: 15000 });

  // **本物のクリック** で event を送る(JS を呼ばず、人と同じ経路で)
  let clickBad = null;
  for (let i = 0; i < evs.length; i++) {
    await page.click(evs[i] === 3 ? "#b3" : "#b1");
    const total = Number(await page.textContent("#total"));
    const wraps = Number(await page.textContent("#wraps"));
    if (total !== trace[i].total || wraps !== trace[i].wraps) {
      clickBad = { i, ev: evs[i], got: { total, wraps }, want: trace[i] }; break;
    }
  }
  // host が状態を書き戻せるか(状態の持ち主は host)
  await page.click("#br");
  const afterReset = Number(await page.textContent("#total"));
  const sz = (await page.textContent("#sz")).trim();
  await browser.close();

  const ok = !clickBad && afterReset === 0 && errs.length === 0;
  if (!ok) bad++;
  if (clickBad) console.log("chromium: ✗ 不一致", JSON.stringify(clickBad));
  else console.log(`chromium: ${evs.length} 回の**本物のクリック**すべて真値と一致 ✓（wasm ${sz} / reset 後 total=${afterReset}）`);
  if (errs.length) console.log("  console error:", errs.slice(0, 3));
}

console.log(bad === 0
  ? "\n秤 ✓ —— kokkos の機械 / node / 実ブラウザの DOM、三点が同じ状態列を出した。"
  : "\n秤 ✗");
process.exit(bad === 0 ? 0 : 1);
