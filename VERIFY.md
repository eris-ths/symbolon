# Verify

Nothing here asks you to believe a number. Each one has a command that regenerates it.

◆ **Read the fourth column first.** Every instrument below is listed with what it *cannot*
say. A tool's blind spot is the part that goes stale silently, so it is written down next to
the tool rather than in a paragraph further down the page — and a new instrument arrives here
as a row, with that column filled in, or it does not arrive.

## The instruments

| instrument | fire it with | what it can say | ▲ what it cannot say |
|---|---|---|---|
| **the suite** | `bash promote.sh` | whether all seven gates pass on this tree, reported as `passed / failed / skipped` | whether a *skipped* gate would have passed. The run refuses to say "promotable" while any gate merely did not run — ⚠️ **a skip is never read as a pass** |
| **the ledger** | `python3 claims.py` | that every number in these documents still matches the tool that produced it, and that no new bare claim has been added | that a number is *right*. It compares the document against the tool; if both are wrong the same way, they agree |
| **the ladder** | `rustc -O floor_ladder.rs -o /tmp/fl` then `/tmp/fl`, `--probe probe_cons.json`, `--web`, `--engine` | instruction counts before and after folding, arena cell counts, and the byte size of every emitted artifact | wall-clock. Counts are deterministic; times move ±40% between runs on the same code, so ratios are reported only within one run |
| **a real engine, a real browser** | `node probe_run.mjs probe_clos.wasm 45750`, `node web_verify.mjs`, `node engine_verify.mjs` | that the emitted module computes the expected value outside our own runner, and that a browser loads the page and clicks it | anything, when Node or a browser is missing. Two gates then skip — and the suite says so rather than passing |
| **the deterministic counter** | `ERIS_EXP03=/path/to/exp/03-wasm-userland bash icount.sh` | byte-identical instruction counts for every emitted ROM, from a sister runtime we did not write | anything without that runtime. 🔴 The first time we could fire it, three of the ledger's own patterns turned out to be wrong — they had been written for a gate that had never once run (`MISSES.md`) |
| **the engine, on that same instrument** | `{ printf 'v'; cat engine_probe.wasm; } \| futh`, and `engine_probe` in the ROM list of `icount.sh` | the value and the instruction count of the engine's own function body, replayed against the truth trace the reference floor produced. No expected value is written twice — the driver reads the trace | that *the seam* was measured. The seam exports `step` and `memory`; the sister runtime enters at `run`. ⚠️ Widening the seam to fit the instrument would trade a published promise for convenience, so the ladder emits a second module from the same body, exporting `run` and `kernel` only — a thing you measure and a thing you hand over should not answer to the same name |
| **the tower** | `ERIS_EXP03=/path/to/exp/03-wasm-userland bash tower.sh` | that the floor cost of a program does not move when the sister interpreter is stacked h=1/2/3 deep around our ROM — checked by three floors *agreeing*, not by "nothing crashed" | that the **answers** agree. Our ROM imports nothing, so it has no `env.print` and every height emits an empty stdout. What matches is the instruction count, not the value. That is the price of self-sufficiency, and it is written down rather than glossed |

▲ No number is restated on this page. `claims.tsv` names, for every quantity in these
documents, the gate that re-derives it — so which command produces which number is answered by
the ledger rather than by a second copy here that could drift.

---

## Inside the suite

Seven gates. ⚠️ **A skipped gate is never read as a pass** — the run reports
`passed / failed / skipped` separately.

| gate | what it checks | how it can fail |
|---|---|---|
| ③ contract | regenerate, compare fingerprints against the lock | change one byte of the contract silently |
| ① agreement | four interpreter floors and a fifth implementation agree on every case | one implementation drifts |
| ② numbers | probes hit their expected values **and emit instruction counts** | "it should be faster" does not promote |
| ④ determinism | the same module, run twice, yields the same instruction count | anything nondeterministic |
| ⑤ projection | a **real browser** loads the page and reports the value it computed | the page renders but does not compute |
| ⑥ isolation | no language implementation was touched since the baseline | the contract alone was not enough |
| ⑦ claims | every number in the documents is re-derived from its gate | a document goes stale |

## Inside the ledger

| pass | what it does | ◆ what it catches |
|---|---|---|
| ① presence | the sentence the ledger quotes must actually appear in the document | "I wrote it" when nothing was written |
| ② re-derivation | fire the gate, extract the number, compare against the document | a document that has gone stale against its own tool |
| ③ coverage | a quantity in neither the ledger nor the exemption list fails the run — **and an exemption that no longer matches anything fails too** | ⚠️ a ratchet in both directions: bare claims cannot accumulate, and stale exemptions cannot be left behind to quietly widen the net later |
| ④ anchors | every limit stated in prose must contain a ledger sentence, or declare itself unanchored with a reason | a limit that rots silently, because gates only watched the numbers |
| ⑤ anchor quality | the count of gates anchored on a **fixed token** may rise and never fall | replacing a machine-checkable token with a human sentence, which comes loose the day that sentence is edited |

Exemptions live in `claims.skip`, and each one carries a reason drawn from a fixed vocabulary
(restatement / derived / historical / structural constant / figure of speech).
**"It was tedious" is not a reason.**
