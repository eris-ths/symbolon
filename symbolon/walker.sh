#!/usr/bin/env bash
# 歩き手の被覆 —— `ifop_count` の表のうち、**一度も踏まれていない枝**を数えて名乗る。
#
# ◆ なぜ要るか（四十八段で実測、五十二段で門にした）。歩き手は *知らない* opcode に当たれば
#   「測れない」で落ちる。⚠️ だが **知っている枝を一度も踏まないまま通る**ことは黙って起きる。
#   ⇒ 「通った」は「その形を見た」ではない。三十六段「引かれなかった形は数に出ない」と同じ族。
#
# 🔴 消すのではなく **数える**。`0x6A`(i32.add) は *いま出ていないだけ* で出る形 ——
#    死んだ枝を削ると、次にその形を吐いた日に歩き手が「測れない」で止まる（＝検査が弱くなる）。
#
# ⚠️ 表は **歩き手の source から引く**（写しを持たない）。
#    ◆ これは ⑨ が禁じた向きではない —— ⑨ は *歩き手の表を、歩き手が判定する相手（吐き出し器）から*
#      導くなという話で、反対できなくなるから。ここで導くのは **数える側の表を、数える対象から**。
#      数える側が歩き手に反対する必要は無い ⇒ 二十六段「宣言は使用から導く」の側。
#
# ⚠️ *不在* を根拠にする門なので、**測れたことを先に要る**（二十三段の型）——
#    module を一本も歩けなかった / 表が読めなかった 時は「0 本死んでいる」ではなく **落ちる**。

set -u
cd "$(dirname "$0")"

ceil=9      # ⚠️ 増えたら落ちる。**減ったらこの数も下げること**（符牒の床と同じ作法、向きが逆）
#
# 🔴 四十八段の本文は **8 本**と書いていた。この門を建てて数え直したら 9 本 —— 記録が誤っていた。
#    差は `0x05`(else)。⚠️ これは走査の広さの違いではない: `floor_ladder.rs` は
#    「`else`(0x05)は使わない —— `if`(0x04)と対で、init.zig が持たない側」と **設計として書いており**、
#    吐き口に 0x05 を出す場所が無い（当たるのは 段F の x86 バイト `0x48 0x05` だけ）⇒ **構造上 出得ない**。
#    ◆ つまり 8 は、門を持たない一度きりの走査で数えて文書に貼った数だった —— この repo が
#      いちばん嫌う形を、その型を書いた節自身が踏んでいた。⇒ 数え直す口を置いたので、もう腐らない。
#
# ▲ 九本は **同じ死に方ではない**:
#    `0x05`(else) は **設計として死んでいる**（NO-IFOP の主張そのものが、これが出ないことに依っている）。
#    残る八本は **まだ出ていないだけ**（`0x6A`=i32.add は出る形）。⇒ どちらも消さない。理由が違うだけ。

fl=/tmp/fl.walker
rustc -O floor_ladder.rs -o "$fl" >/dev/null 2>&1 || { echo "  ⛔ 梯子が建たない —— 測れない"; exit 1; }
python3 probe_emit.py  >/dev/null 2>&1
python3 engine_emit.py >/dev/null 2>&1

tbl=$(sed -n '/const NOIMM/,/];/p' floor_ladder.rs | grep -o '0x[0-9A-Fa-f]\{2\}' \
     | tr '[:lower:]' '[:upper:]' | sed 's/^0X/0x/' | sort -u)
[ -n "$tbl" ] || { echo "  ⛔ 歩き手の表が読めない —— 測れない"; exit 1; }

out=$(
  for p in probe_*.json; do "$fl" --probe "$p"; done
  "$fl" --probe probe_life.json --own
  "$fl" --probe probe_life.json --own2
  "$fl" --web
  "$fl" --engine
) 2>&1

mods=$(printf '%s\n' "$out" | grep -c '\[NO-IFOP ok\]' || true)
hit=$(printf '%s\n' "$out" | sed -n 's/.*歩いた枝: //p' | tr ',' '\n' \
         | tr '[:lower:]' '[:upper:]' | sed 's/^ *//; s/ *$//; s/^0X/0x/' | grep -v '^$' | sort -u)

echo "== 歩き手の被覆 =="
if [ "$mods" -lt 1 ] || [ -z "$hit" ]; then
  echo "  ⛔ module を一本も歩けなかった —— **「死んだ枝 0」ではなく「測れない」**"
  exit 1
fi

live=$(comm -12 <(printf '%s\n' "$tbl") <(printf '%s\n' "$hit"))
dead=$(comm -23 <(printf '%s\n' "$tbl") <(printf '%s\n' "$hit"))
ntbl=$(printf '%s\n' "$tbl" | grep -c .)
nlive=$(printf '%s\n' "$live" | grep -c . || true)
ndead=$(printf '%s\n' "$dead" | grep -c . || true)

echo "  歩いた module $mods 本 / 歩き手の表 $ntbl 本"
echo "  踏んだ $nlive / **一度も踏んでいない $ndead**: $(printf '%s ' $dead)"
echo "  ⚠️ 消さない —— 「いま出ていない」であって「出ない」ではない。削ると、次にその形を吐いた日に歩き手が止まる"

if [ "$ndead" -gt "$ceil" ]; then
  echo "  ⛔ **覆えていない枝が増えた** $ndead / 天井 $ceil —— 表に足した形を、誰も吐いていない"
  exit 1
fi
if [ "$ndead" -lt "$ceil" ]; then
  echo "  ◆ 天井 $ceil を $((ceil - ndead)) 下回った ⇒ **天井も下げること**"
else
  echo "  歩き手の表は $ndead 本が未踏（天井ちょうど） [WALKER ok]"
  exit 0
fi
echo "  [WALKER ok]"
