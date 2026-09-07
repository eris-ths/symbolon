#!/usr/bin/env bash
# 塔 —— 姉妹実験 exp/03(wasm-userland)の init を h 段積んで、**P の底値が高さに依るか**を撃つ。
#
# なぜ要るか: §1-2 に十段から「▲ 高さ非依存は自分で確かめていない —— 塔を建てず collapse 値を直に取った」
#   と書いたままだった。あちらの門番 `verify/verify-wasi-tower-collapse.sh` は錨④ として同じことを撃つが、
#   撃っているのは **あちらの ROM**(userland/tower-kernel.c)。⇒ わたしの ROM で撃たなければ、わたしの主張ではない。
#
#   h=1 : native-init が P を直に byte-walk         (interp 1 層)
#   h=2 : native-init → init.wasm → P               (interp 2 層)
#   h=3 : native-init → init.wasm → init.wasm → P   (interp 3 層)
#   各層は自分の [cost] を吐く ⇒ **一番内側の instrs が「P の底値」**。これが h で動かないことが高さ非依存。
#
# ⚠️ あちらの錨①(出力恒等)は、こちらでは張れない —— わたしの ROM は自給で env.print を import しない
#   ∴ 出力は全高さで 0 byte。**一致しているのは命令数であって、値ではない。** 自給の代価をここに書いておく。
#
# 作法は icount.sh と同じ ── **写さず指す**。あちらの木には一切書かない(*.zig を作業場へ写し、
# program.wasm もそこに置く)。姉妹実験が無ければ正直に skip する。
#
#   ERIS_EXP03=/path/to/exp/03-wasm-userland bash symbolon/tower.sh
#
# ※ h=3 は k² 段の乗算(≈ 93 億 dispatch)で、この器では 1 分ほどかかる。TOWER_H3=0 で外せる。
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exp03="${ERIS_EXP03:-}"
h3on="${TOWER_H3:-1}"

if [ -z "$exp03" ] || [ ! -f "$exp03/init.zig" ]; then
  echo "exp/03 が見つからない(ERIS_EXP03 未設定 or init.zig 不在)—— skip。"
  echo "  ERIS_EXP03=…/exp/03-wasm-userland bash $0"
  exit 0
fi
command -v python3 >/dev/null || { echo "python3 が要る —— skip。"; exit 0; }
python3 -m ziglang version >/dev/null 2>&1 || { echo "zig が無い(pip install ziglang)—— skip。"; exit 0; }
[ -f "$here/compiled.wasm" ] || { echo "compiled.wasm が無い(先に floor_ladder を撃つこと)—— skip。"; exit 0; }

