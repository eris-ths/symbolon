# symbolon

**Every application grows a small interpreter.** Feature-flag conditions, validation rules,
pricing logic, search filters, workflow steps — each one ends up as a JSON tree plus a
hand-written function that walks it. That walker re-reads the same tree on every call, grows a
new `case` every sprint, gets rewritten once per platform, and the copies drift apart.

`symbolon` replaces it with two moves.

1. **Write the meaning as data, not as a `switch`.** The walker stops being something you
   maintain and becomes something generated from the contract. Five back-ends here were written
   without reading a single line of the language's implementation — only the contract data.
2. **Fold the interpreter against one program** — the first Futamura projection, the same
   operation as `re.compile()` or precompiling a template, applied to a general interpreter.
   The dispatch disappears; what is left is a straight line for that one program.

Measured: the interpreter's step count falls by **407×**, and the output is a **100-byte** wasm
module — no LLVM, no emscripten, no runtime, nothing to install.

▲ This is a research vessel, not a product. Read *What this does not do* before adopting it.

◆ **There is a second thing here, and for some readers it is the first.** Every number above was
produced by a machine, and is re-derived by gates that read the claim out of the document and out
of the tool and refuse to agree unless the two match. The *subject* is the Futamura projection;
the *method* is what to do when the thing writing your numbers is also the thing being checked.
`MISSES.md` is the honest half of it — predictions written down before each run, kept whether or
not they held. If you came for the method rather than for the interpreter, read `MISSES.md`,
`claims.py` and `promote.sh` first, and read the rest as the worked example they run against.

`symbolon` — Greek σύμβολον: a tally broken in two, matched later to prove it is genuine. The
name is the method: everything here is checked by putting two halves together.

**Apache-2.0.** Copyright 2026 eris-ths.

⚠️ Almost every commit here is authored by `umbra`, because **this tree is generated, not
written**. Three are not, and that is recorded at the bottom along with the reason the history
is short.

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

## Terms

This tree names things, and the names are not standard. ▲ The Japanese notes carry more of them;
these are the ones you need to read the English pages.

⚠️ **`projection` means two different things here** and both are load-bearing. Which one is meant
is always decidable from context, and confusing them is the fastest way to misread this repository.

<!-- lexicon -->

| term | what it means here | where it lives |
|---|---|---|
| fold | specialising the interpreter against one program, so the dispatch disappears | `README.md` |
| projection ① | the Futamura kind: interpreter + program → a program | `README.md` |
| projection ② | the publishing kind: this whole tree, regenerated from a private repository | `MISSES.md` |
| shadow | this published tree, as opposed to the source it is projected from | `README.md` |
| tally | σύμβολον: two halves matched to prove a claim, rather than one side asserting it | `README.md` |
| floor | one rung of the interpreter, A through D — the thing being folded away | `floor_ladder.rs` |
| ladder | the four floors plus the specialiser and back-ends, in one file | `floor_ladder.rs` |
| stage | an emission target above the floors — E through H: specialised, x86-64, wasm, engine | `floor_ladder.rs` |
| specialiser | the thing that performs the fold | `MISSES.md` |
| probe | one small program used as a subject, with its expected answer | `floor_ladder.rs` |
| seam | the engine's exported surface (`step`, `memory`) — what a host would hold onto | `floor_ladder.rs` |
| ROM | an emitted module handed to the sister runtime to be counted, not run for its value | `tower.sh` |
| walker | the pass that reads an emitted module's opcodes and reports what it stepped on | `walker.sh` |
| gate | a check that can fail. A skipped one is never read as passed | `VERIFY.md` |
| ledger | the table of every numeric claim, with the command that re-derives each one | `VERIFY.md` |
| anchor | the token a ledger row matches on. A row anchored on prose goes quiet when the prose is edited | `VERIFY.md` |
| ratchet | a count that may move one way only, so a check cannot be weakened by accident | `VERIFY.md` |
| exemption | a ledger row excused from re-derivation, with a reason. An exemption that excuses nothing fails | `VERIFY.md` |

◆ A gate reads this table and fails if a term no longer appears where the last column says it does,
and if the table shrinks. ▲ It cannot do the other direction — it does not know which coined words
are missing from the table. That half is a floor, not a proof.

---

## What was measured

| program | interpreter instructions, before → after folding | ratio |
|---|---|---|
| integer loop (`sum 1..1000`) | 6,103,390 → 15,011 | **407×** |
| closure-heavy (300 non-escaping closures) | 2,350,010 → 5,711 | **411×** |
| cons-heavy (build a 300-cell list, fold it) | 3,967,486 → 9,320 | **426×** |
| string-heavy (a 47-character string, ×20) | 7,250,554 → 17,331 | **418×** |

