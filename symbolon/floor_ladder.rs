// 床の梯子 —— 同じ 14-op 契約のまま、床の *実装* だけを一段ずつ上げて head-to-head で測る。
//
// 背景(README 変態四十八 の正直な訂正): 「C/Rust なら Python の 50〜100x」の予想は測定に refute され、
// naive Rust 床は Python の ~8.6x・V8 とほぼ横並びだった。そこで名指しされた真の律速の見立てを、
// **同一プロセスの head-to-head で一段ずつ数字にする**。
//
//   床A  naive   —— 文字列 dispatch + HashMap<String, V> + Rc<V>            (= floor.rs、SOT からの写し)
//   床B  畳み    —— op→enum 節 / register→スロット添字。走行中に文字列を触らない
//   床C  arena   —— 値を Copy(16B)にし、pair を **arena** へ。**Rc = refcount を外す**
//   床D  線形    —— 木の再帰を捨て、**線形命令列 + オペランドスタック**を pc ループで回す
//
// ここまでが「床を磨く」段。A→D で 4.3x 取ったが、bench の 1 反復に床命令が 6,103 個かかる。
// この 6,103x は **塔**(機械そのものを解釈する厚み)で、床をどう磨いても 1 個も減らない。⇒ 塔を畳む段:
//
//   段E  特殊化  —— 機械(machine.json)を *この object 程式* に特殊化する = 第一 Futamura 射影。
//                   ⚠️ 前に 1.1x で外したのは、特殊化先が **また AST** だったから。ここは
//                   **静的に解けるものを全部コンパイル時に解いて命令列へ**落とす(env 参照→スロット、
//                   box→スカラ、dispatch→消滅)。塔そのものが消える。
//   段F  native  —— 段E の整数中核を **x86-64 の機械語へ直接** 吐き、mmap した頁で走らせる。
//                   問い「アセンブラ級にできるか」への実物の答え。アセンブラも LLVM も通さない。
//
// 契約(14 op)も値モデルの *意味*(i64 | Nil | Pair)も machine.json / cases.json も **一切変えていない**。
// 四床は毎回 cases.json で相互に突き合わせ、一つでも食い違えば exit 1(写しの腐りを機構で検出)。
//
// 🔴 床C / 床D の正直な代償 —— **回収しない**。
//   refcount を外すと「いつ解放してよいか」を知る者が居なくなる ⇒ arena は走行中ずっと伸びる。
//   有界な bench だから成立しているだけで、これは GC の不在であって GC の解決ではない。
//   ⇒ ここを埋めるのが **所有**(ptyx の affine な move)。arena の実測サイズがその請求書。
//
//   段G′ 形    —— 値の *形* を静的に推し(Kind = Int | int の list)、**pair を線形メモリの bump heap** へ。
//                   tag も boxing も足さない。形が決まらなければ **断る**。
//   段G″ 寿命  —— 一意所有(`scan_owned`)が言えたら回収する。`--own`=セル単位 / `--own2`=**region ごと捨てる**。
//   段H  接点  —— 状態を線形メモリに置き、host(JS/DOM)に駆動される module を吐く。既存 web/app への seam。
//
// ── 索引(この一枚に何が居るか)────────────────────────────────────────────
//   Json パーサ / 値モデル V(Rc)          … 床A・床B が使う
//   床A ev/ex/run_a  床B ev_b/ex_b/run_b  … 木を歩く二つ
//   Val + Arena / 床C ev_c/ex_c           … Copy 値と arena
//   Ins / 床D run_d・count_d              … 線形命令列(count_d は計器。計測とは別建て)
//   Op / Comp / run_e / jit / Jitted      … 段E(特殊化)と段F(x86-64 を直に吐く)
//   Kind / scan_owned / WComp             … 段G′(形)と段G″(寿命)。wasm を直に吐く
//   wasm_module_d / wasm_engine_module    … module の byte を組む(外部 toolchain ゼロ)
//   main の分岐                            … 既定=梯子A→F / --probe / --web / --engine
//
// ── 走らせ方 ────────────────────────────────────────────────────────────
//   rustc -O floor_ladder.rs -o /tmp/fl                    # lab/ で(data を cwd から読む)
//   /tmp/fl                                                # 梯子 A→F + 四床 differential
//   /tmp/fl --probe probe_life.json [--own|--own2]         # 反証テスト(回収の A/B)
//   /tmp/fl --web    && node web_verify.mjs                # 段G: 自己完結 HTML(実 Chromium で検証)
//   /tmp/fl --engine && node engine_verify.mjs             # 段H: engine seam(本物の DOM クリックで検証)
//
// 正直な線引き:
//   - 床A は head-to-head のため floor.rs から写した。**SOT は floor.rs**。
//   - compiler(program → object-data の符号化)は Python のまま。変態四十四 の線引きは動かしていない。
//   - 束縛規則(env→スロット / box→スカラ / 閉包→inline / 死んだ確保を出さない)は Comp と WComp で
//     **二度書いている**。制御流れの形が違うため back-end を分けた —— ズレは秤が捕まえる。

use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use std::time::Instant;

// ---- 最小 JSON パーサ(floor.rs から写し)----------------------------------------
#[derive(Clone)]
enum Json { Num(i64), Str(String), Arr(Vec<Json>), Obj(Vec<(String, Json)>) }

struct P { c: Vec<char>, i: usize }
impl P {
    fn new(s: &str) -> P { P { c: s.chars().collect(), i: 0 } }
    fn ws(&mut self) { while self.i < self.c.len() && self.c[self.i].is_whitespace() { self.i += 1; } }
    fn val(&mut self) -> Json {
        self.ws();
        match self.c[self.i] { '[' => self.arr(), '{' => self.obj(), '"' => Json::Str(self.string()), _ => self.num() }
    }
    fn arr(&mut self) -> Json {
        self.i += 1; let mut v = Vec::new(); self.ws();
        if self.c[self.i] == ']' { self.i += 1; return Json::Arr(v); }
        loop { v.push(self.val()); self.ws();
            if self.c[self.i] == ',' { self.i += 1; } else { break; } }
        self.i += 1; Json::Arr(v)
    }
    fn obj(&mut self) -> Json {
        self.i += 1; let mut v = Vec::new(); self.ws();
        if self.c[self.i] == '}' { self.i += 1; return Json::Obj(v); }
        loop { self.ws(); let k = self.string(); self.ws(); self.i += 1;
            let val = self.val(); v.push((k, val)); self.ws();
            if self.c[self.i] == ',' { self.i += 1; } else { break; } }
        self.i += 1; Json::Obj(v)
    }
    fn string(&mut self) -> String {
        self.i += 1; let mut s = String::new();
        while self.c[self.i] != '"' { s.push(self.c[self.i]); self.i += 1; }
        self.i += 1; s
    }
    fn num(&mut self) -> Json {
        let mut s = String::new();
        while self.i < self.c.len() && (self.c[self.i].is_ascii_digit() || self.c[self.i] == '-') {
            s.push(self.c[self.i]); self.i += 1; }
        Json::Num(s.parse::<i64>().unwrap_or(0))
    }
}
fn sof(j: &Json) -> &str { match j { Json::Str(s) => s.as_str(), _ => "" } }

/// # 節タグ —— **値オブジェクト**(2026-09-06)
///
/// ⚠️ **契約の 14 op とは別の語彙**。混ぜない:
///   * **14 op** = `machine.json` が *書かれている* 言語（`lit`/`add`/`car`/`respawn`/…）。
///     契約はこちら。数えて 14。版が動くのはこれが変わった時だけ。
///   * **節タグ**(この表) = その機械が *解釈する* 対象言語の節を符号化した番号（0..33）。
///     符号化は `kokkos` の `enc_expr` が正。ここはその **写し**。
///
/// 🔴 なぜ表にしたか。それまでこの file は番号を **69 箇所に直に**書いていて、
///   意味はコメントだけが持っていた（`== 16` の隣に「// ofn」と書く形）。
///   ⇒ 遍在言語が散文には在って **型には無い** 状態。DDD で言う primitive obsession で、
///     実測として効いていた —— 前置の即値判定を書くとき「即値は 0 だったか」を引きに戻った。
/// ⚠️ ここに番号を **足さない**。足すのは符号化の側（契約）で、写しが先に動いたら嘘になる。
#[allow(dead_code)]
mod ntag {
    pub const LIT: i64 = 0;                 // 即値
    pub const ADD: i64 = 1;                 // 加
    pub const SUB: i64 = 2;                 // 減
    pub const VAR: i64 = 3;                 // 変数参照
    pub const LET: i64 = 4;                 // 束縛
    pub const EQ: i64 = 5;                  // 等値
    pub const IF: i64 = 6;                  // 分岐
    pub const NIL: i64 = 7;                 // 空リスト
    pub const CONS: i64 = 8;                // 対を組む(効果位置では文の並び)
    pub const CAR: i64 = 9;                 // 対の第一
    pub const CDR: i64 = 10;                // 対の第二
    pub const PAIRP: i64 = 11;              // 対か
    pub const OFN: i64 = 16;                // 閉包を作る
    pub const OAPP: i64 = 17;               // 閉包を適用
    pub const NEWBOX: i64 = 21;             // 箱を確保
    pub const GETBOX: i64 = 22;             // 箱を読む
    pub const SETBOX: i64 = 23;             // 箱を書く
    pub const WHILE: i64 = 33;              // while
}

// ---- 値モデル(床A / 床B): Int | Nil | Pair、pair は Rc ----------------------------
#[derive(Clone)]
enum V { I(i64), Nil, P(Rc<V>, Rc<V>) }

fn is_pair(v: &V) -> bool { matches!(v, V::P(_, _)) }
fn deep_eq(a: &V, b: &V) -> bool {
    match (a, b) {
        (V::I(x), V::I(y)) => x == y,
        (V::Nil, V::Nil) => true,
        (V::P(a1, a2), V::P(b1, b2)) => deep_eq(a1, b1) && deep_eq(a2, b2),
        _ => false,
    }
}
fn truthy(v: &V) -> bool { !matches!(v, V::I(0)) }
fn show(v: &V) -> String {
    match v { V::I(n) => n.to_string(), V::Nil => "nil".into(), V::P(_, _) => "pair".into() }
}
fn to_v(j: &Json) -> V {
    match j {
        Json::Num(n) => V::I(*n),
        Json::Arr(a) => match &a[0] {
            Json::Str(t) if t == "⟨N⟩" => V::Nil,
            Json::Str(t) if t == "⟨P⟩" => V::P(Rc::new(to_v(&a[1])), Rc::new(to_v(&a[2]))),
            _ => V::I(0),
        },
        _ => V::I(0),
    }
}
/// run() は vs 全体を返すので頂(car)を取る。
fn top(v: V) -> V { match v { V::P(ref car, _) => (**car).clone(), other => other } }

// ==== 床A: naive(floor.rs から写し)===============================================
fn ev(n: &Json, regs: &HashMap<String, V>) -> V {
    let a = match n { Json::Arr(a) => a, _ => return V::I(0) };
    let op = match &a[0] { Json::Str(s) => s.as_str(), _ => return V::I(0) };
    match op {
        "lit" => match &a[1] { Json::Num(k) => V::I(*k), _ => V::I(0) },
        "var" => match &a[1] { Json::Str(k) => regs[k].clone(), _ => V::I(0) },
        "add" => match (ev(&a[1], regs), ev(&a[2], regs)) { (V::I(x), V::I(y)) => V::I(x + y), _ => panic!("add non-int") },
        "sub" => match (ev(&a[1], regs), ev(&a[2], regs)) { (V::I(x), V::I(y)) => V::I(x - y), _ => panic!("sub non-int") },
        "eq" => V::I(if deep_eq(&ev(&a[1], regs), &ev(&a[2], regs)) { 1 } else { 0 }),
        "nil" => V::Nil,
        "cons" => V::P(Rc::new(ev(&a[1], regs)), Rc::new(ev(&a[2], regs))),
        "car" => match ev(&a[1], regs) { V::P(x, _) => (*x).clone(), _ => panic!("car non-pair") },
        "cdr" => match ev(&a[1], regs) { V::P(_, y) => (*y).clone(), _ => panic!("cdr non-pair") },
        "pair" => V::I(if is_pair(&ev(&a[1], regs)) { 1 } else { 0 }),
        _ => panic!("床A: 式にできない {}", op),
    }
}
fn ex(n: &Json, regs: &mut HashMap<String, V>, fuel: &mut i64, respawn: &mut bool) {
    let a = match n { Json::Arr(a) => a, _ => return };
    let op = match &a[0] { Json::Str(s) => s.as_str(), _ => return };
    match op {
        "seq" => if let Json::Arr(ss) = &a[1] { for s in ss { ex(s, regs, fuel, respawn); } },
        "set" | "let" => { let v = ev(&a[2], regs); if let Json::Str(k) = &a[1] { regs.insert(k.clone(), v); } }
        "if" => { if truthy(&ev(&a[1], regs)) { ex(&a[2], regs, fuel, respawn); } else { ex(&a[3], regs, fuel, respawn); } }
        "respawn" => {
            if a.len() > 1 { if let Json::Str(g) = &a[1] { if !truthy(&regs[g]) { return; } } }
            if *fuel <= 0 { return; }
            *fuel -= 1; *respawn = true;
        }
        _ => { ev(n, regs); }
    }
}
fn run_a(machine: &Json, mut regs: HashMap<String, V>, mut fuel: i64) -> V {
    let mut respawn = true;
    while respawn { respawn = false; ex(machine, &mut regs, &mut fuel, &mut respawn); }
    regs["vs"].clone()
}
fn host_a(obj: &Json) -> HashMap<String, V> {
    let mut m = HashMap::new();
    if let Json::Obj(kvs) = obj { for (k, v) in kvs { m.insert(k.clone(), to_v(v)); } }
    m
}

// ==== 畳んだ節(床B / 床C が共有。床D はここからさらに線形化する)===================
enum N {
    Lit(i64), Var(u32),
    Add(u32, u32), Sub(u32, u32), Eq(u32, u32), Cons(u32, u32),
    Nil, Car(u32), Cdr(u32), IsPair(u32),
    Seq(Vec<u32>), Set(u32, u32), If(u32, u32, u32), Respawn(Option<u32>),
}

struct Names(HashMap<String, u32>);
impl Names {
    fn new() -> Names { Names(HashMap::new()) }
    fn intern(&mut self, s: &str) -> u32 {
        if let Some(i) = self.0.get(s) { return *i; }
        let i = self.0.len() as u32; self.0.insert(s.to_string(), i); i
    }
    fn len(&self) -> usize { self.0.len() }
}

/// 機械 AST(Json)を一度だけ畳む: 木 → Vec<N> + 添字、名前 → スロット。
fn build(j: &Json, ns: &mut Vec<N>, nm: &mut Names) -> u32 {
    let a = match j { Json::Arr(a) => a, _ => { ns.push(N::Lit(0)); return (ns.len() - 1) as u32 } };
    let op = match &a[0] { Json::Str(s) => s.clone(), _ => { ns.push(N::Lit(0)); return (ns.len() - 1) as u32 } };
    let node = match op.as_str() {
        "lit"  => N::Lit(match &a[1] { Json::Num(k) => *k, _ => 0 }),
        "var"  => N::Var(nm.intern(sof(&a[1]))),
        "nil"  => N::Nil,
        "add"  => { let x = build(&a[1], ns, nm); let y = build(&a[2], ns, nm); N::Add(x, y) }
        "sub"  => { let x = build(&a[1], ns, nm); let y = build(&a[2], ns, nm); N::Sub(x, y) }
        "eq"   => { let x = build(&a[1], ns, nm); let y = build(&a[2], ns, nm); N::Eq(x, y) }
        "cons" => { let x = build(&a[1], ns, nm); let y = build(&a[2], ns, nm); N::Cons(x, y) }
        "car"  => { let x = build(&a[1], ns, nm); N::Car(x) }
        "cdr"  => { let x = build(&a[1], ns, nm); N::Cdr(x) }
        "pair" => { let x = build(&a[1], ns, nm); N::IsPair(x) }
        "seq"  => { let mut v = Vec::new();
                    if let Json::Arr(ss) = &a[1] { for s in ss { v.push(build(s, ns, nm)); } }
                    N::Seq(v) }
        "set" | "let" => { let slot = nm.intern(sof(&a[1])); let e = build(&a[2], ns, nm); N::Set(slot, e) }
        "if"   => { let c = build(&a[1], ns, nm); let t = build(&a[2], ns, nm); let f = build(&a[3], ns, nm); N::If(c, t, f) }
        "respawn" => { let g = if a.len() > 1 { match &a[1] { Json::Str(s) => Some(nm.intern(s)), _ => None } } else { None };
                       N::Respawn(g) }
        other => panic!("畳み: 未知 op {}", other),
    };
    ns.push(node); (ns.len() - 1) as u32
}

// ==== 床B: 畳み(値は床A と同じ Rc<V> のまま —— 変えたのは dispatch と lookup だけ)===
fn reg_b(regs: &[Option<V>], s: u32) -> &V { regs[s as usize].as_ref().expect("床B: 未束縛 register") }

fn ev_b(ns: &[N], i: u32, regs: &[Option<V>]) -> V {
    match &ns[i as usize] {
        N::Lit(k) => V::I(*k),
        N::Var(s) => reg_b(regs, *s).clone(),
        N::Add(x, y) => match (ev_b(ns, *x, regs), ev_b(ns, *y, regs)) { (V::I(a), V::I(b)) => V::I(a + b), _ => panic!("add non-int") },
        N::Sub(x, y) => match (ev_b(ns, *x, regs), ev_b(ns, *y, regs)) { (V::I(a), V::I(b)) => V::I(a - b), _ => panic!("sub non-int") },
        N::Eq(x, y) => V::I(if deep_eq(&ev_b(ns, *x, regs), &ev_b(ns, *y, regs)) { 1 } else { 0 }),
        N::Nil => V::Nil,
        N::Cons(x, y) => V::P(Rc::new(ev_b(ns, *x, regs)), Rc::new(ev_b(ns, *y, regs))),
        N::Car(x) => match ev_b(ns, *x, regs) { V::P(a, _) => (*a).clone(), _ => panic!("car non-pair") },
        N::Cdr(x) => match ev_b(ns, *x, regs) { V::P(_, d) => (*d).clone(), _ => panic!("cdr non-pair") },
        N::IsPair(x) => V::I(if is_pair(&ev_b(ns, *x, regs)) { 1 } else { 0 }),
        _ => panic!("床B: 式にできない節"),
    }
}
fn ex_b(ns: &[N], i: u32, regs: &mut Vec<Option<V>>, fuel: &mut i64, respawn: &mut bool) {
    match &ns[i as usize] {
        N::Seq(ss) => { for s in ss { ex_b(ns, *s, regs, fuel, respawn); } }
        N::Set(slot, e) => { let v = ev_b(ns, *e, regs); regs[*slot as usize] = Some(v); }
        N::If(c, t, f) => { if truthy(&ev_b(ns, *c, regs)) { ex_b(ns, *t, regs, fuel, respawn) } else { ex_b(ns, *f, regs, fuel, respawn) } }
        N::Respawn(g) => {
            if let Some(g) = g { if !truthy(reg_b(regs, *g)) { return; } }
            if *fuel <= 0 { return; }
            *fuel -= 1; *respawn = true;
        }
        _ => { ev_b(ns, i, regs); }
    }
}
fn run_b(ns: &[N], root: u32, mut regs: Vec<Option<V>>, mut fuel: i64, vs: u32) -> V {
    let mut respawn = true;
    while respawn { respawn = false; ex_b(ns, root, &mut regs, &mut fuel, &mut respawn); }
    reg_b(&regs, vs).clone()
}
fn host_b(obj: &Json, nm: &mut Names) -> Vec<Option<V>> {
    if let Json::Obj(kvs) = obj { for (k, _) in kvs { nm.intern(k); } }
    let mut regs: Vec<Option<V>> = (0..nm.len()).map(|_| None).collect();
    if let Json::Obj(kvs) = obj { for (k, v) in kvs { let s = nm.intern(k) as usize; regs[s] = Some(to_v(v)); } }
    regs
}