w="$(mktemp -d)"; trap 'rm -rf "$w"' EXIT
cp "$exp03"/*.zig "$w"/ 2>/dev/null
Z=(python3 -m ziglang)

build_native() { ( cd "$w" && "${Z[@]}" build-exe init.zig -target x86_64-linux -O ReleaseSmall -fstrip -femit-bin="$1" >/dev/null 2>&1 ); }
build_wasm()   { ( cd "$w" && "${Z[@]}" build-exe init.zig -target wasm32-wasi -O ReleaseSmall --stack 1048576 -femit-bin="$1" >/dev/null 2>&1 ); }
# 各層の [cost] を上から順に。⚠️ 一番 **内側**が先に吐く(内から畳まれて戻るため)= 先頭が P の底値。
costs() { grep -o 'instrs=[0-9]*' "$1" | cut -d= -f2; }

echo "== 塔 —— exp/03 の init を h 段積んで P(compiled.wasm)を回す =="
printf '%-6s %s\n' '高さ' '各層の [cost](内側 → 外側)'

# h=1
cp "$here/compiled.wasm" "$w/program.wasm"
build_native h1 || { echo "⛔ h=1 の build に失敗"; exit 1; }
( cd "$w" && ./h1 >o1.bin 2>e1.txt ) || true
mapfile -t c1 < <(costs "$w/e1.txt")
[ "${#c1[@]}" -ge 1 ] || { echo "⛔ h=1 が数を出さなかった"; exit 1; }
printf '%-6s %s\n' 'h=1' "${c1[*]}"

# init.wasm(P を embed)
cp "$here/compiled.wasm" "$w/program.wasm"
build_wasm initP.wasm || { echo "⛔ init.wasm の build に失敗"; exit 1; }

# h=2
cp "$w/initP.wasm" "$w/program.wasm"
build_native h2 || { echo "⛔ h=2 の build に失敗"; exit 1; }
( cd "$w" && ./h2 >o2.bin 2>e2.txt ) || true
mapfile -t c2 < <(costs "$w/e2.txt")
[ "${#c2[@]}" -ge 2 ] || { echo "⛔ h=2 が二層分の数を出さなかった(${c2[*]:-空})"; exit 1; }
printf '%-6s %s\n' 'h=2' "${c2[*]}"

c3=()
if [ "$h3on" != 0 ]; then
  cp "$w/initP.wasm" "$w/program.wasm"
  build_wasm initinitP.wasm || { echo "⛔ init.wasm² の build に失敗"; exit 1; }
  cp "$w/initinitP.wasm" "$w/program.wasm"
  build_native h3 || { echo "⛔ h=3 の build に失敗"; exit 1; }
  ( cd "$w" && ./h3 >o3.bin 2>e3.txt ) || true
  mapfile -t c3 < <(costs "$w/e3.txt")
  printf '%-6s %s\n' 'h=3' "${c3[*]:-（数が出なかった）}"
fi

# collapse: P の threaded 残余を native-init が直に(interp 層ゼロ)
( cd "$w" && "${Z[@]}" build-exe futamura-harness.zig -O ReleaseSafe -femit-bin=futh >/dev/null 2>&1 ) || true
nc=""
if [ -x "$w/futh" ]; then
  nc="$( { printf 'c'; cat "$here/compiled.wasm"; } | ( cd "$w" && ./futh ) 2>&1 >/dev/null | grep -o 'instrs=[0-9]*' | tail -1 | cut -d= -f2 )"
  [ -n "$nc" ] && printf '%-6s %s\n' 'collapse' "$nc   ← interp 層ゼロ = A(P)"
fi

echo
# 出力 byte —— 自給の ROM は何も出さない。**「同じ」ではなく「無い」**と名乗る。
o_sizes="$(for f in o1 o2 o3; do [ -f "$w/$f.bin" ] && wc -c < "$w/$f.bin"; done | tr '\n' ' ')"
echo "▲ 出力は全高さで 0 byte（$o_sizes）—— 自給の ROM は env.print を import しない"
echo "   ⇒ 一致しているのは **命令数** であって、値ではない。あちらの錨①(出力恒等)はここでは張れない。"

# 一層の乗算 k
if [ "${#c2[@]}" -ge 2 ]; then
  printf '一層の乗算 h2/h1 %s 倍\n' "$(python3 -c "print(f'{${c2[-1]}/${c1[-1]}:.1f}')")"
fi
if [ "${#c3[@]}" -ge 3 ]; then
  printf '一層の乗算 h3/h2 %s 倍\n' "$(python3 -c "print(f'{${c3[-1]}/${c3[-2]}:.1f}')")"
fi

# ── 錨④ 高さ非依存 ──────────────────────────────
# 🔴 **不在ではなく在で判定する** —— 「落ちなかった」ではなく「底値が三度そろって在る」を見る。
base=("${c1[0]}" "${c2[0]}")
[ "${#c3[@]}" -ge 1 ] && base+=("${c3[0]}")
same=1
for v in "${base[@]}"; do [ "$v" = "${base[0]}" ] || same=0; done
hs="h=1/2$( [ "${#c3[@]}" -ge 1 ] && echo '/3' )"
if [ "$same" = 1 ] && [ "${#base[@]}" -ge 2 ]; then
  echo "  高さ非依存: 底値は $hs で ${base[0]} —— 一致 [TOWER ok]"
else
  echo "  ⛔ 底値が高さで動いた: ${base[*]} —— 高さ非依存は偽"
  exit 1
fi
