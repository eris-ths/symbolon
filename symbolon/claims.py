#!/usr/bin/env python3
# 門⑦ —— 主張と門番の突き合わせ (DESIGN A7 の機械化)
#
# なぜ在るか:
#   八条(秤)は *測定* を守っていたが、*報告* を守っていなかった。
#   実際、第七段で表だけ直して本文を直さず、第九段で注記だけ直して表を直さなかった。
#   どちらも誰にも気づかれずに残った。⇒ 段が文書の全体に触れたかを、人でなく機械が見る。
#
# 三つの関門:
#   ① 突き合わせ  台帳の言う一文が、その文書に実際に在るか(= 「書いたつもり」を潰す)
#   ② 再導出      その数を、門番を撃って作り直し、一致するか(= 文書の数が古くないか)
#   ③ 被覆        文書に在る「量」で、台帳にも免除表にも無いものが出ていないか(ratchet)
#
# 台帳 claims.tsv の列: id / 文書 / 姿 / 門 / 正規表現
#   姿   … 文書に literal で在るべき一文。主張する数だけを ⟦ ⟧ で括る。
#   正規表現 … 門の出力から同じ数を取り出す。捕獲群ひとつ。空なら ② を飛ばす(門が無い主張)。
import os, re, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
DOCS = os.path.abspath(os.path.join(HERE, ".."))
FL   = os.environ.get("FL", "/tmp/fl")

門 = {
    "ladder": [FL],
    "clos":   [FL, "--probe", "probe_clos.json"],
    "cons":   [FL, "--probe", "probe_cons.json"],
    "web":    [FL, "--web"],
    "engine": [FL, "--engine"],
    "life":      [FL, "--probe", "probe_life.json"],
    "life_own":  [FL, "--probe", "probe_life.json", "--own"],
    "life_own2": [FL, "--probe", "probe_life.json", "--own2"],
    "icount": ["bash", "icount.sh"],
}

def 正規化(s):
    return re.sub(r"[,\s]", "", s)

def 読む(path):
    with open(os.path.join(DOCS, path), encoding="utf-8") as f:
        return f.read()

def 台帳():
    rows = []
    with open(os.path.join(HERE, "claims.tsv"), encoding="utf-8") as f:
        for ln, line in enumerate(f, 1):
            line = line.rstrip("\n")
            if not line.strip() or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) != 5:
                sys.exit(f"台帳 {ln} 行目: 列が 5 でない({len(parts)})")
            rows.append((ln, *parts))
    return rows

def 撃つ(name):
    """門を一度だけ撃って出力を返す。撃てなければ None(= skip)。"""
    if name not in 撃つ.cache:
        cmd = 門[name]
        if cmd[0] == FL and not os.path.exists(FL):
            撃つ.cache[name] = None
        elif name == "icount" and not os.environ.get("ERIS_EXP03"):
            撃つ.cache[name] = None
        else:
            try:
                p = subprocess.run(cmd, cwd=HERE, capture_output=True, text=True, timeout=900)
                撃つ.cache[name] = p.stdout + p.stderr
            except Exception:
                撃つ.cache[name] = None
    return 撃つ.cache[name]
撃つ.cache = {}

