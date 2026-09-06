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
import os, re, shutil, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
DOCS = os.path.abspath(os.path.join(HERE, ".."))
FL   = os.environ.get("FL", "")

def 建てる():
    """門を撃つ実体（床の梯子）を用意する。
    ⚠️ 2026-09-05 実測 —— ここが `/tmp/fl` 決め打ちだったため、**手元では偶然通り、CI では
       55 件すべてを黙って飛ばした**。単独で叩かれても成り立つように、無ければ自分で建てる。
       ※ この 55 は **その日の台帳の行数**（履歴）。いまの数ではない —— 現在の数は走らせれば出る。"""
    global FL
    if FL and os.path.exists(FL):
        return True
    src = os.path.join(HERE, "floor_ladder.rs")
    if not shutil.which("rustc") or not os.path.exists(src):
        return False
    out = os.path.join(tempfile.mkdtemp(), "fl")
    if subprocess.run(["rustc", "-O", src, "-o", out],
                      capture_output=True).returncode != 0:
        return False
    FL = out
    return True

建った = 建てる()

# (コマンド, 前提) —— **前提が満たされなければ skip。満たされて rc≠0 なら「壊れた」= 落ち。**
# 🔴 2026-09-05、指摘を受けて直した。それまでは
#    ① rc を見ておらず、失敗した門の *エラー文* を「門の出力」として cache していた
#    ② skip の判定が icount だけの特別扱いで、一般則になっていなかった
#    ⇒ 「撃てない(skip)」と「撃ったが壊れた(落ち)」は **別の事実**。混ぜると
#      「落ちるべきでない落ち」の隣に「静かに通る」が待つ。
# ⚠️ 前提に「素材が在ること」を書かない —— 台帳が名指す門の入力が無いのは
#    *環境が無い* のではなく **この木の欠陥**。skip にすると「緑だが行が裸」に戻る。
#    ⇒ 素材の不在は門が rc≠0 で落ち、**壊れた** として鳴る。skip は外の物にだけ許す。
門 = {
    "ladder":    ([FL],                                              None),
    "clos":      ([FL, "--probe", "probe_clos.json"],                 None),
    "cons":      ([FL, "--probe", "probe_cons.json"],                 None),
    "str":       ([FL, "--probe", "probe_str.json"],                 None),
    "web":       ([FL, "--web"],                                     None),
    "engine":    ([FL, "--engine"],                                  None),
    "life":      ([FL, "--probe", "probe_life.json"],                 None),
    "life_own":  ([FL, "--probe", "probe_life.json", "--own"],                 None),
    "life_own2": ([FL, "--probe", "probe_life.json", "--own2"],                 None),
    # 🔴 **断りを要る門**(2026-09-05)。他の門は「畳めた/一致した」を要るが、この二本は
    #    **畳めないと言うこと**を要る。証拠の向きは同じ（在ることを要る）—— 印 [REFUSE …] が
    #    出力に無ければ落ちる。⚠️ 錨は符牒であって文言ではない（言い換えで静かに外れないように）。
    # 🔴 **入れ子の cons**(2026-09-05)。畳むだけの門は壊れていても緑だった
    #    ⇒ この形は **実走まで撃たないと証拠にならない**（下の nest_run が本体）。
    "nest":      ([FL, "--probe", "probe_nest.json"],                 None),
    # 🔴 **閉包の捕捉**(2026-09-06)。三段が inline 先の env で名前を解いていた。
    #    ⚠️ 所有の走査は「list の箱」が在る時だけ走る ⇒ **--own2 でも撃つ**（別の穴がそこに在った）。
    "capture":   ([FL, "--probe", "probe_capture.json"],              None),
    "cap_own2":  ([FL, "--probe", "probe_capture.json", "--own2"],    None),
    # 🔴 **枝で置き方が割れる list**(2026-09-06、差分ファズが掴んだ)。要素が 0..255 の枝だけ
    #    段G‴ が詰めて Str にするので、同じ「list を返す if」が **形の不一致**として断られていた。
    #    ◆ 断りを *狭めた* 直し ⇒ 「畳めた」だけでなく **実走**まで要る（join_run が本体）。
    "join":      ([FL, "--probe", "probe_join.json"],                 None),
    # 🔴 **回収の判定を、両側から撃つ**(2026-09-06)。`--own` のセル単位回収は
    #    「外れた頭が bump heap の **頂**に在る」時だけ hp を戻す。`probe_life` は作りたてを
    #    舐めるので **通る側しか撃っていなかった** ⇒ 否定側(後から積んでから舐める)を足した。
    #    ◆ 分岐を持つ最適化は、通る側と通らない側の両方を撃って初めて撃ったと言える。
    "lifo":      ([FL, "--probe", "probe_lifo.json", "--own"],        None),
    "lifo_def":  ([FL, "--probe", "probe_lifo.json"],                 None),
    "prepend":   ([FL, "--probe", "probe_prepend.json"],              None),
    "prependw":  ([FL, "--probe", "probe_prependw.json"],             None),
    "strbox":    ([FL, "--probe", "probe_strbox.json"],               None),
    # ⚠️ 吐いた物を走らせる門は、吐く門の *後* でなければ意味がない。順を宣言する
    #    （cache の並び順に頼っていた —— 脆かった）。
    "str_run":   (["node", "probe_run.mjs", "probe_str.wasm", "87320"], ("後に", "str")),
    "nest_run":  (["node", "probe_run.mjs", "probe_nest.wasm", "702"], ("後に", "nest")),
    "prep_run":  (["node", "probe_run.mjs", "probe_prepend.wasm", "88020"], ("後に", "prepend")),
    "cap_run":   (["node", "probe_run.mjs", "probe_capture.wasm", "414"], ("後に", "capture")),
    "join_run":  (["node", "probe_run.mjs", "probe_join.wasm", "303"], ("後に", "join")),
    # 峰は **実走でしか出ない** ⇒ 回収の両側とも run 側で見る（畳めただけでは証拠にならない）
    "life_own_run": (["node", "probe_run.mjs", "probe_life.own.wasm", "25500"], ("後に", "life_own")),
    "lifo_run":  (["node", "probe_run.mjs", "probe_lifo.own.wasm", "45500"], ("後に", "lifo")),
    "lifo_def_run": (["node", "probe_run.mjs", "probe_lifo.wasm", "45500"], ("後に", "lifo_def")),
    "life_run":  (["node", "probe_run.mjs", "probe_life.wasm", "25500"], ("後に", "life")),
    # 🔴 **書かれた言語の側から撃つ門**(2026-09-06)。手で置いた probe は「思いついた形」しか覆えない
    #    ⇒ 種を固定した差分ファズで、形の方を機械に作らせる。⚠️ 種と本数は決め打ち（再現できないと門にならない）。
    #    ⚠️ 前提は **言語実装が木に在ること**。影には出さない物なので、公開では skip と名乗る
    #      （門⑥ と同じ理由）。「撃てない」と「撃って壊れた」を混ぜない。
    "fuzz":      (["python3", "fuzz.py", "--n", "120", "--seed", "20260906", "--depth", "4"],
                  ("file", "../experiments/third/kokkos.py")),
    "icount":    (["bash", "icount.sh"],                             ("env", "ERIS_EXP03")),
}

