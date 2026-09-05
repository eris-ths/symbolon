# symbolon

**A contract written as data. An interpreter folded into the program it runs.**

`symbolon` (Greek σύμβολον — a tally broken in two, matched later to prove it is genuine)
is the shared middle of several small languages. It is three things:

- a **14-instruction contract**, expressed as data rather than prose,
- a **specializer** that collapses an interpreter against one program (first Futamura projection),
- **five back-ends** that emit standalone artifacts — no LLVM, no emscripten, no runtime.

> ## Don't trust this README. Run it.
>
> Every number on this page is re-derived by a gate that reads the number out of the
> document and out of the tool, and refuses to agree unless they match.
>
> ```bash
> cd symbolon
> python3 claims.py       # re-derives every claim; prints how many matched, and how many it could not shoot
> bash promote.sh         # the full gate suite
> ```
>
> A gate that has never failed is not a gate. Ours has — see `MISSES.md`.

---

## Reading marks

These appear throughout. They are load-bearing, not decoration.

| mark | means |
|---|---|
| ◆ | this is the point |
| ▲ | a limit, honestly stated |
| ⚠️ | read this before acting |
| 🔴 | this one bit us |

---

## What was measured

| program | interpreter instructions, before → after folding | ratio |
|---|---|---|
| integer loop (`sum 1..1000`) | 6,103,390 → 15,011 | **407×** |
| closure-heavy (300 non-escaping closures) | 2,350,010 → 5,711 | **411×** |
| cons-heavy (build a 300-cell list, fold it) | 3,967,486 → 9,320 | **426×** |

| artifact | size |
|---|---|
| wasm (integer loop / closure) | **100 B** / **107 B** |
| wasm with a bump heap (cons-heavy) | **226 B** |
| x86-64 machine code, hand-encoded, `mmap` + W^X | **187 B** |
| engine module (state + `step`) | **133 B** |
| self-contained HTML, wasm inlined as base64 | **2,694 B** |

◆ **The win is removing instructions, not lowering the output level.**
Folding the interpreter away removes 407 of every 408 instructions the interpreter would have
taken. Emitting native code afterwards runs *the same* instruction sequence faster — a
wall-clock difference we deliberately do not pin to a number here, because wall-clock moves
between runs and a number in a README cannot. Order matters: fold first, emit native last.

🔴 We wrote a fixed ratio here on the first draft. The gate rejected it: the document said one
thing, the tool said another, and both were "right" — it is a time, and times move. The gate
enforced a rule we had written and then broken ourselves.

▲ These are deterministic counts, not wall-clock. Wall-clock moves between runs and we
refuse to compare across them — see the eight articles in `COMMON.md` §1.

---

## Layout

```
machine.json   the contract: an interpreter for the object language,
               written in the 14 instructions, as data
cases.json     the acceptance conditions, as data
contract.lock  version + fingerprints. Change the contract → raise the version

floor_ladder.rs   four interpreter floors, the specializer, and the back-ends
floor_ladder.mjs  the same fold, in a second runtime, as a cross-check

promote.sh     seven gates. Skips are never read as passes
claims.py      every numeric claim, re-derived from the tool that produced it
icount.sh      deterministic instruction counts from a sister runtime
```

## Running it

```bash
cd symbolon
rustc -O floor_ladder.rs -o /tmp/fl

/tmp/fl                              # the ladder, with differential tests
/tmp/fl --probe probe_clos.json      # closure-heavy
/tmp/fl --web                        # wasm + a self-contained HTML page
/tmp/fl --engine                     # state in linear memory, driven by events
```

⚠️ Node and a browser are needed for two of the gates (`web_verify.mjs`, `engine_verify.mjs`);
they drive a real browser and click a real page, not a simulated one.

---

## ▲ What this does not do

- **No strings.** Integers and lists of integers only. Adding strings is the next rung.
- **No calls out.** The emitted module exports; it imports nothing. It cannot reach a host.
- **No surface syntax.** Programs arrive encoded. A front-end is deliberately out of scope.
- **Not the fastest thing available.** It is the smallest thing that can be checked end to end.

---

## ⚠️ This repository is a shadow

It is **generated** from a private working repository, every time, in full. That is why every
commit here is authored by `umbra`.

- **Do not send pull requests against these files** — the next projection overwrites them.
- Only files named in a manifest on the other side appear here. Anything not named stays home.
- Issues and discussion are welcome; they are read by the people on the other side.

`COMMON.md` is the working notebook — the chronicle, in Japanese, published as-is rather than
translated. A translation is one more thing that can silently drift, and we would not be able to
gate it. `DESIGN.md` is its normative counterpart: requirements read backwards out of what was
actually measured.

▲ The runnable material lives in `symbolon/`; this page is a copy placed at the root so the front
page is legible. The commands above assume you are inside `symbolon/`.
