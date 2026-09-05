// 段G の秤 —— 吐いた wasm と自己完結 HTML が、Python 床と同じ答えを出すか。
//
// 二段で確かめる: (1) node で wasm を直に instantiate、(2) **実 Chromium** で HTML を開く。
// 見た目を信じない(#5 の「黄色バグ」の教訓)—— 頁が出したテキストを読んで突き合わせる。
//
//   node web_verify.mjs        # symbolon/ で。compiled.wasm / demo-compiled.html を読む
//   (先に: rustc -O floor_ladder.rs -o /tmp/fl && /tmp/fl --web)

import { readFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const bench = JSON.parse(readFileSync(join(here, "bench_host.json"), "utf8"));
const expect = BigInt(bench.expect);
const N = bench.n;
let bad = 0;

// ---- (1) node で wasm を直に -------------------------------------------------
const bin = readFileSync(join(here, "compiled.wasm"));
const { instance } = await WebAssembly.instantiate(bin, {});
let r;
let best = Infinity;
for (let k = 0; k < 5; k++) {
  const t0 = process.hrtime.bigint();
  for (let i = 0; i < 1000; i++) r = instance.exports.run();
  best = Math.min(best, Number(process.hrtime.bigint() - t0) / 1e6 / 1000);
}
// ⚠️ 「速すぎる」時は消えている疑いを先に潰す —— 呼び数を 10x にして時間も 10x になるか。
let acc = 0n, t10 = 0;
{
  const t0 = process.hrtime.bigint();
  for (let i = 0; i < 10000; i++) acc += instance.exports.run();
  t10 = Number(process.hrtime.bigint() - t0) / 1e6;
}
const scaled = t10 / (best * 1000);
console.log(`         線形性: 呼び 10x で時間 ${scaled.toFixed(1)}x(acc=${acc})⇒ ${scaled > 5 ? "本当に走っている" : "⚠️ 消えている疑い"}`);

const nodeOk = r === expect;
if (!nodeOk) bad++;
console.log(`node   : run() = ${r} (期待 ${expect}) ${nodeOk ? "✓ 一致" : "✗ 不一致"}`);
console.log(`         wasm ${bin.length} B / ${best.toFixed(4)} ms = ${(best * 1e6 / N).toFixed(1)} ns/反復`);

// ---- (2) 実 Chromium で HTML を開く ------------------------------------------
const htmlPath = join(here, "demo-compiled.html");
// ESM の import は NODE_PATH を見ない ⇒ createRequire で解決してから読む。
let pw = null;
try {
  const { createRequire } = await import("node:module");
  const req = createRequire(import.meta.url);
  const paths = ["playwright", "/opt/node22/lib/node_modules/playwright"];
  for (const p of paths) { try { pw = req(p); break; } catch { /* 次を試す */ } }
} catch { /* 無ければ skip */ }
if (!pw) {
  console.log("chromium: playwright が無い —— skip(node 側だけで検証)");
} else if (!existsSync(htmlPath)) {
  console.log("chromium: HTML が無い —— skip");
} else {
  const browser = await pw.chromium.launch();
  const page = await browser.newPage();
  const errs = [];
  page.on("pageerror", (e) => errs.push(String(e)));
  page.on("console", (m) => { if (m.type() === "error") errs.push(m.text()); });
  await page.goto("file://" + resolve(htmlPath));
  await page.waitForFunction(() => document.getElementById("out").textContent !== "…", null, { timeout: 15000 });
  const out = await page.textContent("#out");
  const ok = await page.textContent("#ok");
  const sz = await page.textContent("#sz");
  await browser.close();
  const browserOk = out.trim() === String(expect) && errs.length === 0;
  if (!browserOk) bad++;
  console.log(`chromium: 頁が出した値 = ${out.trim()} / ${sz.trim()} / ${ok.trim()} ${browserOk ? "✓ 一致" : "✗ 不一致"}`);
  if (errs.length) console.log("  console error:", errs.slice(0, 3));
}

console.log(bad === 0 ? "\n秤 ✓ —— node / 実ブラウザ とも Python 床と同じ答え。" : "\n秤 ✗");
process.exit(bad === 0 ? 0 : 1);