def main():
    rows = 台帳()
    落ちた, 飛ばした, 通った = [], [], 0

    print("== 門⑦ 主張と門番の突き合わせ ==\n")
    print(f"  台帳 {len(rows)} 件\n")

    # ── 関門① 突き合わせ ─────────────────────────────
    doc_cache = {}
    for ln, cid, doc, 姿, gate, rx in rows:
        if doc not in doc_cache:
            doc_cache[doc] = 読む(doc)
        needle = 姿.replace("⟦", "").replace("⟧", "")
        if needle not in doc_cache[doc]:
            落ちた.append((cid, "①突き合わせ", f"{doc} に この一文が無い: {needle[:70]}"))

    # ── 関門② 再導出 ─────────────────────────────────
    for ln, cid, doc, 姿, gate, rx in rows:
        if not rx:
            飛ばした.append((cid, "門が無い(台帳が正規表現を持たない)"))
            continue
        out = 撃つ(gate)
        if out is None:
            飛ばした.append((cid, f"門 {gate} を撃てない"))
            continue
        m = re.search(r"⟦(.+?)⟧", 姿)
        if not m:
            落ちた.append((cid, "②再導出", "姿に ⟦ ⟧ が無い"))
            continue
        主張 = 正規化(m.group(1))
        g = re.search(rx, out, re.M)
        if not g:
            落ちた.append((cid, "②再導出", f"門 {gate} の出力から取り出せない: /{rx}/"))
            continue
        実測 = 正規化(g.group(1))
        if 主張 != 実測:
            落ちた.append((cid, "②再導出", f"文書 {主張} ⟷ 門 {gate} {実測}"))
        else:
            通った += 1

    # ── 関門③ 被覆 ───────────────────────────────────
    # ⚠️ 英語の面でも量は量。日本語の助数詞しか見ていないと、英語の一枚だけ裸になる（実測 2026-09-05）。
    量 = re.compile(
        r"([0-9][0-9,]*(?:\.[0-9]+)?)[-\s]*(?:B\b|バイト|セル|個(?![人性])|命令|分の\s*1"
        r"|bytes?\b|cells?\b|instructions?\b|×)"
        r"|([0-9][0-9,]{2,})\s*→"          # 「前 → 後」の前側
        r"|→\s*([0-9][0-9,]{2,})"          # その後側
    )
    免除 = set()
    skip_path = os.path.join(HERE, "claims.skip")
    if os.path.exists(skip_path):
        for line in open(skip_path, encoding="utf-8"):
            line = line.strip()
            if line and not line.startswith("#"):
                免除.add(line.split("\t")[0])
    覆われた = {}
    for ln, cid, doc, 姿, gate, rx in rows:
        needle = 姿.replace("⟦", "").replace("⟧", "")
        覆われた.setdefault(doc, []).append(needle)
    # DESIGN.md は自分で「ここは数を持たない」と宣言している ⇒ その宣言も機械が見る。
    # 表に立つ文書は、数を持つ以上すべて走査する。⚠️ ここに足し忘れると、その一枚だけ裸になる。
    走査 = sorted(set(r[2] for r in rows) | {
        "DESIGN.md",                # 自分で「数を持たない」と宣言している ⇒ 宣言を機械が見る
        "symbolon/README.md",       # 表の一枚。「信じるな、走らせろ」の対象そのもの
        "symbolon/VERIFY.md",
        "symbolon/MISSES.md",
    })
    未被覆 = []
    for doc in 走査:
        if doc not in doc_cache:
            doc_cache[doc] = 読む(doc)
        text = doc_cache[doc]
        for i, line in enumerate(text.splitlines(), 1):
            if not 量.search(line):
                continue
            covered = any(n in line for n in 覆われた.get(doc, []))
            for m in 量.finditer(line):
                tok = m.group(0)
                key = f"{doc}:{正規化(tok)}"
                if covered or key in 免除:
                    continue
                未被覆.append((doc, i, tok.strip(), key))

    # ── 判定 ──────────────────────────────────────────
    for cid, 関門, why in 落ちた:
        print(f"  ✗ {cid:<22} {関門}  {why}")
    if 落ちた:
        print()
    print(f"  ① 突き合わせ  文書に在る            {len(rows) - sum(1 for c in 落ちた if c[1]=='①突き合わせ')}/{len(rows)}")
    print(f"  ② 再導出      門を撃って一致した    {通った} 件(飛ばした {len(飛ばした)} 件)")
    if 未被覆:
        print(f"  ③ 被覆        ▲ 台帳にも免除表にも無い量 {len(未被覆)} 件:")
        for doc, i, tok, key in 未被覆[:80]:
            print(f"       {doc}:{i}  {tok}    ← 台帳に足すか claims.skip に理由付きで")
        if len(未被覆) > 80:
            print(f"       … 他 {len(未被覆)-80} 件")
    else:
        print(f"  ③ 被覆        新しい未被覆なし")
    print()
    if 飛ばした:
        名 = sorted(set(w for _, w in 飛ばした))
        print(f"  ⚠ 飛ばした: {' / '.join(名)}")
        print( "     ⇒ 飛ばしが在るうちは「文書は門番に裏打ちされている」と言わない。")
    if 落ちた or 未被覆:
        print("\n  ✗ 判定: 文書と門番がずれている")
        return 1
    if 飛ばした:
        print("\n  ● 判定: 撃てた範囲では一致(飛ばしあり)")
        return 0
    print("\n  ✓ 判定: 全ての主張が門番から作り直せた")
    return 0

if __name__ == "__main__":
    sys.exit(main())
