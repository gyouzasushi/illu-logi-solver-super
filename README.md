# illu-logi-solver-super

イラストロジック（ノノグラム、お絵かきロジック）のルールベースソルバです。
[illu-logi-solver](https://github.com/gyouzasushi/illu-logi-solver) のアイデア
（人間が使う8つの行内推論規則・根拠つきヒント・総当たり検証）を引き継いで、
設計を一から作り直したものです。

- 各行・各列の「黒マスの連続長の並び」という制約から、8つの基本推論規則を
  適用して盤面を埋めます。30×30程度のパズルなら数msでフルソルブできます。
- 背理法や二択試行のような線形推論を超える探索は行いません（非目標）。
  そのぶん、確定した各マスについて「なぜそう言えるか」（`Reason`）を常に
  説明できます。人間向けヒント機能はこの資産の上に成り立っています。
- 一意に解けないパズルは `Outcome::Stuck` という**正常な結果**として返り、
  その時点までに確定した部分盤面をそのまま読み出せます。

## 設計

旧実装からの主な変更点は、機能ではなく構造です。

| | 旧 (`illu-logi-solver`) | 新 (`illu-logi-solver-super`) |
|---|---|---|
| セル状態 | 行の `Line` と列の `Line` が同じセルを二重保持し手動同期 | `Grid` 一枚が唯一の所有者 |
| 行の推論 | 可変な `Line`（候補範囲・キューを永続保持） | 純関数 `LineAnalysis::compute`（毎回ゼロから不動点計算） |
| 座標変換 | 各所に転置ロジックが分散 | `LineId::cell` / `orthogonal_at` の2メソッドに集約 |
| ヒントとステップ実行 | `hint()` と `advance()` が別ロジック（残留キューの特例あり） | `next_step()` ＝「`hint()` を適用する」そのもの（食い違いが構成上あり得ない） |
| 一意に解けない | `Err(Indeterminate)` | `Ok(Outcome::Stuck)` |
| 入力検証 | `Solver::new` が `Result` | `Clues::new` だけが `Result`（parse, don't validate）。以降の構築は失敗しない |

検証済みの難所（候補ブロックIDの不動点計算と8つの推論規則の本体）は
旧実装から逐語移植し、総当たりテストで**推論力が1マスも変わっていない**
ことを確認しています（後述のテスト参照）。

## 制約の与え方

制約は `Clues::new(rows, cols)` に「行の制約（上から順）」「列の制約
（左から順）」として渡します。各要素はその行・列の「黒マスの連続長」を
左（上）から順に並べたもので、空リストはその行・列が全マス白であることを
表します。

```rust
use illu_logi_solver_super::Clues;

// 5行5列。行は上から、列は左から。
let clues = Clues::new(
    vec![vec![2, 1], vec![3], vec![2, 2], vec![1, 2], vec![1, 1]],
    vec![vec![3, 1], vec![4], vec![1, 1], vec![2], vec![1, 2]],
)
.unwrap();
```

高さと幅は独立に決まり（`height = rows.len()`, `width = cols.len()`）、
長方形のパズルもそのまま扱えます。制約は構築時に検証されます:
ブロックサイズ0は `ClueError::ZeroBlock`、どう詰めても線に収まらない制約は
`ClueError::TooLong` で拒否されます。**検証はここだけ**で、検証済みの
`Clues` を受け取る `Solver::new` / `Session::new` は失敗しません。

## `Solver`: 推論エンジン

```rust
use illu_logi_solver_super::{Clues, Outcome, Solver};

let clues = Clues::new(
    vec![vec![2, 1], vec![3], vec![2, 2], vec![1, 2], vec![1, 1]],
    vec![vec![3, 1], vec![4], vec![1, 1], vec![2], vec![1, 2]],
)
.unwrap();
let mut solver = Solver::new(clues);
assert_eq!(solver.solve().unwrap(), Outcome::Solved);
assert!(solver.judge());
println!("{solver}");
```

- `solve()`: 確定できる限り推論し尽くす。全確定なら `Ok(Solved)`、矛盾なく
  推論が尽きれば `Ok(Stuck)`（部分盤面は `grid()` で読める）、矛盾があれば
  `Err(Contradiction)`。
- `next_step()`: 最も安い推論ステップを1件だけ実行する。可視化やステップ
  実行に。
- `hint()`: 次の1手とその根拠を非破壊で求める。**`next_step()` は
  「`hint()` が返した確定を盤面に適用する」ことそのものとして実装されて
  いる**ので、両者が食い違うことはありません。
- `with_grid(clues, grid)`: 既知のマスを種にして構築。
- `grid()` / `state(i, j)` / `judge()`: 盤面の読み出しと制約充足の判定。

矛盾エラー（`Contradiction`）の座標はすべて盤面座標です。セル状態の
所有者が `Grid` 一枚なので、旧実装にあった「行内座標と盤面座標の混在
（転置バグ）」は構造的に起こりません。

## `Session`: ユーザーとの対話用の薄い層

ユーザーが盤面を埋めながらヒントや間違いチェックを受ける、といった対話的な
用途向けのラッパーです。`Session` 自身はソルバの状態を一切持たず、ユーザーの
盤面と操作履歴だけを真実（ground truth）として持ちます。`hint`/`deduce`/
`judge`/`mistakes` を呼ぶたびに、現在の盤面から使い捨ての `Solver` を新品で
構築して問い合わせます（フルソルブが数msなので、1操作ごとに作り直しても
実用上問題ありません）。

- `set(i, j, state)`: 盤面を書き換え、履歴に記録する（未確定への巻き戻しも
  ただの代入なので常に正しい）。
- `hint()`: 次の1手とその根拠を非破壊で求める。
- `deduce()`: 現盤面から確定できる範囲まで推論した結果の盤面を返す。
- `judge()`: 現盤面が制約を満たしているか。
- `mistakes()`: **制約だけから**求めた解答と現盤面を突き合わせ、食い違う
  確定セルの座標を返す（ユーザーの誤記入がヒントの前提に混ざらない、
  独立した間違い検出）。
- `undo()` / `rollback(t)`: 履歴の切り詰め＋盤面再構築。

```rust
use illu_logi_solver_super::{CellState, Clues, Session};

let clues = Clues::new(vec![vec![2], vec![]], vec![vec![1], vec![1]]).unwrap();
let mut session = Session::new(clues);
session.set(0, 0, CellState::Black);
session.set(0, 1, CellState::Black);
session.set(1, 0, CellState::White);
session.set(1, 1, CellState::White);
assert!(session.judge());
assert_eq!(session.mistakes().unwrap(), Vec::new());
```

## ヒント（`Hint`）の中身

`hint()` が返す `Hint` は、確定操作 `step`（どの行・列の、どの範囲を、
何色に、なぜ塗れるか）に加えて、その `step` を算出したのと同一の
スナップショットから読み出した行コンテキスト（`candidates`: セルごとの
候補ブロックID範囲、`blocks`: ブロックごとの配置可能範囲とサイズ）を
持ちます。これらを組み合わせれば「なぜこのマスが確定するのか」を人間向けに
説明する文言を組み立てられます。文言化はUI側の仕事とし、このクレートは
生データだけを提供します。日本語の文言化の実例が `examples/solve.rs` に
あります:

```sh
cargo run --example solve -- --steps examples/data/15x15.txt
#    1. 行0 のマス5..7 を黒に確定: ブロック1（長さ4）はマス3..9の
#       どこかに入るしかなく、左右どちらに寄せてもここは必ず黒
#    ...
```

## テスト

```sh
cargo test
```

- `tests/soundness.rs`: 実在する盤面から生成した制約に対して、誤確定や偽の
  矛盾が出ないことを総当たりで検証します（4×4全列挙、5×5/7×7/10×10
  ランダム、長方形5×8/7×3ランダム。シードは旧実装と同一）。
- `tests/completeness.rs`: 8規則による行推論を「完全な行推論（全配置列挙の
  不動点）」と比較し、取りこぼし（gap）が悪化していないことを検証します。
  実測値は旧実装と**完全一致**しています（4×4: 51,234件解け gap 0、
  5×5: 14,846件 gap 0、7×7: 6,618件 gap 2）。
- `tests/api.rs`: 公開APIの仕様テスト。`hint()` と `next_step()` の同一性、
  `solve()` と逐次実行の閉包一致のプロパティテスト、旧実装で実証された
  3件のバグ（巻き戻しの残留・推論し尽くした後の伝播漏れ・矛盾座標の転置）
  が新設計では発生し得ないことの回帰テストを含みます。