// ==== 値モデル(床C / 床D): Copy な 16B 値 + pair は arena の添字。**refcount 無し** =====
#[derive(Clone, Copy, PartialEq)]
enum Val { I(i64), Nil, P(u32) }

type Arena = Vec<(Val, Val)>;

fn cons_a(ar: &mut Arena, a: Val, d: Val) -> Val { ar.push((a, d)); Val::P((ar.len() - 1) as u32) }
fn truthy_v(v: Val) -> bool { !matches!(v, Val::I(0)) }
fn eq_v(ar: &Arena, a: Val, b: Val) -> bool {
    match (a, b) {
        (Val::I(x), Val::I(y)) => x == y,
        (Val::Nil, Val::Nil) => true,
        (Val::P(x), Val::P(y)) => {
            let (a1, a2) = ar[x as usize]; let (b1, b2) = ar[y as usize];
            eq_v(ar, a1, b1) && eq_v(ar, a2, b2)
        }
        _ => false,
    }
}
fn to_val(j: &Json, ar: &mut Arena) -> Val {
    match j {
        Json::Num(n) => Val::I(*n),
        Json::Arr(a) => match &a[0] {
            Json::Str(t) if t == "⟨N⟩" => Val::Nil,
            Json::Str(t) if t == "⟨P⟩" => { let x = to_val(&a[1], ar); let y = to_val(&a[2], ar); cons_a(ar, x, y) }
            _ => Val::I(0),
        },
        _ => Val::I(0),
    }
}
/// 比較・表示のため床A/B の値へ戻す(計測の外側でだけ使う)。
fn val_to_v(ar: &Arena, v: Val) -> V {
    match v {
        Val::I(n) => V::I(n), Val::Nil => V::Nil,
        Val::P(i) => { let (a, d) = ar[i as usize]; V::P(Rc::new(val_to_v(ar, a)), Rc::new(val_to_v(ar, d))) }
    }
}
fn host_cd(obj: &Json, nm: &mut Names, ar: &mut Arena) -> Vec<Option<Val>> {
    if let Json::Obj(kvs) = obj { for (k, _) in kvs { nm.intern(k); } }
    let mut regs: Vec<Option<Val>> = vec![None; nm.len()];
    if let Json::Obj(kvs) = obj { for (k, v) in kvs { let s = nm.intern(k) as usize; regs[s] = Some(to_val(v, ar)); } }
    regs
}

// ==== 床C: 畳み + arena(Rc を外しただけ。木の再帰は床B のまま)=====================
fn reg_c(regs: &[Option<Val>], s: u32) -> Val { regs[s as usize].expect("床C: 未束縛 register") }

fn ev_c(ns: &[N], i: u32, regs: &[Option<Val>], ar: &mut Arena) -> Val {
    match &ns[i as usize] {
        N::Lit(k) => Val::I(*k),
        N::Var(s) => reg_c(regs, *s),
        N::Add(x, y) => match (ev_c(ns, *x, regs, ar), ev_c(ns, *y, regs, ar)) { (Val::I(a), Val::I(b)) => Val::I(a + b), _ => panic!("add non-int") },
        N::Sub(x, y) => match (ev_c(ns, *x, regs, ar), ev_c(ns, *y, regs, ar)) { (Val::I(a), Val::I(b)) => Val::I(a - b), _ => panic!("sub non-int") },
        N::Eq(x, y) => { let a = ev_c(ns, *x, regs, ar); let b = ev_c(ns, *y, regs, ar); Val::I(if eq_v(ar, a, b) { 1 } else { 0 }) }
        N::Nil => Val::Nil,
        N::Cons(x, y) => { let a = ev_c(ns, *x, regs, ar); let d = ev_c(ns, *y, regs, ar); cons_a(ar, a, d) }
        N::Car(x) => match ev_c(ns, *x, regs, ar) { Val::P(i) => ar[i as usize].0, _ => panic!("car non-pair") },
        N::Cdr(x) => match ev_c(ns, *x, regs, ar) { Val::P(i) => ar[i as usize].1, _ => panic!("cdr non-pair") },
        N::IsPair(x) => Val::I(if matches!(ev_c(ns, *x, regs, ar), Val::P(_)) { 1 } else { 0 }),
        _ => panic!("床C: 式にできない節"),
    }
}
fn ex_c(ns: &[N], i: u32, regs: &mut Vec<Option<Val>>, ar: &mut Arena, fuel: &mut i64, respawn: &mut bool) {
    match &ns[i as usize] {
        N::Seq(ss) => { for s in ss { ex_c(ns, *s, regs, ar, fuel, respawn); } }
        N::Set(slot, e) => { let v = ev_c(ns, *e, regs, ar); regs[*slot as usize] = Some(v); }
        N::If(c, t, f) => { let cv = ev_c(ns, *c, regs, ar);
                            if truthy_v(cv) { ex_c(ns, *t, regs, ar, fuel, respawn) } else { ex_c(ns, *f, regs, ar, fuel, respawn) } }
        N::Respawn(g) => {
            if let Some(g) = g { if !truthy_v(reg_c(regs, *g)) { return; } }
            if *fuel <= 0 { return; }
            *fuel -= 1; *respawn = true;
        }
        _ => { ev_c(ns, i, regs, ar); }
    }
}
fn run_c(ns: &[N], root: u32, mut regs: Vec<Option<Val>>, ar: &mut Arena, mut fuel: i64, vs: u32) -> Val {
    let mut respawn = true;
    while respawn { respawn = false; ex_c(ns, root, &mut regs, ar, &mut fuel, &mut respawn); }
    reg_c(&regs, vs)
}

// ==== 床D: 線形命令列 + オペランドスタック(木の再帰そのものを捨てる)================
#[derive(Clone, Copy)]
enum Ins {
    Lit(i64), Load(u32), Add, Sub, Eq, NilV, Cons, Car, Cdr, IsPair,
    Store(u32), JmpF(u32), Jmp(u32), Respawn(i32), Drop,
    // ---- 畳んだ連なり(段D′)。**当て先は数えて選んだ** ---------------------------
    // 実測 2026-09-06、sum 1..1000 の 610 万命令の内訳:
    //   Load 25.5% / Lit 20.3% / Eq 19.8% / JmpF 13.9%  —— 上位四つで **79.5%**。
    //   動的な四連なり `Load Lit Eq JmpF` が **787,565 回**(dispatch の 38.7% を占める)。
    //   ◆ これは機械そのものの **命令選り分け**（op の番号を 14 個の定数と突き合わせる所）。
    //     床の速さの問題ではなく、*機械が機械であること* の値段。だから連なりで出る。
    EqK(u32, i64),            // Load(s); Lit(k); Eq
    EqKJmpF(u32, i64, u32),   // Load(s); Lit(k); Eq; JmpF(t)
    // 🔴 **跡地**。畳んでも列を縮めない —— 縮めると飛び先(添字)が全部ずれ、
    //    「二箇所が一致していなければならない」形になる（段F の長さの表で一度払った型）。
    //    ⇒ 畳んだ命令が自分で `pc` を跨がせる。跡地は **踏まれない**。
    Nop,
}

/// 連なりを畳む(段D′)。◆ **添字は一つも動かさない。**
/// ⚠️ 飛び先になっている位置は畳まない —— 途中へ飛び込まれると、跨ぎが効かず
///   スタックの高さが合わなくなる。飛び先の集合を **列そのものから作る**（別表を持たない）。
fn 畳む(code: &mut Vec<Ins>) -> usize {
    let mut 的: Vec<bool> = vec![false; code.len() + 1];
    for ins in code.iter() {
        match *ins { Ins::JmpF(t) | Ins::Jmp(t) => 的[t as usize] = true, _ => {} }
    }
    let mut n = 0;
    let mut i = 0usize;
    while i + 2 < code.len() {
        let 三 = matches!((code[i], code[i + 1], code[i + 2]),
                          (Ins::Load(_), Ins::Lit(_), Ins::Eq)) && !的[i + 1] && !的[i + 2];
        if !三 { i += 1; continue; }
        let (s, k) = match (code[i], code[i + 1]) { (Ins::Load(s), Ins::Lit(k)) => (s, k), _ => unreachable!() };
        let 四 = i + 3 < code.len() && matches!(code[i + 3], Ins::JmpF(_)) && !的[i + 3];
        if 四 {
            let t = match code[i + 3] { Ins::JmpF(t) => t, _ => unreachable!() };
            code[i] = Ins::EqKJmpF(s, k, t);
            code[i + 1] = Ins::Nop; code[i + 2] = Ins::Nop; code[i + 3] = Ins::Nop;
            i += 4;
        } else {
            code[i] = Ins::EqK(s, k);
            code[i + 1] = Ins::Nop; code[i + 2] = Ins::Nop;
            i += 3;
        }
        n += 1;
    }
    n
}

fn emit_expr(ns: &[N], i: u32, out: &mut Vec<Ins>) {
    match &ns[i as usize] {
        N::Lit(k) => out.push(Ins::Lit(*k)),
        N::Var(s) => out.push(Ins::Load(*s)),
        N::Nil => out.push(Ins::NilV),
        N::Add(x, y) => { emit_expr(ns, *x, out); emit_expr(ns, *y, out); out.push(Ins::Add); }
        N::Sub(x, y) => { emit_expr(ns, *x, out); emit_expr(ns, *y, out); out.push(Ins::Sub); }
        N::Eq(x, y) => { emit_expr(ns, *x, out); emit_expr(ns, *y, out); out.push(Ins::Eq); }
        N::Cons(x, y) => { emit_expr(ns, *x, out); emit_expr(ns, *y, out); out.push(Ins::Cons); }
        N::Car(x) => { emit_expr(ns, *x, out); out.push(Ins::Car); }
        N::Cdr(x) => { emit_expr(ns, *x, out); out.push(Ins::Cdr); }
        N::IsPair(x) => { emit_expr(ns, *x, out); out.push(Ins::IsPair); }
        _ => panic!("床D: 式にできない節"),
    }
}
fn emit_stmt(ns: &[N], i: u32, out: &mut Vec<Ins>) {
    match &ns[i as usize] {
        N::Seq(ss) => { for s in ss { emit_stmt(ns, *s, out); } }
        N::Set(slot, e) => { emit_expr(ns, *e, out); out.push(Ins::Store(*slot)); }
        N::If(c, t, f) => {
            emit_expr(ns, *c, out);
            let jf = out.len(); out.push(Ins::JmpF(0));          // 偽なら else へ
            emit_stmt(ns, *t, out);
            let je = out.len(); out.push(Ins::Jmp(0));           // then の後は end へ
            out[jf] = Ins::JmpF(out.len() as u32);
            emit_stmt(ns, *f, out);
            out[je] = Ins::Jmp(out.len() as u32);
        }
        N::Respawn(g) => out.push(Ins::Respawn(match g { Some(s) => *s as i32, None => -1 })),
        // 裸の式(文の位置に来た式)—— 値を捨てる。木版の `_ => { ev(..) }` と同義。
        _ => { emit_expr(ns, i, out); out.push(Ins::Drop); }
    }
}

/// 床D の実行 —— **計数も実行もここ一つ**。
///
/// 🔴 かつて `count_d` と `run_d` が **十五の腕をそれぞれ持っていた**（違いは数え上げと戻り値だけ）。
///   ◆ 「二箇所が一致していなければならない」形そのもの —— この repo はその型で二度払っている
///     （memory 節を使用の隣に宣言した日 / 命令長の表が実体と一 byte ずれた日）。
///   ⚠️ 2026-09-06、連なりを畳んだ時に **その両方へ四つの腕を足した**。増やす側に回っていた。
///   ⇒ 一つにした。**代償は実測 6%**（16.5 → 17.6 ms、六本 vs 四本の走行で帯が重ならない）。
///     ⚠️ `#[inline(always)]` で両方を完全に単相化しても戻らなかった ⇒ 単相化の漏れではなく、
///       戻り値と生存区間が変わったことの値段。**測った上で払っている。**
///   ◆ なぜ払うか: `count_d` の数は **台帳が門で守っている量**（塔の厚み）。二本立てのままだと、
///     腕の意味が片方だけずれた時に *数だけ静かに間違う* —— 網は答えを見ているので気づかない。
///     ⚠️ Rust の網羅検査は「腕の**取りこぼし**」は止めるが、「腕の**中身の食い違い**」は止めない。
///   ▲ 分母が 6% 遅くなる = 塔の比が 6% 大きく出る。**看板を甘くする向き**なので、
///     比はこの統一後の版で測り直して載せる（下の数はすべて統一後）。
fn exec_d<const COUNT: bool>(code: &[Ins], mut regs: Vec<Option<Val>>, ar: &mut Arena,
                             mut fuel: i64, vs: u32) -> (Option<Val>, u64, u64) {
    let mut st: Vec<Val> = Vec::with_capacity(64);
    let (mut ins, mut rounds) = (0u64, 0u64);
    let mut respawn = true;
    while respawn {
        respawn = false;
        if COUNT { rounds += 1; }
        st.clear();
        let mut pc = 0usize;
        while pc < code.len() {
            if COUNT { ins += 1; }
            match code[pc] {
                Ins::Lit(k) => st.push(Val::I(k)),
                Ins::Load(s) => st.push(regs[s as usize].expect("床D: 未束縛 register")),
                Ins::Add => { let b = st.pop().unwrap(); let a = st.pop().unwrap();
                              match (a, b) { (Val::I(x), Val::I(y)) => st.push(Val::I(x + y)), _ => panic!("add non-int") } }
                Ins::Sub => { let b = st.pop().unwrap(); let a = st.pop().unwrap();
                              match (a, b) { (Val::I(x), Val::I(y)) => st.push(Val::I(x - y)), _ => panic!("sub non-int") } }
                Ins::Eq => { let b = st.pop().unwrap(); let a = st.pop().unwrap();
                             st.push(Val::I(if eq_v(ar, a, b) { 1 } else { 0 })) }
                Ins::NilV => st.push(Val::Nil),
                Ins::Cons => { let d = st.pop().unwrap(); let a = st.pop().unwrap(); let p = cons_a(ar, a, d); st.push(p) }
                Ins::Car => { match st.pop().unwrap() { Val::P(i) => st.push(ar[i as usize].0), _ => panic!("car non-pair") } }
                Ins::Cdr => { match st.pop().unwrap() { Val::P(i) => st.push(ar[i as usize].1), _ => panic!("cdr non-pair") } }
                Ins::IsPair => { let v = st.pop().unwrap(); st.push(Val::I(if matches!(v, Val::P(_)) { 1 } else { 0 })) }
                Ins::Store(s) => { let v = st.pop().unwrap(); regs[s as usize] = Some(v); }
                Ins::Drop => { st.pop(); }
                Ins::JmpF(t) => { let v = st.pop().unwrap(); if !truthy_v(v) { pc = t as usize; continue; } }
                Ins::Jmp(t) => { pc = t as usize; continue; }
                Ins::Respawn(g) => {
                    if g >= 0 && !truthy_v(regs[g as usize].expect("床D: 未束縛 register")) { pc += 1; continue; }
                    if fuel > 0 { fuel -= 1; respawn = true; }
                }
                Ins::Nop => {}
                // 畳んだ連なり —— 自分で跡地を **跨ぐ**（列は縮めていないので添字は元のまま）
                Ins::EqK(sl, k) => {
                    let a = regs[sl as usize].expect("床D: 未束縛 register");
                    // ⚠️ 比べる所を書き直さない。**eq_v を通す** —— 意味の口は一つ。
                    st.push(Val::I(if eq_v(ar, a, Val::I(k)) { 1 } else { 0 }));
                    pc += 3; continue;
                }
                Ins::EqKJmpF(sl, k, t) => {
                    let a = regs[sl as usize].expect("床D: 未束縛 register");
                    pc = if eq_v(ar, a, Val::I(k)) { pc + 4 } else { t as usize };
                    continue;
                }
            }
            pc += 1;
        }
    }
    (regs.get(vs as usize).copied().flatten(), ins, rounds)
}



fn run_d(code: &[Ins], regs: Vec<Option<Val>>, ar: &mut Arena, fuel: i64, vs: u32) -> Val {
    exec_d::<false>(code, regs, ar, fuel, vs).0.expect("床D: vs 未束縛")
}

/// 計数だけの走行(計測とは別建て。「床はもう底か」を ns/命令 で判定するため)。
/// ⚠️ **畳む前の列**に当てると *塔の厚み*、畳んだ後の列に当てると *床の手数* —— 別の量(三十八段)。
fn count_d(code: &[Ins], regs: Vec<Option<Val>>, ar: &mut Arena, fuel: i64) -> (u64, u64) {
    let (_, ins, rounds) = exec_d::<true>(code, regs, ar, fuel, u32::MAX);
    (ins, rounds)
}

// ==== 塔を畳む: object 程式(td)を読むための小道具 ================================
// object 値は pair の入れ子。tag = car、payload = cdr。tag 表は kokkos.enc_expr が正:
//   0 lit / 1 add / 2 sub / 3 var / 4 let / 5 eq / 6 if / 7 nil / 8 cons
//   21 newbox / 22 getbox / 23 setbox / 33 owhile   (bench が使うのはこの部分言語)
fn is_p(j: &Json) -> bool {
    matches!(j, Json::Arr(a) if a.len() == 3 && matches!(&a[0], Json::Str(t) if t == "⟨P⟩"))
}
fn pcar(j: &Json) -> &Json { match j { Json::Arr(a) => &a[1], _ => panic!("car: pair でない") } }
fn pcdr(j: &Json) -> &Json { match j { Json::Arr(a) => &a[2], _ => panic!("cdr: pair でない") } }
fn pnum(j: &Json) -> i64 { match j { Json::Num(n) => *n, _ => panic!("num でない") } }