| artifact | size |
|---|---|
| wasm (integer loop / closure) | **100 B** / **107 B** |
| wasm with a bump heap (cons-heavy) | **226 B** |
| wasm for the string-heavy program (literal packed into the data section) | **256 B** |
| x86-64 machine code, hand-encoded, `mmap` + W^X | **102 B** |
| engine module (state + `step`) | **159 B** |
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
refuse to compare across them — see the eight articles in `notes/COMMON.md` §1.

---

## Layout

```
README.md      this page          notes/COMMON.md   the chronicle (Japanese)
VERIFY.md      how to check it    SECURITY.md       how to report a leak
MISSES.md      what we got wrong  git log           one entry per step, in English

symbolon/      everything runnable — from here down, run commands inside it:

machine.json   the contract: an interpreter for the object language,
               written in the 14 instructions, as data
cases.json     the acceptance conditions, as data
contract.lock  version + fingerprints. Change the contract → raise the version

floor_ladder.rs   four interpreter floors, the specializer, and the back-ends
floor_ladder.mjs  the same fold, in a second runtime, as a cross-check

promote.sh     the gate suite. Skips are never read as passes
claims.py      every numeric claim, re-derived from the tool that produced it
icount.sh      deterministic instruction counts from a sister runtime
tower.sh       the same runtime stacked h deep; P's floor cost must not move with h
walker.sh      which branches of the opcode walker its own output has never exercised
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

⚠️ Every line below is anchored on something a gate shoots, or is listed as unanchored with a
reason. A limit stated in prose goes stale silently — this section said "no strings" for two
days after strings landed, and no gate rang, because gates only watched the numbers.

- **No string type.** A string is a list of small integers; the language gained no new form and
  the contract did not move. A literal is packed one byte per character and read in place, and
  putting an arbitrary integer in front of one is refused rather than truncated — the gate
  requires that refusal to appear as `REFUSE str-prepend-nonbyte`.
- **No calls out.** The emitted module exports and imports nothing, so it cannot reach a host.
  The emitter scans its own output and must report `NOIMPORT ok`; break it and the run exits
  non-zero rather than going quiet.
- **No surface syntax.** Programs arrive encoded. A front-end is deliberately out of scope.
  ▲ Unanchored: a scope decision, not a measurement. Nothing to shoot.
- **Not the fastest thing available.** It is the smallest thing that can be checked end to end.
  ▲ Unanchored: a stance about what is being optimised. Nothing to shoot.

---

## ⚠️ This repository is a shadow

It is **generated** from a private working repository, every time, in full. That is why the
commits here are authored by `umbra`.

🔴 **Three of them are not, and we are leaving them that way.** They carry a different author —
visible in `git log` — because the clone's identity had been set to satisfy a tooling check on
the source side, and nothing was watching that line: the guard protecting this tree reads
files, and an author is not a file. The history here is append-only, one projection at a time,
so rewriting them is not on the table; a published mistake left visible is worth more than a
tidy history. A gate now refuses to add a commit under any other name, and prints how many
already exist, so the count can only stay where it is.

**How to contribute, given that.**

- **Open an issue.** They are read, and they are the normal path — a finding, a question, a
  disagreement with a number all land better here than as a diff.
- **Send a patch in the issue** if you have one. It gets applied on the source side and arrives
  here in the next projection, with attribution. ⚠️ A pull request against these files cannot be
  merged: the next projection would overwrite it.
- Only files named in a manifest on the source side appear here. Anything not named stays home,
  which is why some paths mentioned in the Japanese notes do not exist in this tree.

`notes/COMMON.md` is the working notebook — the chronicle, in Japanese, published as-is rather than
translated. A translation is one more thing that can silently drift, and we would not be able to
gate it. Its normative counterpart, the requirements read backwards out of what was actually
measured, is kept on the source side and is not published.

**If you want it in English, read `git log`.** Every commit here is a projection, and its message is
a written account of that step — what changed, what was measured, what turned out to be wrong. It is
written, not generated: a summary produced by machine would be one more thing that can drift, for
the same reason the notes are not translated. The log is the English record; the notes are the
Japanese one; neither is derived from the other.

▲ The published tree is arranged as: this page, `VERIFY.md` and `MISSES.md` at the root; the
Japanese notes under `notes/`; and everything runnable — contract, floors, specializer, gates —
under `symbolon/`. The commands above assume you are inside `symbolon/`.
