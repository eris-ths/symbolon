#!/usr/bin/env bash
# 合流の門番 —— 姉妹実験 exp/03(wasm-userland)の init.zig で、**決定的な命令数**を測る。
#
# なぜ要るか: この lab の梯子は ms で測っていたが、容器の絶対時間は走行間で ±40% 動き、
# JIT の段(tier)が測定を支配する ── 八段目の「region 回収の費用は分布に埋もれて区別がつかない」は
# **計器の限界**だった。init.zig は seed 固定で byte 一致する `[cost] instrs=` を刷る ∴ そこが消える。
#
# 作法は ptyx の `scripts/rom/probe.sh` と同じ ── **写さず指す**(pointer は腐らない)。
# 姉妹実験が無ければ正直に skip する。
#
#   ERIS_EXP03=/path/to/exp/03-wasm-userland bash symbolon/icount.sh
#
# ⚠️ 二種類の走行を分ける:
#   峰を読む走行  = 計器つき(`.wasm`)      —— heap の峰が memory[0] に出る
#   費用を測る走行 = 計器なし(`.np.wasm`)  —— 計器自体が命令数を動かすため(実測: 回収すると
#                                            hp>peak が偽になり店じまいが省かれ、**回収した方が少なく出た**)
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exp03="${ERIS_EXP03:-}"

if [ -z "$exp03" ] || [ ! -f "$exp03/init.zig" ]; then
  echo "exp/03 が見つからない(ERIS_EXP03 未設定 or init.zig 不在)—— skip。"
  echo "  ERIS_EXP03=…/exp/03-wasm-userland bash $0"
  exit 0
fi
command -v python3 >/dev/null || { echo "python3 が要る —— skip。"; exit 0; }
python3 -m ziglang version >/dev/null 2>&1 || { echo "zig が無い(`pip install ziglang`)—— skip。"; exit 0; }

work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
cp "$exp03"/*.zig "$work"/ 2>/dev/null
cp "$here/compiled.wasm" "$work/program.wasm" 2>/dev/null || : > "$work/program.wasm"
( cd "$work" && python3 -m ziglang build-exe init.zig -target x86_64-linux -O ReleaseSmall -fstrip -femit-bin=init >/dev/null )

icount() { ( cd "$work" && ./init "$1" 2>&1 ) | grep -oE 'instrs=[0-9]+' | cut -d= -f2; }
peakcells() { ( cd "$work" && ./init "$1" 2>&1 ) >/dev/null; }   # 峰は node 側(probe_run.mjs)が読む

echo "== 決定的な命令数(exp/03 の init.zig。byte-walk・seed 固定で byte 一致)=="
printf '%-26s %10s\n' 'ROM' 'instrs'
# ⚠️ probe_join は **値位置の `if`** を持つ唯一の ROM —— 2026-09-06 に block(void) 二枚へ
#    書き直した形が、あちらの受け側を本当に通るかは **ここでしか測れない**。
#    ▲ その日の器に exp/03 が無かったので、この行自身はまだ一度も撃たれていない（十五段の型:
#      撃てていない門の中身は検証されていない）⇒ 環境が在る日に、まずここを疑う。
# ※ engine.wasm はこの列に入れられない —— export が `step`/`memory` で、あちらの ABI(`run`)と違う。
#   接点の可否は seam の側の問いであって、この計器の問いではない。
for w in compiled probe_clos probe_str probe_join; do
  [ -f "$here/$w.wasm" ] && printf '%-26s %10s\n' "$w" "$(icount "$here/$w.wasm")"
done
echo
echo "-- 段G″: 回収のしかたを A/B(計器なし = 費用の真値)--"
base=""
for w in probe_life.np probe_life.own.np probe_life.own2.np; do
  [ -f "$here/$w.wasm" ] || continue
  n="$(icount "$here/$w.wasm")"
  if [ -z "$base" ]; then base="$n"; printf '%-26s %10s   —\n' "$w" "$n"
  else printf '%-26s %10s   %+d 命令 (%+.3f%%)\n' "$w" "$n" "$((n - base))" \
       "$(python3 -c "print(($n-$base)/$base*100)")"; fi
done
echo
echo "== A(P): 塔を畳んだ底値(collapse = threaded 残余を native-init が直に回す)=="
# 🔴 **撃つ前の予想(2026-09-04。測定前にここへ書いた。外れたら COMMON.md 側を書き換える)**:
#   ① byte-walk : collapse の比 k は、interp 層の塔倍率(exp/03 実測 717〜908)より **ずっと小さい**
#      —— あれは interp を一段積む倍率で、ここは同一層の fetch/LEB/dispatch を焼いた分だけ ∴ 2〜10x と見る。
#   ② **所有(region)の代金は畳んでも消えない。** +80 は local.get/set の *本当の仕事* で解釈の厚みではない
#      ∴ A も同じ向きに増える(比率はほぼ保たれる)。
#   ③ セル単位(+22.3%)も同様に残る。
#   ⇒ **反証**: region の ΔA が ~0 なら、あの +80 は畳めば消える *解釈の厚み* だったことになる
#      —— その時は所有の代金は「底では無料」で、②は外れ。**そちらの方が所有には良い報せ**。
futh="$work/futh"
( cd "$work" && python3 -m ziglang build-exe futamura-harness.zig -O ReleaseSafe -femit-bin=futh >/dev/null 2>&1 ) || true
if [ ! -x "$futh" ]; then
  echo "  futamura-harness の build に失敗 —— A(P) は skip。"
else
  apex() { { printf 'c'; cat "$1"; } | "$futh" 2>&1 >/dev/null | tail -3; }
  printf '%-26s %s\n' 'ROM' 'collapse(A)'
  for w in compiled probe_life.np probe_life.own.np probe_life.own2.np; do
    [ -f "$here/$w.wasm" ] || continue
    out="$(apex "$here/$w.wasm")"
    n="$(printf '%s' "$out" | grep -o 'instrs=[0-9]*' | tail -1 | cut -d= -f2 || true)"
    if [ -n "$n" ]; then printf '%-26s %10s\n' "$w" "$n"
    else printf '%-26s %s\n' "$w" "$(printf '%s' "$out" | tr '\n' ' ')"; fi
  done
fi
echo
echo "▲ ここに ms を書かない。ms は走行間で動く ⇒ 跨いで比べられない(§COMMON.md 1)。"
echo "▲ byte-walk の icount と collapse の A は **別の量**。混ぜて比べない。"