// ==== 段E/F の目的コード ==========================================================
#[derive(Clone, Copy)]


enum Op {
    Lit(i64), Load(u32), Store(u32), Dup, Pop,
    Add, Sub, Eq, NilV, Cons, Car, Cdr, IsPair,
    Jz(u32), Jmp(u32),
}

#[derive(Clone, Copy)]
enum Bind { Slot(u32), Boxed(u32), Fn(u32) }
//   Boxed = box をスカラへ置換した(escape しない時だけ成立)
//   Fn    = 閉包を **呼び先が静的に一つに定まる時だけ** 束ねた(値として使われたら escape ⇒ 断る)。
//           コードも slot も出さない —— 適用のたびに body を inline する。

/// ⚠️ `fns` の三つ目は **定義時の env の長さ**。閉包は *定義した場所* の束縛を捕まえる ——
///   inline は「body をここへ貼る」ことなので、貼った先の env で名前を解くと **別の値**を読む。
///   🔴 実測 2026-09-06: `let a=5; let f=ofn(q,q+a); let a=90; f(1)` が
///     床D では 6、段E/段F/段G′ では **91**。落ちない。手で置いた probe は影を作らないので当たらなかった。
struct Comp { code: Vec<Op>, env: Vec<(i64, Bind)>, nslots: u32, fns: Vec<(i64, Json, usize)>, depth: u32 }

impl Comp {
    fn new() -> Comp { Comp { code: Vec::new(), env: Vec::new(), nslots: 0, fns: Vec::new(), depth: 0 } }
    fn slot(&mut self) -> u32 { let s = self.nslots; self.nslots += 1; s }
    fn look(&self, id: i64) -> Option<Bind> {
        self.env.iter().rev().find(|(i, _)| *i == id).map(|(_, b)| *b)
    }
    fn here(&self) -> u32 { self.code.len() as u32 }

    /// let の共通処理 —— 値が newbox ならスカラへ置換して束縛、そうでなければ普通のスロット。
    fn bind_let(&mut self, arg: &Json) -> Result<(i64, &'static str), String> {
        let id = pnum(pcar(arg));
        let rest = pcdr(arg);
        let val = pcar(rest);
        if is_p(val) && pnum(pcar(val)) == ntag::OFN {        // ofn —— 閉包。コードも slot も出さない
            let param = pnum(pcar(pcdr(val)));
            let body = pcdr(pcdr(val)).clone();
            self.fns.push((param, body, self.env.len()));   // ← **定義時の env** を控える
            self.env.push((id, Bind::Fn((self.fns.len() - 1) as u32)));
            return Ok((id, "fn"));
        }
        let s = self.slot();
        if is_p(val) && pnum(pcar(val)) == ntag::NEWBOX {
            self.emit_val(pcdr(val))?;                 // newbox の初期値
            self.code.push(Op::Store(s));
            self.env.push((id, Bind::Boxed(s)));
            Ok((id, "box"))
        } else {
            self.emit_val(val)?;
            self.code.push(Op::Store(s));
            self.env.push((id, Bind::Slot(s)));
            Ok((id, "val"))
        }
    }

    /// 呼び先が静的に一つに定まる適用だけを inline する。定まらなければ断る(= escape)。
    fn inline_call(&mut self, arg: &Json) -> Result<(), String> {
        let f = pcar(arg);
        if !is_p(f) || pnum(pcar(f)) != ntag::VAR { return Err("呼び先が var でない(閉包が escape)".into()); }
        let idx = match self.look(pnum(pcdr(f))) {
            Some(Bind::Fn(i)) => i,
            _ => return Err("呼び先が静的に定まらない(閉包が escape)".into()),
        };
        if self.depth >= 8 { return Err("inline が深すぎる(再帰閉包か)".into()); }
        let (param, body, elen) = self.fns[idx as usize].clone();
        self.emit_val(pcdr(arg))?;                     // 実引数(**呼び側の env** で評価する)
        let s = self.slot();
        self.code.push(Op::Store(s));
        // body は **定義時の env** で解く ⇒ 呼び側の束縛を一旦外し、終わったら戻す
        let 呼び側 = self.env.split_off(elen);
        self.env.push((param, Bind::Slot(s)));
        self.depth += 1;
        let r = self.emit_val(&body);
        self.env.truncate(elen);        // ← param もここで落ちる
        self.env.extend(呼び側);
        self.depth -= 1;
        // ⚠️ かつてここに `self.env.pop()` が在った（param を落とすため）。
        //   復元を足した後も残っていて、**呼び側の束縛を一つ食っていた**。
        //   実測 2026-09-06: 直した 10 分後に差分ファズが 60 本で掴んだ ——
        //   `let a=57; let f=…; let a=331; if(f(a), a, …)` が then 枝で古い a を返した。
        //   ◆ 型: **後始末を足したら、元の後始末を外したか見る。** 二重に片付けると、隣を壊す。
        r
    }

    fn box_slot(&self, h: &Json) -> Result<u32, String> {
        if !is_p(h) || pnum(pcar(h)) != ntag::VAR { return Err("box handle が var でない(escape)".into()); }
        match self.look(pnum(pcdr(h))) {
            Some(Bind::Boxed(s)) => Ok(s),
            _ => Err("box handle が静的に解けない(escape)".into()),
        }
    }

    /// 値を一つだけスタックへ残す。
    fn emit_val(&mut self, n: &Json) -> Result<(), String> {
        if !is_p(n) { return Err(format!("object 節でない")); }
        let tag = pnum(pcar(n));
        let arg = pcdr(n);
        match tag {
            ntag::LIT => self.code.push(Op::Lit(pnum(arg))),
            ntag::ADD | ntag::SUB => { self.emit_val(pcar(arg))?; self.emit_val(pcdr(arg))?;
                       self.code.push(if tag == ntag::ADD { Op::Add } else { Op::Sub }); }
            ntag::EQ => { self.emit_val(pcar(arg))?; self.emit_val(pcdr(arg))?; self.code.push(Op::Eq); }
            ntag::NIL => self.code.push(Op::NilV),
            ntag::CONS => { self.emit_val(pcar(arg))?; self.emit_val(pcdr(arg))?; self.code.push(Op::Cons); }
            ntag::CAR | ntag::CDR | ntag::PAIRP => { self.emit_val(arg)?;                                              // car / cdr / pair?
                             self.code.push(match tag { 9 => Op::Car, 10 => Op::Cdr, _ => Op::IsPair }); }
            ntag::VAR => match self.look(pnum(arg)) {
                Some(Bind::Slot(s)) => self.code.push(Op::Load(s)),
                Some(Bind::Boxed(_)) => return Err("box handle を値として使った(escape)".into()),
                Some(Bind::Fn(_)) => return Err("閉包を値として使った(escape)".into()),
                None => return Err(format!("未束縛の var id {}", pnum(arg))),
            },
            ntag::OAPP => self.inline_call(arg)?,                                                  // oapp
            ntag::OFN => return Err("閉包が let の外(名前が付かない ⇒ 呼び先が定まらない)".into()),
            ntag::LET => { self.bind_let(arg)?; self.emit_val(pcdr(pcdr(arg)))?; self.env.pop(); }
            ntag::IF => { // if(c,(t,f))
                self.emit_val(pcar(arg))?;
                let jf = self.here(); self.code.push(Op::Jz(0));
                self.emit_val(pcar(pcdr(arg)))?;
                let je = self.here(); self.code.push(Op::Jmp(0));
                self.code[jf as usize] = Op::Jz(self.here());
                self.emit_val(pcdr(pcdr(arg)))?;
                self.code[je as usize] = Op::Jmp(self.here());
            }
            ntag::GETBOX => { let s = self.box_slot(arg)?; self.code.push(Op::Load(s)); }
            ntag::SETBOX => { let s = self.box_slot(pcar(arg))?;      // setbox は書いた値を返す(kokkos.py:1484)
                    self.emit_val(pcdr(arg))?; self.code.push(Op::Dup); self.code.push(Op::Store(s)); }
            ntag::WHILE => { // owhile —— 返り値は反復回数(kokkos.py:156)
                let it = self.slot();
                self.code.push(Op::Lit(0)); self.code.push(Op::Store(it));
                let top = self.here();
                self.emit_val(pcar(arg))?;
                let jf = self.here(); self.code.push(Op::Jz(0));
                self.emit_eff(pcdr(arg))?;                 // body の値は捨てられる = 効果位置
                self.code.push(Op::Load(it)); self.code.push(Op::Lit(1));
                self.code.push(Op::Add); self.code.push(Op::Store(it));
                self.code.push(Op::Jmp(top));
                self.code[jf as usize] = Op::Jz(self.here());
                self.code.push(Op::Load(it));
            }
            ntag::NEWBOX => return Err("newbox が let の外(スカラ置換できない)".into()),
            t => return Err(format!("この切片は tag {} を持たない", t)),
        }
        Ok(())
    }

    /// 効果だけ要る位置(値は捨てられる)。cons は *列* として使われているので確保を出さない。
    fn emit_eff(&mut self, n: &Json) -> Result<(), String> {
        if !is_p(n) { return Err("object 節でない".into()); }
        let tag = pnum(pcar(n));
        let arg = pcdr(n);
        match tag {
            ntag::CONS => { self.emit_eff(pcar(arg))?; self.emit_eff(pcdr(arg))?; }   // 死んだ確保を出さない
            ntag::LET => { self.bind_let(arg)?; self.emit_eff(pcdr(pcdr(arg)))?; self.env.pop(); }
            ntag::SETBOX => { let s = self.box_slot(pcar(arg))?; self.emit_val(pcdr(arg))?; self.code.push(Op::Store(s)); }
            _ => { self.emit_val(n)?; self.code.push(Op::Pop); }
        }
        Ok(())
    }
}

// ---- 段E: 特殊化した命令列をそのまま回す(値は床C/D と同じ Val + arena)-------------
fn run_e(code: &[Op], nslots: u32, ar: &mut Arena, count: &mut u64) -> Val {
    let mut sl: Vec<Val> = vec![Val::I(0); nslots as usize];
    let mut st: Vec<Val> = Vec::with_capacity(32);
    let mut pc = 0usize;
    while pc < code.len() {
        *count += 1;
        match code[pc] {
            Op::Lit(k) => st.push(Val::I(k)),
            Op::Load(s) => st.push(sl[s as usize]),
            Op::Store(s) => { sl[s as usize] = st.pop().unwrap(); }
            Op::Dup => { let v = *st.last().unwrap(); st.push(v); }
            Op::Pop => { st.pop(); }
            Op::Add => { let b = st.pop().unwrap(); let a = st.pop().unwrap();
                         match (a, b) { (Val::I(x), Val::I(y)) => st.push(Val::I(x + y)), _ => panic!("add non-int") } }
            Op::Sub => { let b = st.pop().unwrap(); let a = st.pop().unwrap();
                         match (a, b) { (Val::I(x), Val::I(y)) => st.push(Val::I(x - y)), _ => panic!("sub non-int") } }
            Op::Eq => { let b = st.pop().unwrap(); let a = st.pop().unwrap();
                        st.push(Val::I(if eq_v(ar, a, b) { 1 } else { 0 })) }
            Op::NilV => st.push(Val::Nil),
            Op::Cons => { let d = st.pop().unwrap(); let a = st.pop().unwrap(); let p = cons_a(ar, a, d); st.push(p) }
            Op::Car => { match st.pop().unwrap() { Val::P(i) => st.push(ar[i as usize].0), _ => panic!("car non-pair") } }
            Op::Cdr => { match st.pop().unwrap() { Val::P(i) => st.push(ar[i as usize].1), _ => panic!("cdr non-pair") } }
            Op::IsPair => { let v = st.pop().unwrap(); st.push(Val::I(if matches!(v, Val::P(_)) { 1 } else { 0 })) }
            Op::Jz(t) => { let v = st.pop().unwrap(); if !truthy_v(v) { pc = t as usize; continue; } }
            Op::Jmp(t) => { pc = t as usize; continue; }
        }
        pc += 1;
    }
    st.pop().unwrap_or(Val::I(0))
}

// ---- 段F: x86-64 を直に吐く(アセンブラも LLVM も通さない)------------------------
// System V: 第一引数 = rdi(スロット配列の先頭)、返り値 = rax。オペランドスタックは *本物の* rsp。
// 整数中核だけを受ける —— Cons / NilV が出たら断る(値の形が静的に int と分かる範囲、§所有)。
extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    fn mprotect(addr: *mut u8, len: usize, prot: i32) -> i32;
}
const PROT_READ: i32 = 1; const PROT_WRITE: i32 = 2; const PROT_EXEC: i32 = 4;
const MAP_PRIVATE: i32 = 0x02; const MAP_ANON: i32 = 0x20;

/// 段F の **一段の register 割付**(top-of-stack caching、2026-09-06)。
///
/// それまでの段F は素朴なスタック機械の直訳で、値が動くたびに `push`/`pop` を挟んでいた。
/// ここでは「オペランドスタックの **頂だけ** を `rax` に載せたまま持ち歩く」——
/// 一段しか持たないが、`push` して次の命令が即 `pop` する形が支配的なので、これで大半が消える。
///
/// ⚠️ **落とし穴は分岐**。ラベルに複数の道から入ってくるので、入口で「頂が rax に在るか」が
///   食い違うと *静かに間違える*。⇒ **ラベルの手前で必ず吐き出す**（規約: ラベル入口は常に「載っていない」）。
///   ⚠️ 吐き出す命令は **ラベルより前** に置く。ラベルの中に置くと、飛んできた側がそれを実行する
///     （rax にはまだ何も無い）⇒ だから `start`(吐き出しの位置)と `label`(飛び先)を別に持つ。
#[derive(Clone, Copy, PartialEq)]
struct Tos(bool);          // 頂が rax に載っているか

/// 命令一つの長さを、**入口の状態込み**で返す。⚠️ 状態で長さが変わるので、
/// 位置を決める周と吐く周で **同じ状態列**を辿らなければならない（`plan` が一度だけ作る）。
/// 段F の **本物の register 割付**(2026-09-06)。
///
/// 頂だけを持ち歩く一段の割付では 1.37x しか出なかった。梯子が毎回印字していた「まだ数倍」の
/// 当て先は **局所変数**で、Load / Store のたびに `rdi` 経由でメモリを往復していた。
/// ⇒ よく使うスロットを **物理 register に固定**する。7 byte の往復が 3 byte の move になる。
///
/// ⚠️ 使ってよい register の根拠: **この関数は何も呼ばない** ⇒ caller-saved は全部自由。
///   rax は頂、rcx は作業、rdi は引数（固定しなかったスロットの置き場）。残りを固定に回す。
///   callee-saved(rbx/rbp/r12-r15)は **触らない** —— 保存すれば使えるが、保存を忘れた日に
///   *呼び元が壊れる*。得より、忘れうる手を置かない方を採る。
/// ⚠️ 固定した register は **入口で 0 にする**。呼び元はスロット配列を 0 で埋めてから渡すが、
///   register には何も入っていない ⇒ 書く前に読むと garbage を返す（静かに間違える形）。
const PIN: [u8; 6] = [2, 6, 8, 9, 10, 11];      // rdx, rsi, r8..r11

/// どのスロットを register に載せるか。使用回数の多い順に PIN の分だけ。
fn pin_map(code: &[Op]) -> HashMap<u32, u8> {
    let mut 数: HashMap<u32, usize> = HashMap::new();
    for op in code {
        if let Op::Load(s) | Op::Store(s) = op { *数.entry(*s).or_insert(0) += 1; }
    }
    let mut v: Vec<(u32, usize)> = 数.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));   // ⚠️ 同数は slot 番号で決める（決定的に）
    v.into_iter().take(PIN.len()).enumerate().map(|(i, (s, _))| (s, PIN[i])).collect()
}

fn rex(w_r: bool, r: u8) -> u8 { 0x48 | if r >= 8 { if w_r { 0x04 } else { 0x01 } } else { 0 } }
/// mov <reg>, rax
fn mov_r_rax(r: u8, b: &mut Vec<u8>) { b.push(rex(false, r)); b.push(0x89); b.push(0xC0 | (r & 7)); }
/// mov rax, <reg>
fn mov_rax_r(r: u8, b: &mut Vec<u8>) { b.push(rex(true, r)); b.push(0x89); b.push(0xC0 | ((r & 7) << 3)); }
/// xor <reg>, <reg>
fn xor_rr(r: u8, b: &mut Vec<u8>) {
    b.push(0x48 | if r >= 8 { 0x05 } else { 0 }); b.push(0x31); b.push(0xC0 | ((r & 7) << 3) | (r & 7));
}

/// **二項演算の両辺畳み**(2026-09-06)。`Load a; Lit k; Add` のように、
/// 両辺が「取り出すだけ」の時、間の `push`/`pop` は要らない ——
/// `mov rax,<a>` の後に `add rax, imm` を直に当てられる。
///
/// ⚠️ 当て先は **数えて選んだ**。最初は `Jz`/`Eq` を絞るつもりだったが、吐いた列を数えたら
///   `Jz` は 1 回、`Eq` は **0 回**。多いのはこの形だった ⇒ **当て先は見立てで選ばない。**
/// ⚠️ 飛び先を跨いで畳まない —— 飛んできた側が畳んだ命令の途中に落ちる。
fn 畳める(code: &[Op], i: usize, targets: &std::collections::HashSet<usize>) -> bool {
    if i + 2 >= code.len() { return false; }
    if targets.contains(&(i + 1)) || targets.contains(&(i + 2)) { return false; }
    if !matches!(code[i], Op::Load(_)) { return false; }
    if !matches!(code[i + 2], Op::Add | Op::Sub) { return false; }
    match &code[i + 1] {
        Op::Load(_) => true,
        Op::Lit(k) => *k >= i32::MIN as i64 && *k <= i32::MAX as i64,
        _ => false,
    }
}

