#!/usr/bin/env bash
# 昇格試験 —— 「育成場から出してよいか」を **人でなく機械が言う**。
#
# 由来: COMMON.md §5(Boost の型)で「昇格の条件は機械で書ける」と書いた。⇒ **書けるなら書く。**
#   ① 全実装が cases.json を通る ② 数が出ている(「速いはず」は昇格しない) ③ 契約を変えていない
#   ＋ この repo で実際に効いた三つ: ④ 決定性 ⑤ 投影が実機で立つ ⑥ 言語実装から隔離されている
#
# ◆ 昇格 *先* の器は `symbolon/`(2026-09-04 命名。割符)。⚠️ 通過 7 / 保留 0 を出すまで住まない。
#    ここが出すのは「出してよいか」の可否だけ。道は通すが、扉には名を書かない。
# ⚠️ **skip があれば「昇格可」と言わない**(緑が並ぶことと最後まで走ったことは別)。
#
#   bash symbolon/promote.sh
#   ERIS_EXP03=…/exp/03-wasm-userland BASE=origin/main bash …/promote.sh   # 門④⑥ も撃つ
set -uo pipefail
_finished=0
trap '[ "$_finished" = 1 ] || echo "❌ [締め] **締めに到達せず終了した** ── ここまでの ✓ を緑と読まないこと"' EXIT
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
cd "$here"
pass=0; fail=0; skip=0
ok()   { echo "  ✓ $1"; pass=$((pass+1)); }
ng()   { echo "  ✗ $1"; fail=$((fail+1)); }
sk()   { echo "  · skip: $1"; skip=$((skip+1)); }

echo "== 昇格試験 =="

# ---- 門③ 契約を変えていない(先に撃つ ── 契約が動いていたら他の緑は意味を持たない)----
echo "門③ 契約"
if ! command -v python3 >/dev/null; then sk "python3 が無い"; else
  python3 floor_emit.py >/dev/null 2>&1
  lockv=$(awk '$1=="version"{print $2}' contract.lock)
  bad=0
  while read -r k f want; do
    [ "$k" = "sha256" ] || continue
    got=$(sha256sum "$f" 2>/dev/null | cut -d' ' -f1)
    if [ -z "$got" ]; then echo "    · $f が無い(生成が要る)"; bad=1
    elif [ "$got" != "$want" ]; then echo "    ✗ $f の指紋が錠と違う"; bad=1; fi
  done < contract.lock
  if [ "$bad" = 0 ]; then ok "契約 v$lockv —— 指紋が錠と一致(黙って広げていない)"; else ng "契約が錠から動いている ⇒ **版を上げてから出直す**"; fi
fi

# ---- 門① 全実装が cases.json を通る ----
echo "門① 実装間の一致"
fl="$(mktemp -d)/fl"
if ! command -v rustc >/dev/null; then sk "rustc が無い"; else
  if rustc -O floor_ladder.rs -o "$fl" >/dev/null 2>&1; then
    out="$("$fl" 2>&1)"; rc=$?
    n=$(printf '%s' "$out" | grep -o '四床の食い違い [0-9]*' | grep -o '[0-9]*$')
    if [ "$rc" = 0 ] && [ "${n:-1}" = 0 ]; then ok "四床 + Python が cases を通る(食い違い 0)"
    else ng "cases で食い違い(rc=$rc, 食い違い=${n:-?})"; fi
  else ng "floor_ladder.rs が建たない"; fi
fi

# ---- 門② 数が出ている ----
echo "門② 数が出ている"
if [ ! -x "$fl" ]; then sk "floor_ladder が無い"; else
  python3 probe_emit.py >/dev/null 2>&1
  bad=0; got=0
  for p in probe_clos probe_cons probe_life; do
    o="$("$fl" --probe $p.json 2>&1)"
    printf '%s' "$o" | grep -q '全段一致' || { bad=1; echo "    ✗ $p が一致しない"; }
    printf '%s' "$o" | grep -q '命令の減り' && got=$((got+1))
  done
  if [ "$bad" = 0 ] && [ "$got" -ge 3 ]; then ok "probe 三本が期待値と一致し、**命令数を出している**"
  else ng "数が出ていない or 一致しない(一致=$((3-bad)) 数=$got)"; fi
fi

# ---- 門④ 決定性(要 exp/03)----
echo "門④ 決定性"
if [ -z "${ERIS_EXP03:-}" ] || [ ! -f "${ERIS_EXP03:-/nonexistent}/init.zig" ]; then
  sk "exp/03 が無い(ERIS_EXP03)—— 決定的 icount を撃てない"
