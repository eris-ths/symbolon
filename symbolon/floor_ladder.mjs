// 共通床の速さ実験(JS 版)—— floor_ladder.rs の「畳み」(段1)と同じ変形を V8 の上で測る。
//
// 狙い: 「畳み(op→数値 opcode / register→スロット添字)の勝ちは *Rust の勝ち* ではなく
// *床の表現の勝ち* だ」を、host 言語を替えて二点で確かめる。契約(14 op)も値モデル
// (number | ["⟨N⟩"] | ["⟨P⟩",a,d])も machine.json / cases.json も一切変えていない。
//
// 正直な線引き: naive 側(runFloor)は head-to-head のため **本体側の naive な床から写した**
// (SOT はそちら。この木には持ってきていない)。
// 写しの腐りは毎回 cases.json で両床を突き合わせて検出する(食い違えば exit 1)。
//
//   node floor_ladder.mjs        # lab/ で(machine.json / cases.json / bench_host.json を読む)

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const machine = JSON.parse(readFileSync(join(here, "machine.json"), "utf8"));
const cases = JSON.parse(readFileSync(join(here, "cases.json"), "utf8"));

// ---- 値モデル(本体側の床と同一)-------------------------------------------
const isPair = (v) => Array.isArray(v) && v.length === 3 && v[0] === "⟨P⟩";
const NIL = () => ["⟨N⟩"];
function deepEq(a, b) {
  if (typeof a === "number" && typeof b === "number") return a === b;
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) if (!deepEq(a[i], b[i])) return false;
    return true;
  }
  return a === b;
}
const truthy = (v) => v !== 0;

// ==== 床 A: naive(本体側の床から写し。文字列 switch + 文字列キー host)==========
function runNaive(machineAst, host, fuel) {
  let respawn = true;
  function ev(n) {
    switch (n[0]) {
      case "lit": return n[1];
      case "var": return host[n[1]];
      case "add": return ev(n[1]) + ev(n[2]);
      case "sub": return ev(n[1]) - ev(n[2]);
      case "eq": return deepEq(ev(n[1]), ev(n[2])) ? 1 : 0;
      case "nil": return NIL();
      case "cons": return ["⟨P⟩", ev(n[1]), ev(n[2])];
      case "car": { const v = ev(n[1]); if (!isPair(v)) throw new Error("car を pair でない値に"); return v[1]; }
      case "cdr": { const v = ev(n[1]); if (!isPair(v)) throw new Error("cdr を pair でない値に"); return v[2]; }
      case "pair": return isPair(ev(n[1])) ? 1 : 0;
      default: throw new Error(`床A: 式にできない ${n[0]}`);
    }
  }
  function ex(n) {
    switch (n[0]) {
      case "seq": for (const s of n[1]) ex(s); break;
      case "set": case "let": host[n[1]] = ev(n[2]); break;
      case "if": truthy(ev(n[1])) ? ex(n[2]) : ex(n[3]); break;
      case "respawn": {
        const guard = n[1];
        if (guard !== null && !truthy(host[guard])) return;
        if (fuel <= 0) return;
        fuel -= 1; respawn = true; break;
      }
      default: ev(n);
    }
  }
  respawn = true;
  while (respawn) { respawn = false; ex(machineAst); }
  return host["vs"][1];
}

// ==== 床 B: 畳んだ床(数値 opcode + スロット添字。走行中に文字列を触らない)=======
const OP = { LIT:0, VAR:1, ADD:2, SUB:3, EQ:4, NIL:5, CONS:6, CAR:7, CDR:8, PAIR:9,
             SEQ:10, SET:11, IF:12, RESPAWN:13 };

/** 機械 AST を一度だけ畳む: 木 → 単型オブジェクトの配列 + 添字、名前 → スロット。 */
function fold(machineAst) {
  const nodes = [];
  const slot = new Map();
  const intern = (s) => { let i = slot.get(s); if (i === undefined) { i = slot.size; slot.set(s, i); } return i; };
  const push = (o, x, y, z, k, list) => { nodes.push({ o, x, y, z, k, list }); return nodes.length - 1; };

  function build(n) {
    switch (n[0]) {
      case "lit": return push(OP.LIT, 0, 0, 0, n[1], null);
      case "var": return push(OP.VAR, intern(n[1]), 0, 0, 0, null);
      case "nil": return push(OP.NIL, 0, 0, 0, 0, null);
      case "add": { const x = build(n[1]), y = build(n[2]); return push(OP.ADD, x, y, 0, 0, null); }
      case "sub": { const x = build(n[1]), y = build(n[2]); return push(OP.SUB, x, y, 0, 0, null); }
      case "eq":  { const x = build(n[1]), y = build(n[2]); return push(OP.EQ,  x, y, 0, 0, null); }
      case "cons":{ const x = build(n[1]), y = build(n[2]); return push(OP.CONS,x, y, 0, 0, null); }
      case "car": return push(OP.CAR,  build(n[1]), 0, 0, 0, null);
      case "cdr": return push(OP.CDR,  build(n[1]), 0, 0, 0, null);
      case "pair":return push(OP.PAIR, build(n[1]), 0, 0, 0, null);
      case "seq": { const list = n[1].map(build); return push(OP.SEQ, 0, 0, 0, 0, list); }
      case "set": case "let": { const s = intern(n[1]); const e = build(n[2]); return push(OP.SET, s, e, 0, 0, null); }
      case "if":  { const c = build(n[1]), t = build(n[2]), f = build(n[3]); return push(OP.IF, c, t, f, 0, null); }
      case "respawn": return push(OP.RESPAWN, n[1] === null ? -1 : intern(n[1]), 0, 0, 0, null);
      default: throw new Error(`床B: 未知 op ${n[0]}`);
    }
  }
  const root = build(machineAst);
  return { nodes, root, slot, vs: intern("vs") };
}