/// 命令一つ（または畳んだ三つ組）を吐く。**長さはここから *出る* もので、別表を持たない。**
///
/// 🔴 かつて `op_len` という **長さの表**を別に持っていた。2026-09-06、畳みを足した時に
///   `add rax, imm32` を 5 byte と書いた（実際は 6）—— 表と実体が一 byte ずれ、
///   以降の飛び先が全部ずれて **segfault と無限ループ**になった。
///   ◆ **宣言を使用から導く**（同日、wasm の memory 節で踏んだのと同じ型）。
///   ⇒ 位置を決める周も、吐く周も、**この一つの口**を通す。表が無ければ、ずれようがない。
/// ⚠️ 飛び先の値だけは周で変わるが、`rel32` は長さが固定なので **長さは変わらない**。
fn emit_one(b: &mut Vec<u8>, code: &[Op], i: usize, t: Tos, pins: &HashMap<u32, u8>,
            fuse: &[u8], label: &[usize], here: usize) -> Result<(), String> {
    let d32 = |v: i32| v.to_le_bytes();
    let 取る = |b: &mut Vec<u8>, s: &u32| match pins.get(s) {          // mov rax, <slot>
        Some(&r) => mov_rax_r(r, b),
        None => { b.extend_from_slice(&[0x48, 0x8B, 0x87]); b.extend_from_slice(&d32((*s as i32) * 8)); }
    };
    if fuse[i] == 2 { return Ok(()); }                                 // 前の畳みが呑んだ
    if fuse[i] == 1 {
        if t.0 { b.push(0x50); }                                       // 前の頂を落とす
        if let Op::Load(a) = &code[i] { 取る(b, a); }
        let 加 = matches!(code[i + 2], Op::Add);
        match &code[i + 1] {
            Op::Lit(k) => { b.extend_from_slice(&[0x48, if 加 { 0x05 } else { 0x2D }]);
                            b.extend_from_slice(&d32(*k as i32)); }              // add/sub rax, imm32
            Op::Load(s) => match pins.get(s) {
                Some(&r) => { b.push(0x48 | if r >= 8 { 0x04 } else { 0 });
                              b.push(if 加 { 0x01 } else { 0x29 });
                              b.push(0xC0 | ((r & 7) << 3)); }                   // add/sub rax, reg
                None => { b.extend_from_slice(&[0x48, if 加 { 0x03 } else { 0x2B }, 0x87]);
                          b.extend_from_slice(&d32((*s as i32) * 8)); }          // add/sub rax, [rdi+d]
            },
            _ => {}
        }
        return Ok(());
    }
    let load = |b: &mut Vec<u8>| if !t.0 { b.push(0x58); };                      // pop rax
    match &code[i] {
        Op::Lit(k) => {
            if t.0 { b.push(0x50); }
            if *k >= i32::MIN as i64 && *k <= i32::MAX as i64 {
                b.extend_from_slice(&[0x48, 0xC7, 0xC0]); b.extend_from_slice(&d32(*k as i32));
            } else {
                b.extend_from_slice(&[0x48, 0xB8]); b.extend_from_slice(&k.to_le_bytes());
            }
        }
        Op::Load(s) => { if t.0 { b.push(0x50); } 取る(b, s); }
        Op::Store(s) => {
            load(b);
            match pins.get(s) {
                Some(&r) => mov_r_rax(r, b),
                None => { b.extend_from_slice(&[0x48, 0x89, 0x87]); b.extend_from_slice(&d32((*s as i32) * 8)); }
            }
        }
        Op::Dup => { load(b); b.push(0x50); }
        Op::Pop => { if !t.0 { b.extend_from_slice(&[0x48, 0x83, 0xC4, 0x08]); } }
        Op::Add => { load(b); b.push(0x59); b.extend_from_slice(&[0x48, 0x01, 0xC8]); }
        Op::Sub => { load(b); b.extend_from_slice(&[0x48, 0x89, 0xC1]); b.push(0x58);
                     b.extend_from_slice(&[0x48, 0x29, 0xC8]); }
        Op::Eq  => { load(b); b.push(0x59);
                     b.extend_from_slice(&[0x48, 0x39, 0xC8, 0x0F, 0x94, 0xC0, 0x48, 0x0F, 0xB6, 0xC0]); }
        Op::Jz(tgt) => {
            load(b);
            b.extend_from_slice(&[0x48, 0x85, 0xC0, 0x0F, 0x84]);
            let to = label[*tgt as usize] as i64;
            b.extend_from_slice(&d32(0)); let n = b.len();
            let 末 = here + n;                                                    // この命令の終端
            let rel = to - 末 as i64;
            b[n - 4..].copy_from_slice(&d32(rel as i32));
        }
        Op::Jmp(tgt) => {
            if t.0 { b.push(0x50); }
            b.push(0xE9);
            let to = label[*tgt as usize] as i64;
            b.extend_from_slice(&d32(0)); let n = b.len();
            let 末 = here + n;
            let rel = to - 末 as i64;
            b[n - 4..].copy_from_slice(&d32(rel as i32));
        }
        Op::NilV | Op::Cons | Op::Car | Op::Cdr | Op::IsPair =>
            return Err("段F は整数中核のみ(pair が要る)".into()),
    }
    Ok(())
}

fn next_tos(op: &Op) -> Tos {
    Tos(match op {
        Op::Lit(_) | Op::Load(_) | Op::Dup | Op::Add | Op::Sub | Op::Eq => true,
        Op::Store(_) | Op::Pop | Op::Jz(_) | Op::Jmp(_) => false,
        _ => false,
    })
}

fn jit(code: &[Op]) -> Result<Vec<u8>, String> {
    // 0 周目: 飛び先を集める。⚠️ 末尾(= epilogue)も飛び先になりうる。
    let mut targets = std::collections::HashSet::new();
    for op in code {
        match op { Op::Jz(t) | Op::Jmp(t) => { targets.insert(*t as usize); }, _ => {} }
    }
    let pins = pin_map(code);
    let 前口 = 3 * pins.len();
    let n = code.len();
    let mut fuse = vec![0u8; n];
    let mut i = 0usize;
    while i < n {
        if 畳める(code, i, &targets) { fuse[i] = 1; fuse[i+1] = 2; fuse[i+2] = 2; i += 3; } else { i += 1; }
    }
    // 1 周目 / 2 周目 —— **同じ口で吐く**。1 周目は飛び先が仮（長さは同じ）。
    let mut label = vec![0usize; n + 1];
    let mut start = vec![0usize; n + 1];
    let mut tos_in = vec![Tos(false); n + 1];
    let mut out: Vec<u8> = Vec::new();
    for 周 in 0..2 {
        out = Vec::with_capacity(256);
        let mut 順: Vec<u8> = pins.values().cloned().collect();
        順.sort();
        for r in 順 { xor_rr(r, &mut out); }
        debug_assert_eq!(out.len(), 前口);
        let mut tos = Tos(false);
        for i in 0..n {
            start[i] = out.len();
            if tos.0 && targets.contains(&i) { out.push(0x50); tos = Tos(false); }
            label[i] = out.len();
            tos_in[i] = tos;
            let h = out.len();
            let mut piece = Vec::new();
            emit_one(&mut piece, code, i, tos, &pins, &fuse, &label, h)?;
            out.extend_from_slice(&piece);
            tos = match fuse[i] { 1 => Tos(true), 2 => tos, _ => next_tos(&code[i]) };
        }
        start[n] = out.len();
        if tos.0 && targets.contains(&n) { out.push(0x50); tos = Tos(false); }
        label[n] = out.len();
        tos_in[n] = tos;
        if !tos.0 { out.push(0x58); }
        out.push(0xC3);
        let _ = 周;
    }
    Ok(out)
}

struct Jitted { page: *mut u8 }
impl Jitted {
    fn new(bytes: &[u8]) -> Jitted {
        let len = (bytes.len() + 4095) & !4095;
        unsafe {
            let p = mmap(std::ptr::null_mut(), len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
            assert!(p as isize != -1, "mmap 失敗");
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
            assert!(mprotect(p, len, PROT_READ | PROT_EXEC) == 0, "mprotect 失敗");  // W^X: 書いてから実行可へ
            Jitted { page: p }
        }
    }
    fn call(&self, slots: &mut [i64]) -> i64 {
        let f: extern "C" fn(*mut i64) -> i64 = unsafe { std::mem::transmute(self.page) };
        f(slots.as_mut_ptr())
    }
}


// ==== 段G: wasm を吐いて自己完結 HTML に畳む(出力先は backend の選択肢)==============
// 段F(x86-64)の兄弟。同じ束縛規則(env→スロット / box→スカラ / 死んだ cons を出さない)で、
// 制御流れだけ **構造化**(wasm には任意 jump が無い ⇒ block / loop / br_if)。
// ⚠️ 束縛規則は Comp と同じものを二度書いている。**ズレは秤が捕まえる**(同じ答えを出すまで進めない)。
// 外部 toolchain ゼロ —— wat2wasm も LLVM も Emscripten も通さず、module の byte を直に組む。

fn uleb(v: u32, out: &mut Vec<u8>) {
    let mut v = v;
    loop { let mut b = (v & 0x7f) as u8; v >>= 7; if v != 0 { b |= 0x80; } out.push(b); if v == 0 { break; } }
}
fn sleb(mut v: i64, out: &mut Vec<u8>) {
    loop {
        let b = (v & 0x7f) as u8; v >>= 7;
        let sign = b & 0x40 != 0;
        if (v == 0 && !sign) || (v == -1 && sign) { out.push(b); break; }
        out.push(b | 0x80);
    }
}

/// 段H —— 自由変数の行き先。Mem = host(線形メモリ)が持つ状態 / Param = step の引数(event)。
#[derive(Clone, Copy)]
enum Where { Mem(u32), Param(u32) }

/// 段G″ —— **一意所有**の判定。box `t` の持つ list が、他所へ複製されていないか。
///
/// 許す文脈は二つだけ: ① `car`/`cdr`/`pair?` の引数(覗くだけ)/ ② `cons` の **cdr 位置**
/// (= 新しいセルへ *移す*。spine が伸びるだけで別名は増えない)。
/// それ以外の場所に `getbox(t)` が現れたら —— `let ys = getbox(xs)` でも、返り値でも ——
/// **所有を主張しない**(安全側に倒す)。知らない節が在った時も false。
fn scan_owned(n: &Json, t: i64, ctx: u8) -> bool {   // ctx: 0=その他 / 1=覗く / 2=cons の cdr
    if !is_p(n) { return true; }
    let tag = pnum(pcar(n));
    let a = pcdr(n);
    match tag {
        ntag::GETBOX => {                                                    // getbox
            if is_p(a) && pnum(pcar(a)) == ntag::VAR && pnum(pcdr(a)) == t { return ctx != 0; }
            scan_owned(a, t, 0)
        }
        ntag::CAR | ntag::CDR | ntag::PAIRP => scan_owned(a, t, 1),                         // car / cdr / pair?
        ntag::CONS => scan_owned(pcar(a), t, 0) && scan_owned(pcdr(a), t, 2), // cons —— cdr 位置だけ移動を許す
        ntag::ADD | ntag::SUB | ntag::EQ | ntag::OAPP | ntag::SETBOX | ntag::WHILE => scan_owned(pcar(a), t, 0) && scan_owned(pcdr(a), t, 0),
        ntag::LET => scan_owned(pcar(pcdr(a)), t, 0) && scan_owned(pcdr(pcdr(a)), t, 0),
        ntag::IF => scan_owned(pcar(a), t, 0) && scan_owned(pcar(pcdr(a)), t, 0) && scan_owned(pcdr(pcdr(a)), t, 0),
        // ⚠️ `ofn` は (16,(param, body)) —— body は **pcdr(a)**。
        //   ここは `let`(4,(id,(val,body))) の形を写し間違えて `pcdr(pcdr(a))` になっていた。
        //   実測 2026-09-06: 「list の箱」と「閉包」を **同時に**持つ程式で panic（num でない）。
        //   probe は片方ずつしか持っていなかったので、一度も当たらなかった。
        //   ◆ 型: 似た形を写す時ほど、**一段の深さ**を確かめる。
        ntag::OFN => scan_owned(pcdr(a), t, 0),
        ntag::NEWBOX => scan_owned(a, t, 0),
        ntag::LIT | ntag::VAR | ntag::NIL => true,
        _ => false,
    }
}

/// 段G′ —— 値の *形* を静的に推す。これが在れば pair も register / 線形メモリに置ける。
/// ⚠️ 扱うのは Int と「int の list」だけ。入れ子 list や混在は **断る**(黙って通さない)。
#[derive(Clone, Copy, PartialEq)]
enum Kind { Int, List, Str }

fn kname(k: Kind) -> &'static str { match k { Kind::Int => "int", Kind::List => "list", Kind::Str => "str" } }

// ⚠️ ここに `listish`(= list を要る所は str も受ける)が在った。**一度も呼ばれていなかった**。
//    受け入れの実体は `want` の中に埋まっていて、注釈だけがここで「設計」を名乗っていた。
//    ⇒ 2026-09-05、注釈の言う通りに受け入れていた `want` ごと外した。理由は `want` の上に書く。
//    ◆ str は「小さい整数の list」で **意味は同じ**。だが **置き方が違う** ⇒ 意味が同じことは、
//      語として入れ替えてよい理由にならない。契約(op / machine.json)は一文字も変わらない。

struct WComp {
    /// 詰めた文字列を **作らない**（`if` の枝で形が割れた時だけ立てる。下の IF を見よ）。
    no_intern: bool,
    b: Vec<u8>, env: Vec<(i64, Bind)>, nslots: u32, outer: Vec<(i64, Where)>,
    fns: Vec<(i64, Json, usize)>, depth: u32,   // 三つ目 = **定義時の env の長さ**(閉包の捕捉)
    kinds: HashMap<u32, Kind>,        // スロット → 値の形(静的に推した)
    heap: Option<(u32, u32, u32)>,    // (hp, ta, td)—— 使った時だけ確保する控え
    own: u8,                          // 段G″: 0=回収しない / 1=セル単位で返す / 2=**まとめて捨てる**(region)
    peak: bool,                       // 峰の計器を積むか。⚠️ 計器自体が命令数を動かす ⇒ 費用を測る走行では外す
    regions: Vec<(u32, u32)>,         // (hp の控え slot, 所有 box slot)—— region の入れ子
    foreign_list_write: bool,         // region の中から *外の* list を書いたか(書いたら捨てられない)
    root: Option<Json>,               // 所有判定は全プログラムを見る必要がある
    owned: HashMap<u32, i64>,         // 一意所有と判定した box スロット → その var id
    pool: Vec<u8>,                    // リテラル文字列の実体。線形メモリの 16 番地から置く
    tstr: Option<u32>,                // str を二度使う時の控え
    mem: bool,                        // 線形メモリを触ったか —— module に memory を宣言するかを決める唯一の根拠
}

impl WComp {
    fn new() -> WComp {
        WComp { no_intern: false, b: Vec::new(), env: Vec::new(), nslots: 0, outer: Vec::new(),
                fns: Vec::new(), depth: 0, kinds: HashMap::new(), heap: None,
                own: 0, peak: true, root: None, owned: HashMap::new(),
                regions: Vec::new(), foreign_list_write: false, pool: Vec::new(), tstr: None,
                mem: false }
    }
    fn slot(&mut self) -> u32 { let s = self.nslots; self.nslots += 1; s }
    fn look(&self, id: i64) -> Option<Bind> { self.env.iter().rev().find(|(i, _)| *i == id).map(|(_, b)| *b) }
    fn op(&mut self, o: u8) { self.b.push(o); }
    fn op_u(&mut self, o: u8, x: u32) { self.b.push(o); uleb(x, &mut self.b); }
    // ⚠️ ここに `u`(= 生の uleb を b へ足す)が在った。memop へ通した結果 **誰も呼ばなくなった**。
    //    今日の三つ目の死んだ口。⇒ **warning 0 で建てる**を条件にしたので、その場で鳴った。
    /// 線形メモリを触る命令は **必ずここを通す**。
    /// ⚠️ 触る所と「memory を宣言する所」が離れていると、宣言し忘れても *静かに通る* ——
    ///   実測 2026-09-06: `car` だけして `cons` しない程式（箱が nil で while が一度も回らない）で、
    ///   梯子は「畳めた」と報告しつつ **instantiate できない wasm** を書いていた。
    ///   ◆ 宣言は *使用から導く*。並べて置くと、片方だけ足し忘れる。
    fn memop(&mut self, o: u8, align: u8, off: u32) {
        self.mem = true;
        self.b.push(o); self.b.push(align); uleb(off, &mut self.b);
    }

    /// pair 用の bump heap を使う —— 控えの local を一度だけ確保する。
    fn heap_locals(&mut self) -> (u32, u32, u32) {
        if let Some(h) = self.heap { return h; }
        let h = (self.slot(), self.slot(), self.slot());
        self.heap = Some(h); h
    }
    /// 本体の頭に hp の初期化を差す(heap を使った時だけ)。nil = 番地 0 なので heap は 16 から。
    fn finish(&mut self) {
        if let Some((hp, _, _)) = self.heap {
            let mut pre = Vec::new();
            let base = 16 + ((self.pool.len() as i64 + 15) / 16) * 16;   // 池の後ろ、16 境界に揃える
            pre.push(0x42); sleb(base, &mut pre);        // i64.const <heap の始まり>
            pre.push(0x21); uleb(hp, &mut pre);          // local.set $hp
            pre.extend_from_slice(&self.b);
            self.b = pre;
        }
    }
    fn slot_kind(&self, s: u32) -> Kind { *self.kinds.get(&s).unwrap_or(&Kind::Int) }
    /// ⚠️ **str を list の代わりに通さない**(2026-09-05 実測で外した)。
    /// str は「小さい整数の list」だが、**置き方が違う**(番地<<32|長さ の一語。セルではない)。
    /// ⇒ *形を見て分ける* 所(car/cdr/pair?)は str を自分で捌くので、ここを通らない。
    ///   *語として仕舞う* 所(cons の cdr / list スロットへの setbox)で通すと、
    ///   仕舞った先は「list」と静的に信じたまま **詰めた語を番地として読む** ⇒ 静かに間違える。
    ///   実測: 前置も setbox も 115 の所で **0 を返した**(落ちもしなかった)。
    fn want(k: Kind, got: Kind, what: &str) -> Result<(), String> {
        if k == got { return Ok(()); }
        let 訳 = if k == Kind::List && got == Kind::Str {
            " —— str は詰めた一語(番地<<32|長さ)。list の語として仕舞えない [REFUSE str-as-list]"
        } else { "(形が静的に決まらない)" };
        Err(format!("{} は {} を要るが {} が来た{}", what, kname(k), kname(got), 訳))
    }

