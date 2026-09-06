// 段G′ の秤 —— 吐いた wasm を実際に走らせ、Python 床の期待値と突き合わせて時間を出す。
//   node probe_run.mjs <wasm> <expect>
import { readFileSync } from "node:fs";
const [, , file, want] = process.argv;
const bin = readFileSync(file);
const { instance } = await WebAssembly.instantiate(bin, {});
// ⚠️ 先に温める —— この規模だと JIT の段(tier)が測定を支配し、
//    温める前後で per-call が 2x 動く(線形性チェックが鳴って気づいた)。
let warm = 0n;
for (let i = 0; i < 50000; i++) warm += instance.exports.run();

let r, best = Infinity;
for (let k = 0; k < 5; k++) {
  const t0 = process.hrtime.bigint();
  for (let i = 0; i < 200; i++) r = instance.exports.run();
  best = Math.min(best, Number(process.hrtime.bigint() - t0) / 1e6 / 200);
}
// 「速すぎる」時は消えている疑いを先に潰す(呼び数 10x で時間も伸びるか)
let acc = 0n;
const t0 = process.hrtime.bigint();
for (let i = 0; i < 2000; i++) acc += instance.exports.run();
const scaled = Number(process.hrtime.bigint() - t0) / 1e6 / (best * 200);
// 段G″: heap の峰(peak)は module が memory[0] に置いている(回収の有無が外から見える)
let peak = "—", cells = "—";
if (instance.exports.memory) {
  const m = new BigInt64Array(instance.exports.memory.buffer);
  peak = Number(m[0]); cells = peak ? (peak - 16) / 16 : 0;
}
const ok = String(r) === String(want);
// ⚠️ 錨は **符牒**であって文言ではない —— 門をこの一文に錨づけると、言い方を正した日に静かに外れる
//    （2026-09-05 に一度、2026-09-06 に `床D が実際に回した命令` でもう一度踏んだ）。
console.log(`  段G′ 実走: ${r} (期待 ${want}) ${ok ? "✓ 一致 [RUN ok]" : "✗ 不一致 [RUN bad]"} / ${best.toFixed(5)} ms / wasm ${bin.length} B(温めた後の best-of-5)`);
console.log(`        線形性: 呼び 10x で時間 ${scaled.toFixed(1)}x ⇒ ${scaled > 5 ? "本当に走っている" : "⚠️ 消えている疑い"}`);
if (peak !== "—") console.log(`        heap の峰: ${peak} B = **${cells} セル**`);
process.exit(ok ? 0 : 1);
