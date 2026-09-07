#!/usr/bin/env bash
# 語彙表の生死 —— `README.md` の一覧が、**まだ在る物**を指しているか。
#
# ◆ なぜ要るか（五十四段）。外からの査読:「独自語彙の一覧が一枚あると、外の人が入口で止まらない」。
#   ⚠️ だが README は「翻訳は黙って腐るから notes を訳さない」と明記している ⇒ **手で書いた一覧は同じ形**。
#   ⇒ 一覧は置く。**写しを作らず**、置いた一覧を門が読む形にする（姉妹 repo の「表を印で名乗り、門は印だけを読む」形と同型）。
#
# 🔴 **一覧の正は README の表そのもの。** ここに語を書き写さない —— 写しを持てば片方だけ直る。
#
# ▲ 片側しか撃てない（正直に名乗る）:
#     撃てる —— 表に在る語が、指した先から消えた（＝腐り）
#     撃てない —— 本文に在る独自語が、表に無い（*不在* は導出できない）
#   ⇒ 後者は **床**で持つ（四十一段の符牒の床と同じ作法）。証明ではなく歯止め。
#
# ⚠️ *不在* を根拠にする門なので、**測れたことを先に要る**（二十三段の型）——
#    表が読めなかった時は「腐り 0」ではなく **落ちる**。

set -uo pipefail
cd "$(dirname "$0")"

floor=18    # ⚠️ 減ったら落ちる。**増やしたらこの数も上げること**（符牒の床と同じ向き）

# ◆ 面によって置き場が違う（影は root、本体は symbolon/ の中）⇒ **道を書かず、名で探す**。
find_one() {
  for d in . ..; do [ -f "$d/$1" ] && { echo "$d/$1"; return 0; }; done
  return 1
}

readme=$(find_one README.md) || { echo "  ⛔ README.md が無い —— 測れない"; exit 1; }

echo "== 語彙表の生死 =="

rows=$(awk '
  /<!-- lexicon -->/ { seen=1; next }
  seen && /^\|/       { started=1; print; next }
  seen && started     { exit }
' "$readme")

# 🔴 この行は逆クォートを持たない。持たせると **その場でコマンド置換になる**（五十四段で実測）——
#    今日 walker.sh で直したのと同じ族を、直した手で書いた。踏まない枝の文は、踏むまで壊れていられる。
[ -n "$rows" ] || { echo '  ⛔ 語彙の表が読めない（印 <!-- lexicon --> の直後に表が無い）—— 測れない'; exit 1; }

n=0; dead=0
while IFS= read -r line; do
  cells=$(printf '%s' "$line" | sed 's/^|//; s/|$//')
  term=$(printf '%s' "$cells" | awk -F'|' '{print $1}' | tr -d '`*' \
         | sed 's/[①②③④⑤]//g; s/^ *//; s/ *$//')
  where=$(printf '%s' "$cells" | awk -F'|' '{print $3}' | tr -d '`*' | sed 's/^ *//; s/ *$//')
  # ※ 見出し行と罫線は数えない
  [ -z "$term" ] && continue
  case "$term" in term|---*) continue;; esac
  printf '%s' "$term" | grep -q '^[-: ]*$' && continue
  [ -z "$where" ] && continue
  n=$((n+1))
  f=$(find_one "$where") || { echo "  ⛔ **指した先が無い** $term → $where"; dead=$((dead+1)); continue; }
  grep -qi -- "$term" "$f" || { echo "  ⛔ **指した先に居ない** $term → $where"; dead=$((dead+1)); }
done <<< "$rows"

[ "$n" -gt 0 ] || { echo "  ⛔ 表から一行も読めなかった —— **「腐り 0」ではなく「測れない」**"; exit 1; }

echo "  語 $n / 生きている $((n - dead)) / **指した先から消えた $dead**"

if [ "$dead" -gt 0 ]; then
  echo "  ⛔ **語彙表が腐っている** —— 直す先は表か、動かした先の名"
  exit 1
fi
if [ "$n" -lt "$floor" ]; then
  echo "  ⛔ **語彙が減った** $n / 床 $floor —— 消したのなら床も下げること"
  exit 1
fi
if [ "$n" -gt "$floor" ]; then
  echo "  ◆ 床 $floor を $((n - floor)) 上回った ⇒ **床も上げること** [LEXICON ok]"
else
  echo "  語彙表は $n 語（床ちょうど） [LEXICON ok]"
fi
