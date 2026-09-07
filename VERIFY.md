# Verify

Nothing here asks you to believe a number. Each one has a command that regenerates it.

## Everything at once

```bash
bash promote.sh
```

Seven gates. ⚠️ **A skipped gate is never read as a pass** — the run reports
`passed / failed / skipped` separately and refuses to say "promotable" while any gate was
merely not run.

| gate | what it checks | how it can fail |
|---|---|---|
| ③ contract | regenerate, compare fingerprints against the lock | change one byte of the contract silently |
| ① agreement | four interpreter floors and a fifth implementation agree on every case | one implementation drifts |
| ② numbers | probes hit their expected values **and emit instruction counts** | "it should be faster" does not promote |
| ④ determinism | the same module, run twice, yields the same instruction count | anything nondeterministic |
| ⑤ projection | a **real browser** loads the page and reports the value it computed | the page renders but does not compute |
| ⑥ isolation | no language implementation was touched since the baseline | the contract alone was not enough |
| ⑦ claims | every number in the documents is re-derived from its gate | a document goes stale |

## Just the claims

```bash
python3 claims.py
```

Three passes:

1. **presence** — the sentence the ledger quotes must actually appear in the document.
   ◆ This is what catches "I wrote it" when nothing was written.
2. **re-derivation** — fire the gate, extract the number, compare against the document.
3. **coverage** — any quantity in the documents that is in neither the ledger nor the
   exemption list fails the run. ⚠️ A ratchet: new bare claims cannot accumulate.

The ledger is `claims.tsv`; exemptions are `claims.skip`, and each one carries a reason drawn
from a fixed vocabulary (restatement / derived / historical / structural constant / figure of
speech). **"It was tedious" is not a reason.**

## Reproducing a single number

```bash
rustc -O floor_ladder.rs -o /tmp/fl

/tmp/fl                                # 6,103,390 → 15,011, and the arena cell counts
/tmp/fl --probe probe_clos.json        # 2,350,010 → 5,711
/tmp/fl --probe probe_cons.json        # 3,967,486 → 9,320
/tmp/fl --web                          # wasm and HTML byte sizes
/tmp/fl --engine                       # the engine module

node probe_run.mjs probe_clos.wasm 45750    # runs it, warms up first, checks linearity
node web_verify.mjs                          # drives a real browser
node engine_verify.mjs                       # clicks a real page
```

▲ **Instruction counts are deterministic; wall-clock is not.** Times move ±40% between runs on
the same code. We report ratios only within a single run, and never compare milliseconds
across runs. The reasoning is in `notes/COMMON.md` §1.

## The deterministic counter

```bash
ERIS_EXP03=/path/to/exp/03-wasm-userland bash icount.sh
```

Runs the emitted modules under a sister runtime that prints byte-identical instruction counts
for a fixed seed. ⚠️ Without it, two gates skip — and the suite says so rather than passing.

🔴 The first time we could finally run it, three of the ledger's own patterns turned out to be
wrong. They had been written for a gate that had never once been fired. See `MISSES.md`.

## The engine, on the same instrument

```bash
{ printf 'v'; cat engine_probe.wasm; } | futh      # value
ERIS_EXP03=/path/to/exp/03-wasm-userland bash icount.sh   # instruction count, in the ROM list
```

The engine seam exports `step` and `memory`; the sister runtime enters at `run`. For a long
time this repository recorded that as a reason the engine could not be measured there.
▲ That was one way in, not the only one. Adding `run` to the seam would widen a published
promise in order to make measurement convenient — the wrong direction. So the seam is
untouched, and the ladder emits a second module from **the same function body**, with a driver
that replays the truth trace the Python floor produced. Nothing is copied, and no expected
value is written down twice: the driver reads the trace.

⚠️ That module is not the seam. It exports `run` and `kernel` only, and deliberately does not
carry the names `step` and `memory` — a thing you measure and a thing you hand over should not
answer to the same name.

## The tower

```bash
ERIS_EXP03=/path/to/exp/03-wasm-userland bash tower.sh
```

Stacks the sister interpreter h=1/2/3 deep around our ROM and reads the `[cost]` each layer
prints for itself. The innermost number is the floor cost of P; it must not move with h.
⚠️ We check that by **presence** — three floors that agree — not by "nothing crashed".

▲ **What this does not show: that the answers agree.** Our ROM imports nothing, so it has no
`env.print` and every height emits an empty stdout. The sister suite anchors output identity
byte-for-byte; we cannot, because that anchor needs an import we deliberately do not have.
What matches here is the instruction count, not the value. That is the price of self-sufficiency,
and it is written down rather than glossed.