function runFolded(prog, regs, fuel) {
  const { nodes, root, vs } = prog;
  let respawn = true;
  function ev(i) {
    const n = nodes[i];
    switch (n.o) {
      case OP.LIT: return n.k;
      case OP.VAR: return regs[n.x];
      case OP.ADD: return ev(n.x) + ev(n.y);
      case OP.SUB: return ev(n.x) - ev(n.y);
      case OP.EQ: return deepEq(ev(n.x), ev(n.y)) ? 1 : 0;
      case OP.NIL: return NIL();
      case OP.CONS: return ["⟨P⟩", ev(n.x), ev(n.y)];
      case OP.CAR: { const v = ev(n.x); if (!isPair(v)) throw new Error("car を pair でない値に"); return v[1]; }
      case OP.CDR: { const v = ev(n.x); if (!isPair(v)) throw new Error("cdr を pair でない値に"); return v[2]; }
      case OP.PAIR: return isPair(ev(n.x)) ? 1 : 0;
      default: throw new Error("床B: 式にできない節");
    }
  }
  function ex(i) {
    const n = nodes[i];
    switch (n.o) {
      case OP.SEQ: { const l = n.list; for (let j = 0; j < l.length; j++) ex(l[j]); break; }
      case OP.SET: regs[n.x] = ev(n.y); break;
      case OP.IF: truthy(ev(n.x)) ? ex(n.y) : ex(n.z); break;
      case OP.RESPAWN: {
        if (n.x >= 0 && !truthy(regs[n.x])) return;
        if (fuel <= 0) return;
        fuel -= 1; respawn = true; break;
      }
      default: ev(i);
    }
  }
  respawn = true;
  while (respawn) { respawn = false; ex(root); }
  return regs[vs][1];
}

/** host(名前→値)を スロット配列へ。機械 AST に現れない名前も slot に足す。 */
function hostToRegs(prog, host) {
  for (const k of Object.keys(host)) if (!prog.slot.has(k)) prog.slot.set(k, prog.slot.size);
  const regs = new Array(prog.slot.size).fill(undefined);
  for (const [k, v] of Object.entries(host)) regs[prog.slot.get(k)] = v;
  return regs;
}

// ---- 畳む(実行前に一度だけ)------------------------------------------------
const t0 = process.hrtime.bigint();
const prog = fold(machine);
const foldMs = Number(process.hrtime.bigint() - t0) / 1e6;

console.log("== 畳んだ床(B)vs naive 床(A)—— 同じ 14-op 契約・同じ machine.json / JS(V8)==\n");
console.log(`  畳み: 節 ${prog.nodes.length} 個 / register ${prog.slot.size} スロット / ${foldMs.toFixed(2)} ms(実行前に一度だけ)\n`);

let pass = 0, bad = 0;
for (const c of cases) {
  const ra = runNaive(machine, { ...c.host }, 1000000);
  const rb = runFolded(prog, hostToRegs(prog, { ...c.host }), 1000000);
  const ab = deepEq(ra, rb);
  const ok = deepEq(rb, c.expect);
  const mark = !ab ? "✗ 床A/B 不一致" : ok ? "✓ 一致" : "⚡ 発散(意図的)";
  const s = (v) => (typeof v === "number" ? String(v) : isPair(v) ? "pair" : "nil");
  console.log(`  ${mark}  ${c.name.padEnd(14)} A=${s(ra).padStart(6)} B=${s(rb).padStart(6)} py=${s(c.expect).padStart(6)}  — ${c.note}`);
  if (!ab) bad++; else if (ok) pass++;
}
console.log(`\n  py 期待値と一致 ${pass} / 床A・床B の不一致 ${bad}(不一致 0 = 畳んでも意味は変わっていない)`);

// ---- 速度: sum 1..N を同一プロセスで head-to-head(best-of-5)----------------
const b = JSON.parse(readFileSync(join(here, "bench_host.json"), "utf8"));
let ta = Infinity, tb = Infinity, ra, rb;
for (let i = 0; i < 5; i++) {
  let t = process.hrtime.bigint();
  ra = runNaive(machine, { ...b.host }, 100000000);
  ta = Math.min(ta, Number(process.hrtime.bigint() - t) / 1e6);
  t = process.hrtime.bigint();
  rb = runFolded(prog, hostToRegs(prog, { ...b.host }), 100000000);
  tb = Math.min(tb, Number(process.hrtime.bigint() - t) / 1e6);
}
const ok = deepEq(ra, rb) && deepEq(rb, b.expect);
console.log(`\n速度(sum 1..${b.n} = ${JSON.stringify(rb)}, A=B=py ${ok}) best-of-5:`);
console.log(`  床A naive (文字列 switch + 文字列キー host) : ${ta.toFixed(1).padStart(7)} ms`);
console.log(`  床B 畳み  (数値 opcode + スロット添字)      : ${tb.toFixed(1).padStart(7)} ms   → ${(ta / tb).toFixed(1)}x`);
console.log(`  ※ 畳みの前処理 ${foldMs.toFixed(2)} ms は一度きり。`);
process.exit(ok && bad === 0 ? 0 : 1);