# 🔴 **錨の要る節**(2026-09-06)。関門③ は *量* しか見ない ⇒ **散文で書かれた能力の主張**は
#    台輪の外に在って、古くなっても誰も鳴らない。
#    実測: 公開 README の「No strings」が、文字列を入れた **二日後も**残っていた。
#    外から生ページを読ませたら「integers and integer lists しか扱わない」と要約された ——
#    ⇒ 読み手が実際に誤解する所まで届いていた。数だけ守っても、入口は守れていなかった。
# ◆ 直し方: 限界を並べる節の **各項に錨を要る**。錨とは「台帳の行が名指す一文」で、
#   その行は門を撃って一致することを要る ⇒ *散文が門に裏打ちされる*。
# ⚠️ 錨を持てない項は在る（scope の決めごと・立場）。**消さずに、理由付きで免除**する ——
#   claims.skip と同じ作法。「錨が無い」ことを文書自身にも書かせる(▲ Unanchored:)。
錨の要る節 = [("symbolon/README.md", "## ▲ What this does not do")]

# 🔴 **錨の質**(2026-09-06)。門は「出力のどこ」に錨を取るかで壊れ方が変わる ——
#   人の一文に錨づけた門は、**その一文を正しく直した日に静かに外れる**。
#   実測: 2026-09-05 に「全段一致」を正した時に一本落ち、2026-09-06 に
#   「床D が実際に回した命令」を正した時にもう一本落ちた（どちらも落ちたのは正しい。
#   ⚠️ 怖いのは *落ちる* 方ではない —— **量を取り損ねて黙る**形の方）。
#   ⇒ `[REFUSE …]` `[NOIMPORT ok]` `[AGREE stage-DEF]` `[RUN ok]` `[D-tower]` のような
#     **符牒**に錨を取った門を数え、**減らさない**（一方通行の爪車）。
#   ▲ 全部を符牒にはしていない。`機械語 (\d+) B` のように数のすぐ隣で短い錨は、
#     言い換えの的になりにくい。**まず文らしい錨から替える。**
符牒の床 = 12        # ⚠️ 増やすのは可。**減らしたら落ちる。** 替えたら、この数も上げる
#   11 → 12: 四十四段で `[NO-IFOP ok]` を足した（爪車は一方通行）