    fn bind_let(&mut self, arg: &Json) -> Result<(), String> {
        let id = pnum(pcar(arg));
        let val = pcar(pcdr(arg));
        if is_p(val) && pnum(pcar(val)) == ntag::OFN {                    // ofn —— 閉包(コードも slot も出さない)
            let param = pnum(pcar(pcdr(val)));
            self.fns.push((param, pcdr(pcdr(val)).clone(), self.env.len()));   // ← 定義時の env
            self.env.push((id, Bind::Fn((self.fns.len() - 1) as u32)));
            return Ok(());
        }
        let boxed = is_p(val) && pnum(pcar(val)) == ntag::NEWBOX;
        let k = self.emit_val(if boxed { pcdr(val) } else { val })?;
        let s = self.slot();
        self.op_u(0x21, s);
        self.kinds.insert(s, k);
        if self.own > 0 && boxed && k == Kind::List {
            if let Some(r) = self.root.clone() {
                if scan_owned(&r, id, 0) { self.owned.insert(s, id); }
            }
        }
        self.env.push((id, if boxed { Bind::Boxed(s) } else { Bind::Slot(s) }));
        // 段G″(region): 所有が言えた list box の束縛で **その時の hp を控える**。
        // 束縛の scope を出る時にそこへ戻せば、中で積んだ物を *一命令で* 全部捨てられる。
        if self.own == 2 && boxed && k == Kind::List && self.owned.contains_key(&s) {
            let (hp, _, _) = self.heap_locals();
            let mark = self.slot();
            self.op_u(0x20, hp); self.op_u(0x21, mark);
            self.regions.push((mark, s));
        }
        Ok(())
    }
    /// 束縛の scope を出る。region を開いていたなら、条件が揃った時だけ hp を戻す。
    /// 条件: ① 中から *外の* list を書いていない ② 値として list を持ち出していない。
    /// ⚠️ 揃わなければ **戻さない**(安全側に倒す。捨て損なうだけで壊れはしない)。
    fn close_scope(&mut self, carries_list_out: bool) {
        let mine = match (self.regions.last(), self.env.last()) {
            (Some((_, bs)), Some((_, Bind::Boxed(x)))) => x == bs,
            _ => false,
        };
        if mine {
            let (mark, _) = self.regions.pop().unwrap();
            if !self.foreign_list_write && !carries_list_out {
                let (hp, _, _) = self.heap_locals();
                self.op_u(0x20, mark); self.op_u(0x21, hp);      // hp を控えへ戻す = まとめて捨てる
            }
        }
        self.env.pop();
    }

    fn box_slot(&self, h: &Json) -> Result<u32, String> {
        if !is_p(h) || pnum(pcar(h)) != ntag::VAR { return Err("box handle が var でない(escape)".into()); }
        match self.look(pnum(pcdr(h))) { Some(Bind::Boxed(s)) => Ok(s), _ => Err("box handle が静的に解けない".into()) }
    }

    /// 静的な cons の鎖 —— car が 0..255 の lit、cdr が nil か同じ形 —— を byte 列として読む。
    /// ⚠️ これが見抜けなければ「文字列の速い出口」は成立しない(2026-09-05 の予想の難所②)。
    /// 節が **0..255 の即値**なら、その値。詰めた文字列(1 B/要素)へ足せるのはこれだけ。
    fn static_byte(n: &Json) -> Option<i64> {
        if !is_p(n) || pnum(pcar(n)) != ntag::LIT { return None; }
        let v = pnum(pcdr(n));
        if (0..=255).contains(&v) { Some(v) } else { None }
    }

    /// `if` の両枝を吐いて、それぞれの形を返す。
    /// ◆ **口は一つ** —— 巻き戻して吐き直す時も、必ずこの同じ道を通す。
    /// 値位置の `if` の両枝。値は **控えの局所** に仕舞い、block を抜けてから読む。
    /// ⚠️ `else`(0x05)は使わない —— `if`(0x04)と対で、init.zig(exp/03)が持たない側。
    fn emit_both(&mut self, arg: &Json, s: u32) -> Result<(Kind, Kind), String> {
        let t = self.emit_val(pcar(pcdr(arg)))?;
        self.op_u(0x21, s);                                                   // then の値を控えへ
        self.op_u(0x0C, 1);                                                   // br L1(then の後は抜ける)
        self.op(0x0B);                                                        // end L0
        let f = self.emit_val(pcdr(pcdr(arg)))?;
        self.op_u(0x21, s);                                                   // else の値を控えへ
        self.op(0x0B);                                                        // end L1
        Ok((t, f))
    }

    fn as_static_str(n: &Json) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        let mut cur = n;
        loop {
            if !is_p(cur) { return None; }
            match pnum(pcar(cur)) {
                ntag::NIL => return if out.len() >= 2 { Some(out) } else { None },      // nil で閉じた
                ntag::CONS => {
                    let a = pcar(pcdr(cur));
                    if !is_p(a) || pnum(pcar(a)) != ntag::LIT { return None; }          // car が lit でない
                    let v = pnum(pcdr(a));
                    if !(0..=255).contains(&v) { return None; }                 // byte に収まらない
                    out.push(v as u8);
                    cur = pcdr(pcdr(cur));
                }
                _ => return None,
            }
            if out.len() > 65535 { return None; }
        }
    }
    /// 池へ入れて (番地<<32 | 長さ) を返す。同じ文字列は一度だけ置く。
    fn intern(&mut self, bytes: &[u8]) -> i64 {
        let at = match self.pool.windows(bytes.len()).position(|w| w == bytes) {
            Some(i) => i, None => { let i = self.pool.len(); self.pool.extend_from_slice(bytes); i }
        };
        (((16 + at) as i64) << 32) | bytes.len() as i64
    }
    fn tmp_str(&mut self) -> u32 {
        if let Some(t) = self.tstr { return t; }
        let t = self.slot(); self.tstr = Some(t); t
    }

    /// 峰(peak)を memory[0] へ —— 回収の有無を外から見るための計器。
    /// ⚠️ **計器は命令数を動かす**(回収すると hp>peak が偽になり店じまいが省かれる)
    ///    ⇒ 費用を測る走行では `--nopeak` で外す。峰を読む走行と分ける。
    /// ⚠️ 一口にしてある —— 積む所が二つ(cons と str の前置)になった時、片方にだけ足し忘れると
    ///   「回収した」ように見える。**計器は使う所ごとに書かない。**
    fn emit_peak(&mut self, hp: u32) {
        if !self.peak { return; }
        // ⚠️ `if`(0x04)は使わない —— init.zig(exp/03)が持たない(2026-09-04 実測:
        //    「未対応 opcode 0x4 @body」)。block + br_if は持つので、そちらの形で書く。
        self.op(0x02); self.b.push(0x40);                                          // block(void)
        self.op_u(0x20, hp);
        self.op(0x41); self.b.push(0x00); self.memop(0x29, 0x03, 0);
        self.op(0x55);                                                             // i64.gt_s
        self.op(0x45);                                                             // i32.eqz(偽なら抜ける)
        self.op_u(0x0D, 0);                                                        // br_if 0
        self.op(0x41); self.b.push(0x00); self.op_u(0x20, hp);
        self.memop(0x37, 0x03, 0);                                                 // i64.store
        self.op(0x0B);
    }

    /// 段G⁗: **詰めた文字列の前に一文字足す**(2026-09-06)。
    ///
    /// 前日まではここで断っていた —— 「コピーが要るから畳めない」。断りは *正しかった* が、
    /// 断ると **その程式は丸ごと wasm にならない**（前置が一箇所在るだけで段E へ落ちる）。
    /// ⇒ コピーを **書いた**。詰めた形（番地<<32|長さ）のまま、heap に len+1 バイト置き直す。
    ///
    /// 費用: O(len) の byte 複写。cons セルなら O(1) なので **前置を繰り返す程式では不利**。
    /// ⚠️ ただし置き場所は 1 B/文字 のまま ⇒ セルに落とすより **16 分の 1 の場所**で済む。
    ///   どちらが速いかは形による。ここは「畳めるようにした」であって「速くした」ではない。
    ///
    /// ⚠️ 出来た str は **heap を指す**（それまでは data 節だけを指していた）。
    ///   ⇒ region の回収が下から抜けうる ⇒ close_scope / foreign 判定を Str にも広げた。
    ///   これを忘れると、回収した番地を長さ付きで持ち歩く = 静かに間違える。
    ///
    /// 入り: stack 頂に str の一語 / `ta` に足す文字。出: stack に新しい str の一語。
    fn emit_str_prepend(&mut self, hp: u32, ta: u32) {
        // 控えは **site ごと**(二十五段の型。共有すると入れ子の前置で潰れる)
        let (sv, len, src, dst, i) =
            (self.slot(), self.slot(), self.slot(), self.slot(), self.slot());
        self.op_u(0x21, sv);
        self.op_u(0x20, hp); self.op_u(0x21, dst);                                 // dst = hp
        self.op_u(0x20, sv); self.op(0x42); sleb(0xFFFF_FFFFu32 as i64, &mut self.b);
        self.op(0x83); self.op_u(0x21, len);                                       // len = sv & 0xFFFFFFFF
        self.op_u(0x20, sv); self.op(0x42); self.b.push(32); self.op(0x88);
        self.op_u(0x21, src);                                                      // src = sv >>> 32
        self.op_u(0x20, dst); self.op(0xA7); self.op_u(0x20, ta);
        self.memop(0x3C, 0x00, 0);                                                 // mem8[dst] = 足す文字
        self.op(0x42); self.b.push(0x00); self.op_u(0x21, i);                      // i = 0
        self.op(0x02); self.b.push(0x40);                                          // block  ← depth 1
        self.op(0x03); self.b.push(0x40);                                          // loop   ← depth 0
        self.op_u(0x20, i); self.op_u(0x20, len); self.op(0x51);                   // i == len ?
        self.op_u(0x0D, 1);                                                        // br_if 1(抜ける)
        self.op_u(0x20, dst); self.op_u(0x20, i); self.op(0x7C);
        self.op(0x42); self.b.push(0x01); self.op(0x7C); self.op(0xA7);            // dst+1+i
        self.op_u(0x20, src); self.op_u(0x20, i); self.op(0x7C); self.op(0xA7);
        self.memop(0x31, 0x00, 0);                                                 // mem8[src+i]
        self.memop(0x3C, 0x00, 0);                                                 // mem8[dst+1+i] = それ
        self.op_u(0x20, i); self.op(0x42); self.b.push(0x01); self.op(0x7C); self.op_u(0x21, i);
        self.op_u(0x0C, 0);                                                        // br 0
        self.op(0x0B); self.op(0x0B);
        // hp += (len+1) を 16 に切り上げ —— 峰の読みが「16 B 単位」のままになるように揃える
        self.op_u(0x20, hp);
        self.op_u(0x20, len); self.op(0x42); self.b.push(0x10); self.op(0x7C);
        self.op(0x42); sleb(-16, &mut self.b); self.op(0x83);
        self.op(0x7C); self.op_u(0x21, hp);
        self.emit_peak(hp);
        // 値 = (dst<<32) | (len+1)
        self.op_u(0x20, dst); self.op(0x42); self.b.push(32); self.op(0x86);
        self.op_u(0x20, len); self.op(0x42); self.b.push(0x01); self.op(0x7C);
        self.op(0x84);
    }

    fn emit_val(&mut self, n: &Json) -> Result<Kind, String> {
        if !is_p(n) { return Err("object 節でない".into()); }
        let tag = pnum(pcar(n));
        let arg = pcdr(n);
        let k = match tag {
            ntag::LIT => { self.op(0x42); let v = pnum(arg); sleb(v, &mut self.b); Kind::Int }     // i64.const
            ntag::ADD | ntag::SUB => { let a = self.emit_val(pcar(arg))?; let b = self.emit_val(pcdr(arg))?;
                       Self::want(Kind::Int, a, "add/sub")?; Self::want(Kind::Int, b, "add/sub")?;
                       self.op(if tag == ntag::ADD { 0x7C } else { 0x7D }); Kind::Int }
            ntag::EQ => { let a = self.emit_val(pcar(arg))?; let b = self.emit_val(pcdr(arg))?;
                   Self::want(Kind::Int, a, "eq")?; Self::want(Kind::Int, b, "eq")?;
                   self.op(0x51); self.op(0xAD); Kind::Int }                               // i64.eq → extend
            ntag::NIL => { self.op(0x42); self.b.push(0x00); Kind::List }                          // nil = 番地 0
            ntag::CONS if !self.no_intern && Self::as_static_str(n).is_some() => {   // 段G‴: 静的な文字列は **詰めて置く**
                let bytes = Self::as_static_str(n).unwrap();
                let v = self.intern(&bytes);
                self.op(0x42); sleb(v, &mut self.b);                                       // i64.const (番地<<32|長さ)
                Kind::Str
            }
            ntag::CONS => { // cons —— bump heap に 16 B 積む。**これが段G′ の芯**
                let a = self.emit_val(pcar(arg))?; Self::want(Kind::Int, a, "cons の car")?;
                let (hp, _, _) = self.heap_locals();
                // ⚠️ 控えは **cons の site ごとに取る**。一組を全 site で使い回すと、
                //    cdr 側に入れ子の cons が来た時に *内側が外側の car を潰す*。
                //    実測 2026-09-05: `car(cons(300, cons(2, nil)))` が 300 でなく **2** を返した。
                //    落ちない・床D/段E/段F は 300 で揃う ⇒ 段G′ だけが静かに間違える形だった。
                //    ◆ 局所の値を **大域の器**に置いたのが根。入れ子は言語の側に元から在る。
                let (ta, td) = (self.slot(), self.slot());
                self.op_u(0x21, ta);                                                       // local.set $ta
                let d = self.emit_val(pcdr(arg))?;
                if d == Kind::Str {
                    // 段G⁗: 前置は **コピーが要る**。断っていた所を、コピーを書いて支えた(2026-09-06)。
                    // 🔴 ただし **頭が byte に収まる時だけ**。詰めた文字列は 1 B/要素なので、
                    //   任意の整数を前に置くと *黙って切り捨てる*(i64.store8)。
                    //   実測 2026-09-06: 差分ファズが `cons(add(394,305), str)` で掴んだ ——
                    //   699 が 187(=699&0xFF)に化け、和が 256 ずれた。落ちない。
                    //   ⚠️ **前日「正しい断り」だった所を、静かに間違える実装に替えていた。**
                    //     支えられる形だけ支え、残りは断る —— 断りの範囲を *狭めた* のであって、無くしたのではない。
                    // ▲ まだ書いていない道: str をセルへ開いてから前置すれば任意の整数を支えられる
                    //   (16 B/文字。正しいが場所を食う)。要ると分かってから書く。
                    match Self::static_byte(pcar(arg)) {
                        Some(_) => { self.emit_str_prepend(hp, ta); return Ok(Kind::Str); }
                        None => return Err("詰めた文字列への前置は **頭が 0..255 の即値の時だけ** —— 任意の整数は 1 B に収まらない [REFUSE str-prepend-nonbyte]".into()),
                    }
                }
                Self::want(Kind::List, d, "cons の cdr")?;
                self.op_u(0x21, td);
                for (off, loc) in [(0u32, ta), (8, td)] {
                    self.op_u(0x20, hp); self.op(0xA7);                                    // i32.wrap_i64
                    self.op_u(0x20, loc);
                    self.memop(0x37, 0x03, off);                                           // i64.store
                }
                self.op_u(0x20, hp);                                                       // 値 = 番地
                self.op_u(0x20, hp); self.op(0x42); self.b.push(0x10); self.op(0x7C); self.op_u(0x21, hp);
                self.emit_peak(hp);
                Kind::List
            }
            ntag::CAR | ntag::CDR | ntag::PAIRP => {
                // 先に emit して、返ってきた *形* で分ける。⇒ 先読みの仕掛けが要らない。
                let x = self.emit_val(arg)?;
                if x != Kind::Str {
                    Self::want(Kind::List, x, "car/cdr/pair?")?;
                    return Ok(match tag {
                        ntag::CAR | ntag::CDR => { self.op(0xA7);                                          // i32.wrap
                                    self.memop(0x29, 0x03, if tag == ntag::CAR { 0 } else { 8 });   // i64.load
                                    if tag == ntag::CAR { Kind::Int } else { Kind::List } }
                        _ => { self.op(0x42); self.b.push(0x00); self.op(0x52); self.op(0xAD); Kind::Int }
                    });
                }
                // 詰めた文字列の上での car / cdr / pair? —— **セルを一つも触らない**。
                match tag {
                    ntag::CAR => { // car = 先頭の byte。番地 = v >>> 32
                        self.op(0x42); self.b.push(32); self.op(0x88);                     // i64.const 32 / i64.shr_u
                        self.op(0xA7); self.memop(0x2D, 0x00, 0);                           // i32.wrap / i32.load8_u
                        self.op(0xAD); Kind::Int }                                          // i64.extend_i32_u
                    ntag::CDR => { // cdr = (番地+1, 長さ-1)。長さ 0 なら nil(=0)。
                        let t = self.tmp_str(); self.op_u(0x22, t);                         // local.tee
                        self.op(0x42); sleb(0xFFFF_FFFFu32 as i64, &mut self.b); self.op(0x7C);  // v + (1<<32) - 1
                        self.op(0x42); self.b.push(0x00);                                   // 偽の枝: nil
                        self.op_u(0x20, t); self.op(0x42); sleb(0xFFFF_FFFFu32 as i64, &mut self.b);
                        self.op(0x83);                                                      // i64.and → 長さ
                        self.op(0x42); self.b.push(0x00); self.op(0x52);                    // i64.ne → i32
                        self.op(0x1B); Kind::Str }                                          // select
                    _ => { // pair? = 長さ != 0
                        self.op(0x42); sleb(0xFFFF_FFFFu32 as i64, &mut self.b); self.op(0x83);
                        self.op(0x42); self.b.push(0x00); self.op(0x52); self.op(0xAD); Kind::Int }
                }
            }
            ntag::OAPP => {                                                                        // oapp —— inline
                let f = pcar(arg);
                if !is_p(f) || pnum(pcar(f)) != ntag::VAR { return Err("呼び先が var でない(閉包が escape)".into()); }
                let idx = match self.look(pnum(pcdr(f))) {
                    Some(Bind::Fn(i)) => i, _ => return Err("呼び先が静的に定まらない(閉包が escape)".into()) };
                if self.depth >= 8 { return Err("inline が深すぎる".into()); }
                let (param, body, elen) = self.fns[idx as usize].clone();
                let ka = self.emit_val(pcdr(arg))?;                 // 実引数は呼び側の env で
                let s = self.slot(); self.op_u(0x21, s); self.kinds.insert(s, ka);
                // body は **定義時の env** で解く（段E と同じ理由。二箇所に同じ穴が在った）
                let 呼び側 = self.env.split_off(elen);
                self.env.push((param, Bind::Slot(s)));
                self.depth += 1; let r = self.emit_val(&body); self.depth -= 1;
                self.env.truncate(elen);        // ← param もここで落ちる(元の env.pop() は外した)
                self.env.extend(呼び側);
                r?
            }
            ntag::OFN => return Err("閉包が let の外".into()),
            ntag::VAR => match self.look(pnum(arg)) {
                Some(Bind::Slot(s)) => { self.op_u(0x20, s); self.slot_kind(s) }
                Some(Bind::Fn(_)) => return Err("閉包を値として使った(escape)".into()),
                Some(Bind::Boxed(_)) => return Err("box handle を値として使った(escape)".into()),
                None => {
                    let id = pnum(arg);
                    match self.outer.iter().find(|(i, _)| *i == id).map(|(_, w)| *w) {
                        Some(Where::Param(p)) => { self.op_u(0x20, p); Kind::Int }
                        Some(Where::Mem(s)) => { self.op(0x41); sleb((s as i64) * 8, &mut self.b);
                                                 self.memop(0x29, 0x03, 0); Kind::Int }
                        None => return Err(format!("未束縛の var id {}", id)),
                    }
                }
            },
            ntag::LET => { self.bind_let(arg)?; let k = self.emit_val(pcdr(pcdr(arg)))?;
                   // ⚠️ Str も heap を指しうる(段G⁗ の前置) ⇒ **int 以外は持ち出し**扱い。
                   //   list だけ見ていると、回収した番地を長さ付きで持ち歩くことになる。
                   self.close_scope(k != Kind::Int); k }
            ntag::IF => {
                   // ⚠️ **値位置でも `if`(0x04)を使わない** —— init.zig(exp/03)が持たない(九段の実測
                   //    「未対応 opcode 0x4 @body」)。効果位置(下)と同じ block 二枚で書き、
                   //    枝の値は控えの局所で受ける。⇒ 口を二つ持たない。
                   // ▲ 控えは **site ごと**に取る（共有しない）—— 二十五段の型そのもの。
                   //    入れ子の if で外側の控えを内側が潰す（局所の値を大域の器に置くと合わない）。
                   let sv = self.slot();
                   self.op(0x02); self.b.push(0x40);                                       // block L1(抜け先)
                   self.op(0x02); self.b.push(0x40);                                       // block L0(else 先)
                   let c = self.emit_val(pcar(arg))?; Self::want(Kind::Int, c, "if の条件")?;
                   self.op(0x42); self.b.push(0x00); self.op(0x52);                        // cond != 0
                   self.op(0x45); self.op_u(0x0D, 0);                                      // 偽 → L0 へ
                   // 🔴 枝で形が割れる一番多い理由は **要素の値**（2026-09-06、差分ファズが掴んだ）。
                   //   `if c then cons(3,cons(4,nil)) else cons(300,nil)` —— 前者は byte に収まるので
                   //   段G‴ が **詰めて Str** にし、後者は List のまま ⇒ 同じ「list を返す if」なのに断っていた。
                   //   ◆ Str は list の *置き方* であって別の型ではない ⇒ **割れたら詰めるのをやめて揃える。**
                   //   ⚠️ 予測しない。**同じ emit をもう一度通す** —— 形を先読みする関数を別に持つと、
                   //     それは吐く側と食い違いうる二つ目の口になる（長さの表で一度やった型）。
                   //   ▲ 巻き戻しは枝の *全体* に効く ⇒ 割れた if の下の関係ない文字列も開かれる（遅いが正しい）。
                   //   ▲ 巻き戻すのは **b(吐いた byte)だけ** —— 一周目に取った局所と池の文字は残る
                   //     （使われない局所と、誰も指さない data の数 byte）。正しさには効かない。場所だけ。
                   let mark = self.b.len();
                   let (t, f) = self.emit_both(arg, sv)?;
                   let k = if t == f { t } else {
                       if !matches!((t, f), (Kind::Str, Kind::List) | (Kind::List, Kind::Str)) {
                           return Err("if の両枝で値の形が違う".into());
                       }
                       self.b.truncate(mark);
                       let 元 = self.no_intern;
                       self.no_intern = true;
                       let r = self.emit_both(arg, sv);
                       self.no_intern = 元;              // ⚠️ 落ちても必ず戻す（外側の枝を巻き込まない）
                       let (t2, f2) = r?;
                       if t2 != f2 { return Err("if の両枝で値の形が違う".into()); }
                       t2
                   };
                   self.op_u(0x20, sv);                                                    // 控えを読む = 値位置
                   self.kinds.insert(sv, k);
                   k }
            ntag::GETBOX => { let s = self.box_slot(arg)?; self.op_u(0x20, s); self.slot_kind(s) }
            ntag::SETBOX => { let s = self.box_slot(pcar(arg))?; let want = self.slot_kind(s);
                    let k = self.emit_val(pcdr(arg))?; Self::want(want, k, "setbox")?;
                    self.op_u(0x22, s); k }                                                // local.tee(書いた値を返す)
            ntag::WHILE => {
                let it = self.slot(); self.kinds.insert(it, Kind::Int);
                self.op(0x42); self.b.push(0x00); self.op_u(0x21, it);
                self.op(0x02); self.b.push(0x40);                                          // block(void) ← depth 1
                self.op(0x03); self.b.push(0x40);                                          // loop(void)  ← depth 0
                let c = self.emit_val(pcar(arg))?; Self::want(Kind::Int, c, "while の条件")?;
                self.op(0x42); self.b.push(0x00); self.op(0x51);
                self.op_u(0x0D, 1);                                                        // br_if 1(抜ける)
                self.emit_eff(pcdr(arg))?;
                self.op_u(0x20, it); self.op(0x42); self.b.push(0x01); self.op(0x7C); self.op_u(0x21, it);
                self.op_u(0x0C, 0);                                                        // br 0(先頭へ)
                self.op(0x0B); self.op(0x0B);
                self.op_u(0x20, it); Kind::Int
            }
            ntag::NEWBOX => return Err("newbox が let の外".into()),
            t => return Err(format!("この切片は tag {} を持たない", t)),
        };
        Ok(k)
    }