elif ! python3 -m ziglang version >/dev/null 2>&1; then sk "zig が無い(pip install ziglang)"
else
  w="$(mktemp -d)"; cp "$ERIS_EXP03"/*.zig "$w"/ 2>/dev/null
  cp compiled.wasm "$w/program.wasm" 2>/dev/null || : > "$w/program.wasm"
  ( cd "$w" && python3 -m ziglang build-exe init.zig -target x86_64-linux -O ReleaseSmall -fstrip -femit-bin=init >/dev/null 2>&1 )
  if [ -x "$w/init" ]; then
    a=$( cd "$w" && ./init "$here/compiled.wasm" 2>&1 | grep -o 'instrs=[0-9]*' )
    b=$( cd "$w" && ./init "$here/compiled.wasm" 2>&1 | grep -o 'instrs=[0-9]*' )
    if [ -n "$a" ] && [ "$a" = "$b" ]; then ok "同じ ROM を二度で同じ数($a)"; else ng "決定的でない($a vs $b)"; fi
  else ng "init が建たない"; fi
fi

# ---- 門⑤ 投影が実機で立つ ----
echo "門⑤ 投影"
if ! command -v node >/dev/null; then sk "node が無い"; else
  "$fl" --web >/dev/null 2>&1; "$fl" --engine >/dev/null 2>&1
  o1="$(node web_verify.mjs 2>&1)"; o2="$(node engine_verify.mjs 2>&1)"
  # ⚠️ 語順に依存しない形で見る。2026-09-04 に `一致 ✓` と `✓ 一致` の違いで
  #    **門番が false negative を出した**(環境は在るのに skip と言った)。門番も測定を汚す。
  b1=$(printf '%s' "$o1" | grep '^chromium:' | grep -c '✓' || true)
  b2=$(printf '%s' "$o2" | grep '^chromium:' | grep -c '✓' || true)
  bx=$(printf '%s' "$o1$o2" | grep '^chromium:' | grep -c '✗' || true)
  if [ "$b1" -ge 1 ] && [ "$b2" -ge 1 ] && [ "$bx" = 0 ]; then ok "**実ブラウザ**が頁の値を出す(段G / 段H とも)"
  elif printf '%s' "$o1$o2" | grep -q '秤 ✓'; then sk "playwright が無く実ブラウザを撃てない(node だけ緑)"
  else ng "投影が立たない"; fi
fi

# ---- 門⑥ 言語実装から隔離されている ----
echo "門⑥ 隔離"
base="${BASE:-origin/main}"
# ⚠️ 守る対象が木の中に無ければ、diff は必ず空 ⇒ **落ちえない門になる**（実測 2026-09-05、影の中で空振りの ✓）。
#    ⇒ 「触っていない」ではなく「測れない」と言う。これは本体の側でしか意味を持たない門。
if [ ! -e "$repo/melon" ] && [ ! -e "$repo/experiments/third/kokkos.py" ]; then
  sk "この木に言語実装が無い ⇒ 隔離は測れない(本体の側でだけ意味を持つ門)"
elif ! git -C "$repo" rev-parse "$base" >/dev/null 2>&1; then sk "基準 $base が引けない(BASE= で指定)"; else
  d=$(git -C "$repo" diff --name-only "$base"...HEAD -- melon/ koinon/ experiments/third/kokkos.py experiments/third/recursion.py | wc -l)
  if [ "$d" = 0 ]; then ok "$base 以降、**言語実装に一行も触れていない**"
  else ng "言語実装に触れている($d ファイル)⇒ 契約だけで足りていない"; fi
fi

# ---- 門⑦ 主張が門番に裏打ちされている ----
echo "門⑦ 主張と門番"
if ! command -v python3 >/dev/null; then sk "python3 が無い"; else
  out=$(cd "$here" && python3 claims.py 2>&1) || true
  if printf '%s' "$out" | grep -q "✓ 判定"; then
    ok "$(printf '%s' "$out" | grep -m1 '② 再導出' | sed 's/^ *//')"
  elif printf '%s' "$out" | grep -q "● 判定"; then
    sk "文書の数の一部を、門を撃てないので作り直せていない"
    printf '%s\n' "$out" | grep -E '^  [①②③⚠]' | sed 's/^/     /'
  else
    ng "文書と門番がずれている"
    printf '%s\n' "$out" | grep -E '^  ✗' | sed 's/^/     /'
  fi
fi

# ---- 判定 ----
echo
echo "通過 $pass / 落ち $fail / 保留 $skip"
if [ "$fail" -gt 0 ]; then
  echo "⛔ **昇格不可** —— 落ちた門を直してから出直す。"
elif [ "$skip" -gt 0 ]; then
  echo "⏸ **保留** —— 落ちてはいないが、$skip 門を *撃てていない*。"
  echo "   ⚠️ 撃てていない門を「通った」と読まない。環境を揃えて撃ち直す。"
else
  echo "✅ **昇格可** —— 七門すべてを撃って通った。"
  echo "   ◆ 昇格先は symbolon/ —— 中身を移してよい。"
fi
_finished=1
[ "$fail" -eq 0 ]