錨の免除 = {
    # 節の項の **先頭の太字**をそのまま鍵にする。⚠️ 文言を変えたら外れる —— それは正しい（別の主張だから）
    "No surface syntax.": "scope の決めごと。測る対象が無い",
    "Not the fastest thing available.": "何を最適化していないかの立場。測る対象が無い",
}

壊れた = object()          # 撃ったが落ちた。⚠️ skip ではない —— 隠すと「静かに通る」へ繋がる

def 正規化(s):
    return re.sub(r"[,\s]", "", s)

def 解決(path):
    """台帳の道をそのまま試し、無ければ **同じ名前**を木の中から探す。
    ⚠️ 影では文書の置き場所が変わる（notes/ へ回す）が、名は変えない ⇒ 名で引ければ両方で通る。
       名を変えたら見つからない。それは正しい —— 名が変わったなら別の文書だから。"""
    direct = os.path.join(DOCS, path)
    if os.path.exists(direct):
        return direct
    名 = os.path.basename(path)
    見 = []
    for d, dirs, fs in os.walk(DOCS):
        dirs[:] = [x for x in dirs if x not in (".git", ".umbra", "node_modules", "__pycache__")]
        if 名 in fs:
            見.append(os.path.join(d, 名))
    if len(見) == 1:
        return 見[0]
    if not 見:
        sys.exit(f"⛔ 台帳の指す文書が無い: {path}")
    sys.exit(f"⛔ 同じ名前が {len(見)} 箇所に在る: {path} ⇒ どれが正か決められない")

def 読む(path):
    with open(解決(path), encoding="utf-8") as f:
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

def 前提(name):
    """満たされなければ *skip の理由* を返す。満たされていれば None。
    ⚠️ 門ごとの特別扱いをやめ、宣言に寄せた（前は icount だけを名指しで見ていた）。"""
    cmd, need = 門[name]
    if cmd[0] == FL and not 建った:
        return "床の梯子を建てられない(rustc が無い)"
    if need is None:
        return None
    kind, v = need
    if kind == "env" and not os.environ.get(v):
        return f"{v} が無い"
    if kind == "file" and not os.path.exists(os.path.join(HERE, v)):
        return f"{v} が無い"
    if kind == "後に":
        o = 撃つ(v)
        if o is None or o is 壊れた:
            return f"門 {v} が先に撃てていない"
    return None


def 撃つ(name):
    """門を一度だけ撃つ。前提が無ければ None(skip)、**rc≠0 なら 壊れた**、通れば出力。"""
    if name not in 撃つ.cache:
        why = 前提(name)
        if why is not None:
            撃つ.cache[name] = None; 撃つ.理由[name] = why
        else:
            try:
                p = subprocess.run(門[name][0], cwd=HERE, capture_output=True, text=True, timeout=900)
                if p.returncode != 0:                      # 🔴 ここを見ていなかった
                    撃つ.cache[name] = 壊れた
                    t = (p.stderr or p.stdout).strip().splitlines()
                    撃つ.理由[name] = f"rc={p.returncode} / {t[-1][:80] if t else '(出力なし)'}"
                else:
                    撃つ.cache[name] = p.stdout + p.stderr
            except Exception as e:
                撃つ.cache[name] = 壊れた; 撃つ.理由[name] = f"起動できない: {e}"
    return 撃つ.cache[name]