    fn emit_eff(&mut self, n: &Json) -> Result<(), String> {
        if !is_p(n) { return Err("object 節でない".into()); }
        let tag = pnum(pcar(n));
        let arg = pcdr(n);
        match tag {
            ntag::CONS => { self.emit_eff(pcar(arg))?; self.emit_eff(pcdr(arg))?; }      // 死んだ確保を出さない
            ntag::LET => { self.bind_let(arg)?; self.emit_eff(pcdr(pcdr(arg)))?; self.close_scope(false); }
            ntag::SETBOX => {
                let s = self.box_slot(pcar(arg))?;
                // 段G″: `x := cdr(x)` は **消費**。x が一意所有なら、外れた頭を bump heap へ返す。
                let v = pcdr(arg);
                let is_consume = is_p(v) && pnum(pcar(v)) == ntag::CDR
                    && { let inner = pcdr(v);
                         is_p(inner) && pnum(pcar(inner)) == ntag::GETBOX
                         && self.box_slot(pcdr(inner)).map(|s2| s2 == s).unwrap_or(false) };
                if self.slot_kind(s) != Kind::Int
                   && self.regions.last().map(|(_, bs)| *bs != s).unwrap_or(false) {
                    self.foreign_list_write = true;              // region の外の heap 値を書いた(list / str)
                }
                if self.own == 1 && is_consume && self.owned.contains_key(&s) {
                    let (hp, ta, _) = self.heap_locals();
                    self.op_u(0x20, s); self.op_u(0x22, ta);                    // 旧 head を控える
                    self.op(0xA7); self.memop(0x29, 0x03, 8);                            // cdr
                    self.op_u(0x21, s);                                         // x := cdr(x)
                    // 返せるのは **積んだ順の逆(LIFO)** の時だけ —— 頂なら hp を戻す
                    self.op(0x02); self.b.push(0x40);                           // block(void)
                    self.op_u(0x20, ta); self.op(0x42); self.b.push(0x10); self.op(0x7C);
                    self.op_u(0x20, hp); self.op(0x51);                         // ta+16 == hp ?
                    self.op(0x45); self.op_u(0x0D, 0);                          // 違えば抜ける
                    self.op_u(0x20, ta); self.op_u(0x21, hp);                   // hp = ta(pop)
                    self.op(0x0B);
                } else {
                    let want = self.slot_kind(s);
                    let k = self.emit_val(v)?; Self::want(want, k, "setbox")?;
                    self.op_u(0x21, s);
                }
            }
            ntag::IF => { // if(効果位置)—— block 二枚 + br で書く(0x04 を使わない。上の註と同じ理由)
                   self.op(0x02); self.b.push(0x40);                            // block L1(抜け先)
                   self.op(0x02); self.b.push(0x40);                            // block L0(else 先)
                   let c = self.emit_val(pcar(arg))?; Self::want(Kind::Int, c, "if の条件")?;
                   self.op(0x42); self.b.push(0x00); self.op(0x52);             // cond != 0
                   self.op(0x45); self.op_u(0x0D, 0);                           // 偽 → L0 へ
                   self.emit_eff(pcar(pcdr(arg)))?;
                   self.op_u(0x0C, 1);                                          // br L1(then の後は抜ける)
                   self.op(0x0B);                                               // end L0
                   self.emit_eff(pcdr(pcdr(arg)))?;
                   self.op(0x0B); }                                             // end L1
            _ => { self.emit_val(n)?; self.op(0x1A); }                          // drop
        }
        Ok(())
    }
}

fn section(id: u8, content: Vec<u8>, out: &mut Vec<u8>) {
    out.push(id); uleb(content.len() as u32, out); out.extend_from_slice(&content);
}

/// 完全な wasm module を組む(export "run": () -> i64)。
// ⚠️ ここに `wasm_module`(= 池なしで module を組む薄い包み)が在った。**一度も呼ばれていなかった**。
//    `listish` と同じ型 —— 呼ばれない関数は、在るだけで「そういう道が在る」と読ませる。
//    池が空でも `wasm_module_d(&[])` で足りる ⇒ 包みは要らなかった。rustc の dead_code が
//    ずっと鳴っていたのに、warning を見ていなかった(2026-09-05 に気づいた)。

/// ⚠️ data section(11)は code(10)の **後ろ**。節の順を違えると読み手が拒む。
/// 吐いた module に **import 節(id 2)が在るか**を走査する。
///
/// 🔴 「何も import しない = ホストに手が届かない」は README が **限界として** 名乗っている主張。
///   ⚠️ だがこれは *不在* を根拠にする主張で、崩れた時に黙る側 ——
///     import を一本足しても、誰も「増えた」と言わない。
///   ⇒ 吐いた側が **走査して印を出す**（`[NOIMPORT ok]`）。門はその印が *在ること* を要る。
///     証拠の向きを、不在から在ることへ裏返すための一手（2026-09-06）。
fn has_import(m: &[u8]) -> bool {
    let mut i = 8;                                   // magic(4) + version(4)
    while i < m.len() {
        let sid = m[i]; i += 1;
        let (mut n, mut sh) = (0usize, 0u32);
        loop {
            if i >= m.len() { return false; }
            let x = m[i]; i += 1;
            n |= ((x & 0x7f) as usize) << sh; sh += 7;
            if x < 0x80 { break; }
        }
        if sid == 2 { return true; }
        i += n;
    }
    false
}


/// 吐いた module の code 節を歩き、`if`(0x04)が **居ないこと** を確かめる。
/// ⚠️ これは *不在* を根拠にする主張 ⇒ 二十三段の型どおり、**在ることを要る印**へ裏返す
///    （`[NO-IFOP ok]`。`NOIMPORT` と同じ向き）。
/// ⚠️ 知らない opcode に当たったら「無い」ではなく **測れない** で落ちる ——
///    黙って 0 を返すと、壊れた走査と「本当に無い」が同じ答えになる（門⑥ で一度払った型）。
fn ifop_count(m: &[u8]) -> Result<(usize, usize), String> {
    fn u(m: &[u8], i: &mut usize) -> Result<usize, String> {
        let (mut n, mut sh) = (0usize, 0u32);
        loop {
            if *i >= m.len() { return Err("uleb が module の外へ出た".into()); }
            let x = m[*i]; *i += 1;
            n |= ((x & 0x7f) as usize) << sh; sh += 7;
            if x < 0x80 { return Ok(n); }
        }
    }
    fn sk(m: &[u8], i: &mut usize) -> Result<(), String> {
        loop {
            if *i >= m.len() { return Err("sleb が module の外へ出た".into()); }
            let x = m[*i]; *i += 1;
            if x < 0x80 { return Ok(()); }
        }
    }
    // 引数を持たない opcode（この吐き出し器が実際に出す物だけ）
    const NOIMM: [u8; 24] = [0x00, 0x01, 0x05, 0x0B, 0x0F, 0x1A, 0x1B, 0x45, 0x46, 0x51, 0x52,
                             0x55, 0x6A, 0x6B, 0x7C, 0x7D, 0x7E, 0x7F, 0x83, 0x84, 0x86, 0x88,
                             0xA7, 0xAD];
    let (mut ifs, mut ops) = (0usize, 0usize);
    let mut i = 8;                                        // magic(4) + version(4)
    while i < m.len() {
        let sid = m[i]; i += 1;
        let n = u(m, &mut i)?;
        let end = i + n;
        if end > m.len() { return Err("節が module の外へ出た".into()); }
        if sid == 10 {                                    // code 節
            let mut j = i;
            let cnt = u(m, &mut j)?;
            for _ in 0..cnt {
                let sz = u(m, &mut j)?;
                let bend = j + sz;
                if bend > end { return Err("本体が code 節の外へ出た".into()); }
                let decls = u(m, &mut j)?;
                for _ in 0..decls { u(m, &mut j)?; j += 1; }
                while j < bend {
                    let op = m[j]; j += 1; ops += 1;
                    if op == 0x04 { ifs += 1; }
                    if NOIMM.contains(&op) {
                    } else if op == 0x02 || op == 0x03 || op == 0x04 { j += 1;          // blocktype
                    } else if matches!(op, 0x0C | 0x0D | 0x20 | 0x21 | 0x22) { u(m, &mut j)?;
                    } else if op == 0x41 || op == 0x42 { sk(m, &mut j)?;
                    } else if (0x28..=0x3E).contains(&op) { u(m, &mut j)?; u(m, &mut j)?;
                    } else { return Err(format!("測れない —— 知らない opcode 0x{:02X}", op)); }
                }
                j = bend;
            }
        }
        i = end;
    }
    Ok((ifs, ops))
}

/// module を組む口は必ずここを通る ⇒ 印を出し忘れられない（宣言を使用から導く。二十六段の型）。
fn no_ifop(m: &[u8]) {
    match ifop_count(m) {
        Err(e) => { println!("  ⛔ 値位置の if を測れない —— {}", e); std::process::exit(1); }
        Ok((n, _)) if n > 0 => {
            println!("  ⛔ module に if(0x04) が {} 個 —— init.zig が持たない形を吐いている", n);
            std::process::exit(1);
        }
        Ok((_, ops)) => println!("  自給の続き: 値位置の if も block で書いた —— op {} を歩いて 0x04 なし [NO-IFOP ok]", ops),
    }
}

fn wasm_module_d(body: &[u8], nlocals: u32, pages: Option<u32>, pool: &[u8]) -> Vec<u8> {
    let mut m: Vec<u8> = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
    let mut t = Vec::new(); uleb(1, &mut t); t.push(0x60); uleb(0, &mut t); uleb(1, &mut t); t.push(0x7E);
    section(1, t, &mut m);                                            // Type: () -> i64
    let mut f = Vec::new(); uleb(1, &mut f); uleb(0, &mut f);
    section(3, f, &mut m);                                            // Function: func0 : type0
    let heap = pages.is_some();
    if let Some(pg) = pages {                                          // 段G′: pair の bump heap
        let mut mem = Vec::new(); uleb(1, &mut mem); mem.push(0x00); uleb(pg, &mut mem);
        section(5, mem, &mut m);
    }
    // Export: "run" ＋ **"kernel"(同じ関数の別名)** ＋ heap があれば "memory"。
    // ⚠️ `kernel` は姉妹実験 exp/03 の futamura-harness が **mode 分岐より手前で**要求する
    //    (2026-09-04 実測: 「[fut] `kernel` export が無い」)。A(P) を測る口を開けておくための別名。
    let mut e = Vec::new(); uleb(if heap { 3 } else { 2 }, &mut e);
    uleb(3, &mut e); e.extend_from_slice(b"run"); e.push(0x00); uleb(0, &mut e);
    uleb(6, &mut e); e.extend_from_slice(b"kernel"); e.push(0x00); uleb(0, &mut e);
    if heap { uleb(6, &mut e); e.extend_from_slice(b"memory"); e.push(0x02); uleb(0, &mut e); }
    section(7, e, &mut m);
    let mut fb = Vec::new();
    uleb(1, &mut fb); uleb(nlocals, &mut fb); fb.push(0x7E);          // locals: nlocals × i64
    fb.extend_from_slice(body); fb.push(0x0B);                        // end
    let mut c = Vec::new(); uleb(1, &mut c); uleb(fb.len() as u32, &mut c); c.extend_from_slice(&fb);
    section(10, c, &mut m);                                           // Code
    if !pool.is_empty() {                                             // 段G‴: リテラル文字列の実体
        let mut d = Vec::new(); uleb(1, &mut d); uleb(0, &mut d);      // 1 本 / memidx 0
        d.push(0x41); sleb(16, &mut d); d.push(0x0B);                  // offset = i32.const 16
        uleb(pool.len() as u32, &mut d); d.extend_from_slice(pool);
        section(11, d, &mut m);
    }
    no_ifop(&m);
    m
}

/// 段H: memory を持ち、host に駆動される module。export: step(i64)->i64 と memory。
/// 状態は **線形メモリの i64 スロット** —— JS からは BigInt64Array でそのまま読める(これが seam)。
fn wasm_engine_module(body: &[u8], nlocals: u32) -> Vec<u8> {
    let mut m: Vec<u8> = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
    let mut t = Vec::new(); uleb(1, &mut t); t.push(0x60);
    uleb(1, &mut t); t.push(0x7E);                                    // 引数 1: i64(event)
    uleb(1, &mut t); t.push(0x7E);                                    // 返り値 i64
    section(1, t, &mut m);
    let mut f = Vec::new(); uleb(1, &mut f); uleb(0, &mut f);
    section(3, f, &mut m);
    let mut mem = Vec::new(); uleb(1, &mut mem); mem.push(0x00); uleb(1, &mut mem);  // 1 memory / min 1 page
    section(5, mem, &mut m);
    let mut e = Vec::new(); uleb(2, &mut e);
    uleb(4, &mut e); e.extend_from_slice(b"step"); e.push(0x00); uleb(0, &mut e);
    uleb(6, &mut e); e.extend_from_slice(b"memory"); e.push(0x02); uleb(0, &mut e);
    section(7, e, &mut m);
    let mut fb = Vec::new();
    uleb(1, &mut fb); uleb(nlocals, &mut fb); fb.push(0x7E);
    fb.extend_from_slice(body); fb.push(0x0B);
    let mut c = Vec::new(); uleb(1, &mut c); uleb(fb.len() as u32, &mut c); c.extend_from_slice(&fb);
    section(10, c, &mut m);
    no_ifop(&m);
    m
}

