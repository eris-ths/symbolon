# Security

## If you found something that should not be here

This tree is **generated** from a private source tree. A secret, a credential, an internal
path or a personal detail appearing here means it also exists on the other side, and that the
guard which produces this tree has a hole.

🔴 **Do not open a public issue for it.** A public issue points at the thing and makes it
easier to find.

**Report it privately:** GitHub → **Security** → *Report a vulnerability*. That opens a draft
advisory only the maintainers can read.

⚠️ **What will happen, honestly:** the tooling that publishes this tree cannot delete this
repository, and neither can the agent that maintains it. Closing the window requires a person.
So the response is: a human is told immediately, the repository is taken down by them, and the
cause is fixed on the source side — not here, because writing here changes nothing.

▲ Deleting a repository does not undo publication. Forks, caches and archives keep copies. We
publish on the assumption that what goes out stays out; the report path exists to shorten the
window, not to pretend it can be closed.

## Ordinary bugs

Wrong numbers, failing gates, broken commands — those are **not** security reports. Open a
normal issue; that is the fastest path and the numbers have gates behind them.

## What this code does when you run it

- `promote.sh` compiles `floor_ladder.rs` with `rustc`, writes artifacts next to it, and runs
  Node scripts. One gate launches a headless browser and loads a local page.
- It makes **no network calls** and reads nothing outside its own directory.
- The emitted modules import nothing: they export functions and, when a heap is needed, a
  memory. They cannot reach a host, by construction, which is also why they cannot do anything
  interesting on their own.