撃つ.cache = {}
撃つ.理由 = {}

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
        if out is 壊れた:
            落ちた.append((cid, "②門が壊れた", f"門 {gate}: {撃つ.理由.get(gate, '')}"))
            continue
        if out is None:
            飛ばした.append((cid, f"門 {gate}: {撃つ.理由.get(gate, '撃てない')}"))
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

    # ── 関門④ 錨 ─────────────────────────────────────
    # ⚠️ 判定は **関門③ と同じ述語**を使い回す（「この行は台帳の一文を含むか」）。
    #    別の判定を書くと、二つの門が別々にずれていく。
    錨なし = []
    for doc, 見出し in 錨の要る節:
        if doc not in doc_cache:
            doc_cache[doc] = 読む(doc)
        行 = doc_cache[doc].splitlines()
        if 見出し not in 行:
            落ちた.append((f"錨/{doc}", "④錨", f"節が無い: {見出し} ⇒ 節ごと消えたか、名が変わった"))
            continue
        i = 行.index(見出し)
        j = next((k for k in range(i + 1, len(行)) if 行[k].startswith("## ")), len(行))
        for k in range(i + 1, j):
            ln = 行[k]
            if not ln.startswith("- "):
                continue
            # 項は次の「- 」まで続く（折り返しを含めて一項として見る）
            項 = [ln]
            m = k + 1
            while m < j and not 行[m].startswith("- ") and not 行[m].startswith("## "):
                項.append(行[m]); m += 1
            本文 = "\n".join(項)
            頭 = re.search(r"^- \*\*(.+?)\*\*", ln)
            鍵 = 頭.group(1) if 頭 else ln[2:40]
            if 鍵 in 錨の免除:
                continue
            if any(n in 本文 for n in 覆われた.get(doc, [])):
                continue
            錨なし.append((doc, k + 1, 鍵))

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
            # ⚠️ **免除表に書く形そのもの**を出す。見た目のまま写すと空白と comma で当たらない
            #    （実測 2026-09-06: 出力を写して貼り、一件だけ通らなかった）。
            #    ◆ 道具の報告は、そのまま次の手に使えて初めて読める。
            print(f"       {doc}:{i}  {tok}    ← claims.skip なら  {key}<TAB>理由<TAB>説明")
        if len(未被覆) > 80:
            print(f"       … 他 {len(未被覆)-80} 件")
    else:
        print(f"  ③ 被覆        新しい未被覆なし")
    if 錨なし:
        print(f"  ④ 錨          ▲ 門に錨づいていない限界 {len(錨なし)} 件:")
        for doc, i, 鍵 in 錨なし:
            print(f"       {doc}:{i}  「{鍵}」  ← 台帳の一文を項に入れるか、錨の免除へ理由付きで")
    else:
        print(f"  ④ 錨          限界の節は全て門に裏打ちされている({len(錨の免除)} 件は理由付きで免除)")

    符牒 = [r for r in rows if re.search(r"\\\[", r[5])]
    足りない = len(符牒) < 符牒の床
    if 足りない:
        print(f"  ⑤ 錨の質      ⛔ **符牒に錨を取った門が減った** {len(符牒)} / 床 {符牒の床} —— "
              f"人の一文に戻すと、その一文を直した日に静かに外れる")
    else:
        print(f"  ⑤ 錨の質      符牒に錨を取った門 {len(符牒)} / {len(rows)}"
              + (f"（床 {符牒の床} を {len(符牒)-符牒の床} 上回った ⇒ 床も上げること）" if len(符牒) > 符牒の床 else "（床ちょうど）"))
    print()
    if 飛ばした:
        名 = sorted(set(w for _, w in 飛ばした))
        print(f"  ⚠ 飛ばした: {' / '.join(名)}")
        print( "     ⇒ 飛ばしが在るうちは「文書は門番に裏打ちされている」と言わない。")
    if 落ちた or 未被覆 or 錨なし or 足りない:
        print("\n  ✗ 判定: 文書と門番がずれている")
        return 1
    if 飛ばした:
        print("\n  ● 判定: 撃てた範囲では一致(飛ばしあり)")
        return 0
    print("\n  ✓ 判定: 全ての主張が門番から作り直せた")
    return 0

if __name__ == "__main__":
    sys.exit(main())