fn b64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::new();
    for ch in data.chunks(3) {
        let b = [ch[0], *ch.get(1).unwrap_or(&0), *ch.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        o.push(T[(n >> 18) as usize & 63] as char);
        o.push(T[(n >> 12) as usize & 63] as char);
        o.push(if ch.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        o.push(if ch.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    o
}

// ---- 走らせる ------------------------------------------------------------------
fn main() {
    // ---- 段G: --web —— 同じ程式を wasm へ吐き、自己完結 HTML に畳む ----------------
    if std::env::args().any(|a| a == "--web") {
        let bs = fs::read_to_string("bench_host.json").expect("bench_host.json が要る(floor_emit.py)");
        let bench = P::new(&bs).val();
        let (mut hj, mut n, mut expect) = (Json::Num(0), 0i64, V::I(0));
        if let Json::Obj(kv) = &bench { for (k, v) in kv { match k.as_str() {
            "host" => hj = v.clone(), "n" => if let Json::Num(x) = v { n = *x },
            "expect" => expect = to_v(v), _ => {} } } }
        let mut td: Option<&Json> = None;
        if let Json::Obj(kvs) = &hj { for (k, v) in kvs { if k == "td" { td = Some(v); } } }
        let prog = pcar(td.expect("td が無い"));

        let mut wc = WComp::new();
        wc.emit_val(prog).expect("段G: 畳めない");
        wc.finish();
        // ⚠️ `heap.is_some()` で決めていた —— **cons しないが car する**程式を落としていた。
        //    memory を要るのは *触ったか* であって、*積んだか* ではない（池は data 節が要るので別枠）。
        let pages = if wc.mem || !wc.pool.is_empty() { Some(16) } else { None };
        let module = wasm_module_d(&wc.b, wc.nslots, pages, &wc.pool);
        fs::write("compiled.wasm", &module).unwrap();

        let html = format!(r#"<!doctype html><meta charset="utf-8"><title>kokkos → wasm(自己完結)</title>
<style>
 :root{{color-scheme:light dark;--ink:#1a1a1a;--bg:#fbfaf7;--dim:#6b6b6b;--line:#dcd8d0;--hit:#a4472f}}
 @media (prefers-color-scheme:dark){{:root{{--ink:#e8e6e1;--bg:#16161a;--dim:#9a9a9a;--line:#33333a;--hit:#e08a6a}}}}
 body{{margin:0;padding:2.2rem 1.4rem;background:var(--bg);color:var(--ink);
      font:15px/1.75 ui-sans-serif,system-ui,"Hiragino Sans","Noto Sans JP",sans-serif}}
 main{{max-width:44rem;margin:0 auto}} h1{{font-size:1.25rem;margin:0 0 .3rem}}
 .dim{{color:var(--dim);font-size:.86rem}} code{{font-family:ui-monospace,SFMono-Regular,Menlo,monospace}}
 .out{{font-size:2.4rem;font-weight:600;color:var(--hit);margin:.6rem 0}}
 table{{border-collapse:collapse;width:100%;margin:1.2rem 0;font-size:.9rem}}
 td,th{{border-bottom:1px solid var(--line);padding:.4rem .2rem;text-align:left}}
 td:last-child,th:last-child{{text-align:right;font-family:ui-monospace,monospace}}
 .note{{border-left:3px solid var(--line);padding:.1rem 0 .1rem .9rem;margin:1.2rem 0;font-size:.88rem;color:var(--dim)}}
</style>
<main>
<h1>kokkos の程式が、この頁の中で走る</h1>
<p class="dim">外部 fetch ゼロ・server ゼロ・framework ゼロ。この 1 ファイルだけで完結する。</p>
<p><code>i = {n}; s = 0; while i {{ s = s + i; i = i - 1 }}; s</code></p>
<div class="out" id="out">…</div>
<table>
 <tr><th>この頁が積んでいる wasm</th><td id="sz">—</td></tr>
 <tr><th>期待値(Python 床が出した答え)</th><td>{exp}</td></tr>
 <tr><th>秤</th><td id="ok">—</td></tr>
</table>
<div class="note">
 これは <b>backend の選択肢</b>であって言語の性質ではない —— 同じ特殊化命令列が、
 x86-64(187 B の機械語)にも wasm にも落ちる。HTML はその wasm を base64 で抱えているだけ。
 言語側に HTML も DOM も足していない。
</div>
</main>
<script>
const B="{b64}";
const bin=Uint8Array.from(atob(B),c=>c.charCodeAt(0));
document.getElementById("sz").textContent=bin.length+" B";
WebAssembly.instantiate(bin).then(({{instance}})=>{{
  const t0=performance.now();
  let r; for(let i=0;i<1000;i++) r=instance.exports.run();
  const ms=(performance.now()-t0)/1000;
  document.getElementById("out").textContent=String(r);
  const ok=String(r)==="{exp}";
  document.getElementById("ok").textContent=(ok?"一致 ✓":"不一致 ✗")+" / "+ms.toFixed(4)+" ms";
  document.title=(ok?"一致 ":"不一致 ")+String(r);
}}).catch(e=>{{document.getElementById("out").textContent="wasm 失敗: "+e;}});
</script>
"#, n = n, exp = show(&expect), b64 = b64(&module));
        fs::write("demo-compiled.html", &html).unwrap();
        println!("段G: 程式 → wasm {} B(外部 toolchain ゼロ)/ 自己完結 HTML {} B", module.len(), html.len());
        // ⚠️ 「何も import しない」を **走査して名乗る**。不在を根拠にする主張は黙って崩れる。
        if has_import(&module) {
            println!("  ⛔ 自給が崩れた —— **import 節が在る**。README の限界の一行が嘘になっている");
            std::process::exit(1);
        }
        println!("  自給: 吐いた module に import 節なし —— ホストに手は届かない [NOIMPORT ok]");
        println!("  → compiled.wasm と demo-compiled.html。検証: node web_verify.mjs");
        return;
    }

    // ---- --probe FILE: 反証テスト —— 整数ループ以外で梯子が保つか ------------------
    {
        let args: Vec<String> = std::env::args().collect();
        if let Some(i) = args.iter().position(|a| a == "--probe") {
            let file = args.get(i + 1).expect("--probe <probe_*.json>");
            let machine = P::new(&fs::read_to_string("machine.json").unwrap()).val();
            let mut ns: Vec<N> = Vec::new();
            let mut nm = Names::new();
            let root = build(&machine, &mut ns, &mut nm);
            let vs = nm.intern("vs");
            let mut mcode: Vec<Ins> = Vec::new();
            emit_stmt(&ns, root, &mut mcode);
            // 🔴 **走る列と数える列を分ける**（2026-09-06）。畳むと dispatch は減るが、
            //   `count_d` が数えているのは *塔の厚み*（機械の意味が要求する仕事の量）で、
            //   畳んで減るのは *床の手数* —— **別の量**。混ぜると「塔が薄くなった」と読めてしまう。
            //   ⇒ 数えるのは畳む前、走らせるのは畳んだ後。名前で分ける。
            let mut mcode_run = mcode.clone();
            畳む(&mut mcode_run);

            let pj = P::new(&fs::read_to_string(file).expect("probe json")).val();
            let (mut hj, mut expect, mut n) = (Json::Num(0), V::I(0), 0i64);
            if let Json::Obj(kv) = &pj { for (k, v) in kv { match k.as_str() {
                "host" => hj = v.clone(), "expect" => expect = to_v(v),
                "n" => if let Json::Num(x) = v { n = *x }, _ => {} } } }

            println!("== 反証テスト: {}(n={})== 「407x は整数ループの数字か」を壊しに行く\n", file, n);

            // 床D(現在地)
            let (mut td_ms, mut rd, mut cells_d) = (f64::MAX, V::I(0), 0usize);
            for _ in 0..3 {
                let mut ar: Arena = Vec::new(); let hd = host_cd(&hj, &mut nm, &mut ar);
                let t = Instant::now();
                let v = run_d(&mcode_run, hd, &mut ar, 100_000_000, vs);
                td_ms = td_ms.min(t.elapsed().as_secs_f64() * 1000.0);
                cells_d = ar.len(); rd = top(val_to_v(&ar, v));
            }
            let ins_d = { let mut ar: Arena = Vec::new(); let hd = host_cd(&hj, &mut nm, &mut ar);
                          count_d(&mcode, hd, &mut ar, 100_000_000).0 };

            // 段E(特殊化)
            let mut td_j: Option<&Json> = None;
            if let Json::Obj(kvs) = &hj { for (k, v) in kvs { if k == "td" { td_j = Some(v); } } }
            let prog = pcar(td_j.expect("td"));
            let mut cp = Comp::new();
            match cp.emit_val(prog) {
                Err(e) => { println!("  床D  {:>9.2} ms  結果 {}  実行命令 {}", td_ms, show(&rd), ins_d);
                            println!("  段E  **畳めない** —— {}", e);
                            println!("\n  🔴 ここで梯子が止まる = 反証条件に当たった。何が足りないかは上の一行が名指している。");
                            return; }
                Ok(()) => {}
            }
            let (mut te, mut ins_e, mut ve, mut cells_e) = (f64::MAX, 0u64, V::I(0), 0usize);
            for _ in 0..5 {
                let mut ar: Arena = Vec::new(); let mut cnt = 0u64;
                let t = Instant::now();
                let mut last = Val::I(0);
                for _ in 0..20 { ar.clear(); cnt = 0; last = run_e(&cp.code, cp.nslots, &mut ar, &mut cnt); }
                te = te.min(t.elapsed().as_secs_f64() * 1000.0 / 20.0);
                ins_e = cnt; cells_e = ar.len(); ve = val_to_v(&ar, last);
            }

            // 段F / 段G(通るか、断るか)
            let f_res = jit(&cp.code);
            let mut wc = WComp::new();
            wc.peak = !args.iter().any(|a| a == "--nopeak");
            wc.own = if args.iter().any(|a| a == "--own2") { 2 }
                     else if args.iter().any(|a| a == "--own") { 1 } else { 0 };
            wc.root = Some(prog.clone());
            let g_res = wc.emit_val(prog).map(|_| {
                wc.finish();
                // ⚠️ `heap.is_some()` で決めていた —— **cons しないが car する**程式を落としていた。
        //    memory を要るのは *触ったか* であって、*積んだか* ではない（池は data 節が要るので別枠）。
        let pages = if wc.mem || !wc.pool.is_empty() { Some(16) } else { None };
                let m = wasm_module_d(&wc.b, wc.nslots, pages, &wc.pool);
                let name = file.replace(".json", &format!("{}{}",
                    match wc.own { 2 => ".own2", 1 => ".own", _ => "" },
                    if wc.peak { ".wasm" } else { ".np.wasm" }));
                fs::write(&name, &m).unwrap();
                (m.len(), pages.is_some(), name)
            });
            let (mut tf, mut vf) = (f64::MAX, V::I(0));
            if let Ok(bytes) = &f_res {
                let f = Jitted::new(bytes);
                let mut slots = vec![0i64; cp.nslots as usize];
                for _ in 0..5 {
                    let t = Instant::now(); let mut r = 0i64;
                    for _ in 0..200 { for x in slots.iter_mut() { *x = 0; } r = f.call(&mut slots); }
                    tf = tf.min(t.elapsed().as_secs_f64() * 1000.0 / 200.0);
                    vf = V::I(r);
                }
            }

            let ok = deep_eq(&rd, &expect) && deep_eq(&ve, &expect) && (f_res.is_err() || deep_eq(&vf, &expect));
            println!("  床D  {:>9.2} ms  結果 {}  実行命令 {:>9}  arena {} セル", td_ms, show(&rd), ins_d, cells_d);
            println!("  段E  {:>9.4} ms  結果 {}  実行命令 {:>9}  arena {} セル   → **床D 比 {:.0}x**",
                     te, show(&ve), ins_e, cells_e, td_ms / te);
            println!("       命令の減り: {} → {} = **{:.0} 分の 1**", ins_d, ins_e, ins_d as f64 / ins_e as f64);
            match &f_res {
                Ok(b) => println!("  段F  {:>9.4} ms  結果 {}  機械語 {} B                     → 床D 比 {:.0}x",
                                  tf, show(&vf), b.len(), td_ms / tf),
                Err(e) => println!("  段F  **吐けない** —— {}", e),
            }
            match &g_res {
                Ok((sz, heap, name)) => println!("  段G′ wasm {} B(畳めた{}{})→ {}   検証: node probe_run.mjs {} {}",
                                                sz, if *heap { " / pair は bump heap へ" } else { "" },
                                                match wc.own { 1 => format!(" / **所有でセル単位に回収**: box {} 個", wc.owned.len()),
                                                               2 => format!(" / **所有でまとめて捨てる(region)**: box {} 個", wc.owned.len()),
                                                               _ => String::new() },
                                                name, name, show(&expect)),
                Err(e) => println!("  段G′ **吐けない** —— {}", e),
            }
            // ⚠️ 段G′ は wasm なので **ここでは走っていない**(この bin に runtime が無い)。
            //    かつて「全段一致 ✓」と出していたが、それは 床D/段E/段F の話だった ⇒ 名が範囲を偽っていた。
            println!("\n  秤: 期待 {} と 床D/段E/段F 一致 {}", show(&expect),
                     if ok { "✓ [AGREE stage-DEF]" } else { "✗ 不一致" });
            if let Ok((_, _, name)) = &g_res {
                println!("  ⚠️ 段G′ は **まだ走らせていない** —— 出口の一致は別に撃つ:");
                println!("     node probe_run.mjs {} {}", name, show(&expect));
            }
            if !ok { std::process::exit(1); }
            return;
        }
    }

    // ---- 段H: --engine —— host に駆動される wasm(状態は線形メモリ、DOM は JS)----------
    if std::env::args().any(|a| a == "--engine") {
        fn objget<'a>(j: &'a Json, key: &str) -> Option<&'a Json> {
            match j { Json::Obj(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v), _ => None }
        }
        let pj = P::new(&fs::read_to_string("engine_prog.json").expect("engine_emit.py を先に")).val();
        let ids = objget(&pj, "ids").expect("ids");
        let state: Vec<String> = match objget(&pj, "state") {
            Some(Json::Arr(a)) => a.iter().map(|x| sof(x).to_string()).collect(), _ => vec![] };
        let input = sof(objget(&pj, "input").expect("input")).to_string();
        let nexts: Vec<Json> = match objget(&pj, "next") { Some(Json::Arr(a)) => a.clone(), _ => vec![] };
        let wrap = match objget(&pj, "wrap") { Some(Json::Num(n)) => *n, _ => 10 };
        assert_eq!(state.len(), 2, "この切片は状態 2 つ(total / wraps)");
        assert_eq!(nexts.len(), 2, "next は状態と同数");

        let mut wc = WComp::new();
        for (i, nm) in state.iter().enumerate() {
            wc.outer.push((pnum(objget(ids, nm).expect("id")), Where::Mem(i as u32)));
        }
        wc.outer.push((pnum(objget(ids, &input).expect("id")), Where::Param(0)));
        wc.nslots = 3;                                        // 0 = 引数 ev / 1,2 = 新状態の控え

        wc.emit_val(&nexts[0]).expect("段H: total の式が畳めない");
        wc.op_u(0x21, 1);                                     // local.set $t
        wc.emit_val(&nexts[1]).expect("段H: wraps の式が畳めない");  // ← 旧 total を読む(書き戻し前)
        wc.op_u(0x21, 2);                                     // local.set $w
        for (slot, loc) in [(0i64, 1u32), (8, 2)] {
            wc.op(0x41); sleb(slot, &mut wc.b);               // i32.const addr
            wc.op_u(0x20, loc);                               // local.get
            wc.op(0x37); wc.b.push(0x03); wc.b.push(0x00);    // i64.store
        }
        wc.op_u(0x20, 1);                                     // 返り値 = 新 total
        let module = wasm_engine_module(&wc.b, wc.nslots - 1);
        fs::write("engine.wasm", &module).unwrap();

        // 接点そのもの —— seam をデータで置く。既存の web/app 側はこれだけ読めばよい。
        // ⚠️ ここに `"wasm": <目方>` が在った（〜契約 v1.0.0）。**約束ではなく実装の事実**で、
        //    しかも数の三度目の写しだった（文書 / 門の出力 / ここ）⇒ 実装を直すたびに
        //    「契約が動いた」と門③ が鳴る形になっていた（2026-09-06 実測。値位置の if を
        //    書き直しただけで鳴った）。⇒ 契約は **約束だけ**を持つ。目方は §1-1 と門②が持つ。
        //    ▲ 外しても壊れる者は居ないことを撃って確かめた（この欄を読む機械はゼロ）。
        //    ⚠️ それでも契約ファイルの中身が変わる ⇒ **版を上げた**(1.0.0 → 1.1.0)。黙って指紋だけ直さない。
        let seam = format!(
            "{{\n  \"abi\": \"kokkos-engine/1\",\n  \"exports\": {{ \"step\": \"(i64 event) -> i64\", \"memory\": \"i64 スロット列\" }},\n  \"state\": {{ \"{}\": {{ \"slot\": 0, \"byte\": 0 }}, \"{}\": {{ \"slot\": 1, \"byte\": 8 }} }},\n  \"read\": \"new BigInt64Array(memory.buffer)[slot]\",\n  \"reset\": \"host が同じ配列へ書く(状態の持ち主は host)\"\n}}\n",
            state[0], state[1]);
        fs::write("engine_seam.json", &seam).unwrap();

        let html = format!(r#"<!doctype html><meta charset="utf-8"><title>kokkos engine seam(wasm × DOM)</title>
<style>
 :root{{color-scheme:light dark;--ink:#1a1a1a;--bg:#fbfaf7;--dim:#6b6b6b;--line:#dcd8d0;--hit:#a4472f;--on:#4a7c59}}
 @media (prefers-color-scheme:dark){{:root{{--ink:#e8e6e1;--bg:#16161a;--dim:#9a9a9a;--line:#33333a;--hit:#e08a6a;--on:#7fb08e}}}}
 body{{margin:0;padding:2.2rem 1.4rem;background:var(--bg);color:var(--ink);
   font:15px/1.75 ui-sans-serif,system-ui,"Hiragino Sans","Noto Sans JP",sans-serif}}
 main{{max-width:44rem;margin:0 auto}} h1{{font-size:1.2rem;margin:0 0 .3rem}}
 .dim{{color:var(--dim);font-size:.86rem}} code{{font-family:ui-monospace,SFMono-Regular,Menlo,monospace}}
 .row{{display:flex;gap:.5rem;flex-wrap:wrap;margin:1rem 0}}
 button{{font:inherit;padding:.45rem 1rem;border:1px solid var(--line);border-radius:8px;
   background:transparent;color:var(--ink);cursor:pointer}}
 button:hover{{border-color:var(--hit)}}
 .dots{{display:flex;gap:.35rem;margin:.9rem 0}}
 .dot{{width:1.5rem;height:1.5rem;border-radius:50%;border:1px solid var(--line)}}
 .dot.on{{background:var(--on);border-color:var(--on)}}
 .big{{font-size:2rem;font-weight:600;color:var(--hit)}}
 table{{border-collapse:collapse;width:100%;margin:1rem 0;font-size:.9rem}}
 td,th{{border-bottom:1px solid var(--line);padding:.35rem .2rem;text-align:left}}
 td:last-child,th:last-child{{text-align:right;font-family:ui-monospace,monospace}}
 .note{{border-left:3px solid var(--line);padding:.1rem 0 .1rem .9rem;margin:1.2rem 0;font-size:.88rem;color:var(--dim)}}
</style>
<main>
<h1>状態は wasm が持ち、DOM は JS が持つ</h1>
<p class="dim">押すと <code>step(ev)</code> が走る。JS は状態を <b>読んで描くだけ</b>で、規則を一行も持たない。</p>

<div class="dots" id="dots"></div>
<p><span class="big" id="total">0</span> <span class="dim">/ {wrap} …… 折り返した回数 <b id="wraps">0</b></span></p>

<div class="row">
  <button id="b1">+1</button><button id="b3">+3</button><button id="br">reset(host が状態を書く)</button>
</div>

<table>
 <tr><th>この頁が積んでいる wasm</th><td id="sz">—</td></tr>
 <tr><th>seam</th><td><code>step(i64)→i64</code> ＋ <code>memory</code></td></tr>
 <tr><th>JS が状態を読む方法</th><td><code>BigInt64Array(memory.buffer)</code></td></tr>
</table>

<div class="note">
 これが <b>既存の web / app 開発への接点</b>。React でも素の JS でも、読むのは
 <code>engine_seam.json</code> だけ —— <b>言語側に DOM も HTML も足していない</b>。
 melon engine の三原則(状態は言語 / 入力は source 非依存の event / 出力は projection)を
 そのまま web に写しただけ。<code>ev</code> は人のクリックでも agent の判断でも同じ形で届く。
</div>
</main>
<script>
const B="{b64}";
const bin=Uint8Array.from(atob(B),c=>c.charCodeAt(0));
document.getElementById("sz").textContent=bin.length+" B";
let st=null, mem=null;
function render(){{
  const total=Number(mem[0]), wraps=Number(mem[1]);
  document.getElementById("total").textContent=total;
  document.getElementById("wraps").textContent=wraps;
  const d=document.getElementById("dots"); d.innerHTML="";
  for(let i=0;i<{wrap};i++){{
    const e=document.createElement("div"); e.className="dot"+(i<total?" on":""); d.appendChild(e);
  }}
  document.title=`total=${{total}} wraps=${{wraps}}`;
}}
WebAssembly.instantiate(bin).then(({{instance}})=>{{
  st=instance.exports.step; mem=new BigInt64Array(instance.exports.memory.buffer);
  const send=(n)=>{{ st(BigInt(n)); render(); }};
  document.getElementById("b1").onclick=()=>send(1);
  document.getElementById("b3").onclick=()=>send(3);
  document.getElementById("br").onclick=()=>{{ mem[0]=0n; mem[1]=0n; render(); }};  // 状態の持ち主は host
  window.__send=send;                       // 秤(playwright)から叩くための口
  render();
}}).catch(e=>{{document.getElementById("total").textContent="wasm 失敗: "+e;}});
</script>
"#, wrap = wrap, b64 = b64(&module));
        fs::write("demo-engine.html", &html).unwrap();
        println!("段H: step 式 ×2 → wasm {} B / seam engine_seam.json / 自己完結 HTML {} B",
                 module.len(), html.len());
        println!("  → demo-engine.html。検証: node engine_verify.mjs(node 再演 + 実 Chromium のクリック)");
        return;
    }

    let machine = P::new(&fs::read_to_string("machine.json").unwrap()).val();
    let cases = P::new(&fs::read_to_string("cases.json").unwrap()).val();

    let t_build = Instant::now();
    let mut ns: Vec<N> = Vec::new();
    let mut nm = Names::new();
    let root = build(&machine, &mut ns, &mut nm);
    let vs = nm.intern("vs");
    let build_ms = t_build.elapsed().as_secs_f64() * 1000.0;

    let t_emit = Instant::now();
    let mut code: Vec<Ins> = Vec::new();
    emit_stmt(&ns, root, &mut code);
    let mut code_run = code.clone();          // 走らせる列（畳む）。`code` は **数える列**（畳まない）
    let 畳んだ = 畳む(&mut code_run);
    let emit_ms = t_emit.elapsed().as_secs_f64() * 1000.0;

    println!("== 床の梯子 A→D —— 同じ 14-op 契約・同じ machine.json ==\n");
    println!("  畳み: 節 {} 個 / register {} スロット / {:.2} ms", ns.len(), nm.len(), build_ms);
    println!("  線形: 命令 {} 個 / {:.2} ms(どちらも実行前に一度だけ)  ※ 走る列は連なり {} 組を畳んである\n",
             code.len(), emit_ms, 畳んだ);

    // ---- 四床 differential ----
    let (mut pass, mut bad) = (0, 0);
    if let Json::Arr(cs) = &cases {
        for c in cs {
            let (mut hj, mut expect, mut name, mut note) = (Json::Num(0), V::I(0), String::new(), String::new());
            if let Json::Obj(kv) = c { for (k, v) in kv { match k.as_str() {
                "host" => hj = v.clone(), "expect" => expect = to_v(v),
                "name" => name = sof(v).to_string(), "note" => note = sof(v).to_string(), _ => {} } } }

            let ra = top(run_a(&machine, host_a(&hj), 1_000_000));
            let rb = top(run_b(&ns, root, host_b(&hj, &mut nm), 1_000_000, vs));
            let mut arc: Arena = Vec::new();
            let hc = host_cd(&hj, &mut nm, &mut arc);
            let rc_ = { let v = run_c(&ns, root, hc, &mut arc, 1_000_000, vs); top(val_to_v(&arc, v)) };
            let mut ard: Arena = Vec::new();
            let hd = host_cd(&hj, &mut nm, &mut ard);
            let rd = { let v = run_d(&code_run, hd, &mut ard, 1_000_000, vs); top(val_to_v(&ard, v)) };

            let agree = deep_eq(&ra, &rb) && deep_eq(&rb, &rc_) && deep_eq(&rc_, &rd);
            let ok = deep_eq(&rd, &expect);
            let mark = if !agree { "✗ 床が食い違った" } else if ok { "✓ 一致" } else { "⚡ 発散(意図的)" };
            println!("  {}  {:14} A={:>6} B={:>6} C={:>6} D={:>6} py={:>6}  — {}",
                     mark, name, show(&ra), show(&rb), show(&rc_), show(&rd), show(&expect), note);
            if !agree { bad += 1; } else if ok { pass += 1; }
        }
    }
    println!("\n  py 期待値と一致 {} / 四床の食い違い {}(0 = 段を上げても意味は変わっていない)", pass, bad);

    // ---- 速度: sum 1..N を同一プロセスで head-to-head(best-of-5)----
    let bs = match fs::read_to_string("bench_host.json") { Ok(s) => s, Err(_) => return };
    let bench = P::new(&bs).val();
    let (mut hj, mut expect, mut n) = (Json::Num(0), V::I(0), 0i64);
    if let Json::Obj(kv) = &bench { for (k, v) in kv { match k.as_str() {
        "host" => hj = v.clone(), "expect" => expect = to_v(v),
        "n" => if let Json::Num(x) = v { n = *x }, _ => {} } } }

    let (mut ta, mut tb, mut tc, mut td_ms) = (f64::MAX, f64::MAX, f64::MAX, f64::MAX);
    let (mut ra, mut rb, mut rc_, mut rd) = (V::I(0), V::I(0), V::I(0), V::I(0));
    let (mut cells_c, mut cells_d) = (0usize, 0usize);
    for _ in 0..5 {
        let t = Instant::now(); ra = top(run_a(&machine, host_a(&hj), 1_000_000));
        ta = ta.min(t.elapsed().as_secs_f64() * 1000.0);

        let t = Instant::now(); rb = top(run_b(&ns, root, host_b(&hj, &mut nm), 1_000_000, vs));
        tb = tb.min(t.elapsed().as_secs_f64() * 1000.0);

        let mut arc: Arena = Vec::new(); let hc = host_cd(&hj, &mut nm, &mut arc);
        let t = Instant::now(); let vc = run_c(&ns, root, hc, &mut arc, 1_000_000, vs);
        tc = tc.min(t.elapsed().as_secs_f64() * 1000.0);
        cells_c = arc.len(); rc_ = top(val_to_v(&arc, vc));

        let mut ard: Arena = Vec::new(); let hd = host_cd(&hj, &mut nm, &mut ard);
        let t = Instant::now(); let vd = run_d(&code_run, hd, &mut ard, 1_000_000, vs);
        td_ms = td_ms.min(t.elapsed().as_secs_f64() * 1000.0);
        cells_d = ard.len(); rd = top(val_to_v(&ard, vd));
    }
    let ok = deep_eq(&ra, &rb) && deep_eq(&rb, &rc_) && deep_eq(&rc_, &rd) && deep_eq(&rd, &expect);

    println!("\n速度(sum 1..{} = {}, 四床 + py 一致 {}) best-of-5:", n, show(&rd), ok);
    println!("  床A naive  文字列 dispatch + HashMap<String> + Rc : {:>7.1} ms   —", ta);
    println!("  床B 畳み   enum 節 + スロット添字                 : {:>7.1} ms   A比 {:.1}x", tb, ta / tb);
    println!("  床C arena  ＋ Copy 値 + pair arena(Rc を外す)     : {:>7.1} ms   A比 {:.1}x / B比 {:.1}x", tc, ta / tc, tb / tc);
    println!("  床D 線形   ＋ 線形命令列 + オペランドスタック      : {:>7.1} ms   A比 {:.1}x / C比 {:.1}x", td_ms, ta / td_ms, tc / td_ms);
    println!("\n  🔴 回収しない代償: arena は 1 走行で C {} セル / D {} セル({:.1} MB)まで伸びた。",
             cells_c, cells_d, (cells_d * std::mem::size_of::<(Val, Val)>()) as f64 / 1_048_576.0);
    println!("     refcount を外す = 「いつ解放してよいか」を知る者を外すこと。埋めるのは GC か **所有**。");

    // ---- 床はもう底か —— ns/命令 で判る(計測とは別建ての計数走行)----
    {
        let mut arn: Arena = Vec::new();
        let hn = host_cd(&hj, &mut nm, &mut arn);
        let (ins, rounds) = count_d(&code, hn, &mut arn, 1_000_000);
        // 🔴 **二つの量を分けて数える**（2026-09-06、連なりを畳んだ日に分かれた）。
        //   `ins` = 畳む前 = 機械の意味が要求する仕事 = **塔の厚み**。畳んでも減らない。
        //   `手数` = 畳んだ後 = 床が実際に dispatch した回数 = **床の手数**。
        //   ⚠️ 前は一つの数で両方を名乗っていた。畳んだ瞬間、その名は範囲を偽る
        //     ——「実際に回した命令」と書いてある数が、実際に回した数でなくなる。
        let mut arm: Arena = Vec::new();
        let hm = host_cd(&hj, &mut nm, &mut arm);
        let (手数, _) = count_d(&code_run, hm, &mut arm, 1_000_000);
        let ns_per = td_ms * 1e6 / ins as f64;
        let ns_手 = td_ms * 1e6 / 手数 as f64;
        // ⚠️ 数の隣に **符牒**を置く。門は符牒で錨を取る —— 文言に錨づけると、
        //   言い方を正した日に静かに外れる（2026-09-05 に一度、2026-09-06 にもう一度踏んだ型）。
        println!("\n  床D 塔の厚み(畳む前): {} 命令 [D-tower](trampoline {} 周)⇒ {:.2} ns/意味命令 ≈ {:.1} cycles @3GHz",
                 ins, rounds, ns_per, ns_per * 3.0);
        println!("     床の手数(畳んだ後):   {} dispatch [D-dispatch] ⇒ {:.2} ns/dispatch —— **{:.1}% を畳みで落とした**",
                 手数, ns_手, (ins - 手数) as f64 * 100.0 / ins as f64);
        println!("     ◆ 当て先は数えて選んだ: 動的な四連なり `Load Lit Eq JmpF`(＝機械の命令選り分け)が");
        println!("       単独で dispatch の 38.7% を占めていた。畳んで 25.6 → {:.1} ms。", td_ms);
        println!("     ⚠️ 畳んでも **塔の厚みは 1 命令も減っていない**。減ったのは床の手数だけ。");
        println!("     ▲ だが本題はそこではない —— **ループ 1 反復(加算1+減算1+条件1)あたり床命令 {:.0} 個**。",
                 ins as f64 / 1000.0);
        println!("        この {:.0}x は塔(機械そのものの解釈)の厚みで、床をどれだけ磨いても減らない。",
                 ins as f64 / 1000.0);
        println!("        ⇒ 残りを取るのは床の最適化でなく **段4(機械のコンパイル)**。");
    }

    // ---- 段E / 段F: 塔を畳む -------------------------------------------------
    // object 程式は host の register `td` に data として載っている(その car)。
    let mut td: Option<&Json> = None;
    if let Json::Obj(kvs) = &hj { for (k, v) in kvs { if k == "td" { td = Some(v); } } }
    let prog = match td { Some(t) if is_p(t) => pcar(t), _ => { if !ok || bad > 0 { std::process::exit(1); } return; } };

    println!("\n== 塔を畳む —— 機械を *この程式* に特殊化する(第一 Futamura 射影)==\n");
    let t_sp = Instant::now();
    let mut cp = Comp::new();
    match cp.emit_val(prog) {
        Err(e) => { println!("  段E: この切片では畳めない —— {}", e);
                    if !ok || bad > 0 { std::process::exit(1); } return; }
        Ok(()) => {}
    }
    let spec_ms = t_sp.elapsed().as_secs_f64() * 1000.0;
    println!("  特殊化: 機械 1,615 節 + 程式 ⇒ **命令 {} 個 / スロット {} 個** ({:.3} ms、一度だけ)",
             cp.code.len(), cp.nslots, spec_ms);

    // 段E を回す(best-of-5、速いので 100 回まわして 1 回あたりを出す)
    let (mut te, mut ins_e, mut ve, mut cells_e) = (f64::MAX, 0u64, V::I(0), 0usize);
    for _ in 0..5 {
        let mut ar: Arena = Vec::new(); let mut cnt = 0u64;
        let t = Instant::now();
        let mut last = Val::I(0);
        for _ in 0..100 { ar.clear(); cnt = 0; last = run_e(&cp.code, cp.nslots, &mut ar, &mut cnt); }
        te = te.min(t.elapsed().as_secs_f64() * 1000.0 / 100.0);
        ins_e = cnt; cells_e = ar.len(); ve = val_to_v(&ar, last);
    }

    // 段F: x86-64 を吐いて走らせる
    let jit_res = jit(&cp.code);
    let (mut tf, mut vf, mut code_bytes) = (f64::MAX, V::I(0), 0usize);
    let mut jit_note = String::new();
    match &jit_res {
        Err(e) => jit_note = format!("段F: 吐けない —— {}", e),
        Ok(bytes) => {
            code_bytes = bytes.len();
            let f = Jitted::new(bytes);
            let mut slots = vec![0i64; cp.nslots as usize];
            for _ in 0..5 {
                let t = Instant::now();
                let mut r = 0i64;
                for _ in 0..1000 { for x in slots.iter_mut() { *x = 0; } r = f.call(&mut slots); }
                tf = tf.min(t.elapsed().as_secs_f64() * 1000.0 / 1000.0);
                vf = V::I(r);
            }
        }
    }

    let e_ok = deep_eq(&ve, &expect);
    let f_ok = jit_res.is_ok() && deep_eq(&vf, &expect);
    println!("  段E 特殊化命令列 : {:>9.4} ms   結果 {} ({})   実行命令 {} 個 = 1 反復あたり {:.0} 個",
             te, show(&ve), if e_ok { "一致" } else { "✗ 不一致" }, ins_e, ins_e as f64 / n as f64);
    println!("       ⇒ {:.2} ns/命令(床D の ns/dispatch と同水準 = VM の下限は本当にこの辺)/ {:.1} ns/反復",
             te * 1e6 / ins_e as f64, te * 1e6 / n as f64);
    println!("       ⇒ **arena {} セル**。効果位置の cons(値が死んでいる列)は確保ごと消えた。", cells_e);
    if jit_res.is_ok() {
        println!("  段F x86-64 直吐き: {:>9.4} ms   結果 {} ({})   機械語 {} B(mmap+W^X、外部 assembler なし)",
                 tf, show(&vf), if f_ok { "一致" } else { "✗ 不一致" }, code_bytes);
        println!("       ⇒ {:.1} ns/反復 ≈ {:.0} cycles @3GHz。頂を rax に載せたまま持ち歩く(一段の割付)",
                 tf * 1e6 / n as f64, tf * 1e6 / n as f64 * 3.0);
        // 🔴 予想と実測(2026-09-06)。**予想は消さない**:
        //   予想「素朴なスタック直訳 ⇒ register 割付をすればまだ **数倍**」
        //   実測「頂だけを rax に載せる一段の割付で **1.37x**(0.0055 → 0.0040 ms / 187 → 166 B)」
        //   ⇒ **外れ**。push/pop は消えたが、そこが支配項ではなかった。
        //     残るのは *局所変数* が毎回メモリを往復している所（Load/Store が rdi 経由のまま）。
        //     ⇒ 「まだ数倍」は **本物の割付**(局所を register に固定)の話で、頂の一段では届かない。
        println!("          ⇒ 頂の一段で 1.37x。**まだ数倍**と書いていたが、そこまでは行かない ——");
        println!("             残りは局所変数が毎回メモリを往復している所(Load/Store)。そこが本物の割付。");
    } else { println!("  {}", jit_note); }

    println!("\n  梯子の全景(sum 1..{}):", n);
    println!("    床A naive   {:>9.1} ms   1.0x", ta);
    println!("    床D 線形    {:>9.1} ms   {:>6.1}x", td_ms, ta / td_ms);
    println!("    段E 特殊化  {:>9.4} ms   {:>6.0}x   ← 塔が消えた分", te, ta / te);
    if jit_res.is_ok() {
        println!("    段F native  {:>9.4} ms   {:>6.0}x   ← アセンブラ級", tf, ta / tf);
        println!("\n  🔴 塔の厚み: 床D は 1 反復に床命令 6,103 個。段E は {:.0} 個。**{:.0} 分の 1**。",
                 ins_e as f64 / n as f64, 6103.0 / (ins_e as f64 / n as f64));
        println!("     段E→段F は同じ命令数を native で回す差 = {:.1}x。⇒ 勝ちの本体は **命令を減らすこと**であって",
                 te / tf);
        println!("     機械語を吐くことではない。native 化は最後の一絞り。");
    }

    if !ok || bad > 0 || !e_ok || (jit_res.is_ok() && !f_ok) { std::process::exit(1); }
}
