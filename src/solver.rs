//! 推論エンジン。
//!
//! フルソルブは [`Solver::solve`]、可視化やステップ実行は
//! [`Solver::next_step`] / [`Solver::hint`]。どちらの経路も同じ閉包
//! （同じ最終盤面）に到達する。対話的な用途には [`crate::Session`] を使う。

use crate::analysis::LineAnalysis;
use crate::clue::{Clues, LineId};
use crate::grid::{CellState, Color, Grid, GridMismatch, render_board};
use crate::rules::{Deduction, RULES, Reason};
use std::collections::VecDeque;
use std::fmt;
use std::ops::Range;
use thiserror::Error;

/// 推論を尽くしたときの結果。どちらも「矛盾なく推論が終わった」ことを
/// 表す正常な値で、一意に解けないことはエラーではない。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    /// 全マスが一意に確定した。
    Solved,
    /// 矛盾はないが、これ以上確定できない（複数の解があり得る）。
    /// そこまでに確定した部分盤面は [`Solver::grid`] で読み出せる。
    Stuck,
}

/// 推論中に見つかった矛盾。制約を満たす解が存在しない。
/// 座標はすべて盤面座標（`row`/`col`）。
#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum Contradiction {
    /// `line` 上の推論が根拠 `reason` でセル `(row, col)` を `attempted` に
    /// 塗ろうとしたが、既に逆の状態 `current` が入っていた。
    #[error(
        "contradiction at ({row}, {col}): {line} tried to set {attempted:?} by {reason:?}, but {current:?} is already set"
    )]
    CellConflict {
        /// 矛盾したセルの行番号。
        row: usize,
        /// 矛盾したセルの列番号。
        col: usize,
        /// 既に入っていた状態。
        current: CellState,
        /// 塗ろうとした色。
        attempted: Color,
        /// 塗ろうとした推論が走っていた行・列。
        line: LineId,
        /// 塗ろうとした推論の根拠。
        reason: Reason,
    },
    /// `line` 上でブロック `block` の置き場所がどこにもない。特定のセルの
    /// 書き込み衝突ではなく行レベルの矛盾なので、`CellConflict` とは
    /// 区別する。
    #[error("no valid placement for block {block} on {line}: it does not fit anywhere")]
    NoPlacement {
        /// 矛盾を検出した行・列。
        line: LineId,
        /// 置き場所がなかったブロックのID（0始まり）。
        block: usize,
    },
}

/// 1回の確定操作: どの行・列で、どんな確定（範囲・色・根拠）が起きたか。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Step {
    /// 確定が起きた行・列。
    pub line: LineId,
    /// 確定の中身（行・列内の範囲、色、根拠）。
    pub deduction: Deduction,
    /// `deduction.range` のうち、この確定で新たに塗られるセルのオフセット
    /// （既に塗られていたセルは含まない）。
    pub changed: Vec<usize>,
}

/// ヒント算出と同一スナップショットにおける、1ブロックの配置可能範囲とサイズ。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct HintBlock {
    /// このブロックの長さ（制約に書かれた値そのもの）。
    pub size: usize,
    /// このブロックが配置され得る範囲（半開区間）。ブロック全体（`size`
    /// マス分）がこの範囲に収まり得ることを表す（＝先頭マスの取り得る
    /// 位置は `possible_placement.start..=possible_placement.end - size`）。
    pub possible_placement: Range<usize>,
}

/// 確定の根拠まで含む、自己完結したヒント。
///
/// `candidates`/`blocks` は `step` を算出したのと同一のスナップショットから
/// 読み出した行コンテキストで、常に `step` と整合する。人間向けの文言化は
/// UI層の仕事（実例は `examples/solve.rs`）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Hint {
    /// 次に実行できる確定操作。[`Solver::next_step`] が次に行うことと同一。
    pub step: Step,
    /// `step.deduction.range` と同じ並びで、各セルの候補ブロックID範囲。
    pub candidates: Vec<Range<usize>>,
    /// `step.line` の全ブロックの配置可能範囲とサイズ（制約と同じ並び）。
    pub blocks: Vec<HintBlock>,
}

/// 制約（＋任意で初期盤面）を受け取って推論する、ルールベースのエンジン。
///
/// 構築後に外部から盤面を書き換える手段は持たない。対話的な用途には
/// [`crate::Session`] を使う。
///
/// # 使用例
///
/// ```
/// use illu_logi_solver_super::{CellState, Clues, Outcome, Solver};
///
/// // 5x5。行は上から、列は左から、黒マスが連続する長さの並び。
/// let clues = Clues::new(
///     vec![vec![2, 1], vec![3], vec![2, 2], vec![1, 2], vec![1, 1]],
///     vec![vec![3, 1], vec![4], vec![1, 1], vec![2], vec![1, 2]],
/// )
/// .unwrap();
/// let mut solver = Solver::new(clues);
/// assert_eq!(solver.solve().unwrap(), Outcome::Solved);
/// assert!(solver.judge());
/// assert_eq!(solver.grid().get(0, 0), CellState::Black);
/// ```
pub struct Solver {
    clues: Clues,
    grid: Grid,
    /// まだ推論し尽くしていない可能性のある行・列のワークリスト。
    /// 不変条件: セルが書き換わったら、そのセルを含む行・列は
    /// 消化されるまでこのリストに載っている。
    dirty: VecDeque<LineId>,
    /// `dirty` への重複投入を防ぐフラグ（[`Solver::ordinal`] 添字）。
    in_queue: Vec<bool>,
    steps: usize,
}

impl Solver {
    /// 制約から空盤面のソルバを構築する。
    ///
    /// 制約は [`Clues::new`] で検証済みなので、この構築は失敗しない。
    pub fn new(clues: Clues) -> Self {
        let grid = Grid::new(clues.height(), clues.width());
        Self::from_parts(clues, grid)
    }

    /// 確定済みのセルを含む盤面を種にしてソルバを構築する。
    ///
    /// 盤面の次元が制約の次元（`height` 行 × `width` 列）と一致することを
    /// 検証する。種の盤面が矛盾しているかどうかはここでは調べず、後続の
    /// [`Solver::solve`] / [`Solver::hint`] が推論の過程で検出する。
    pub fn with_grid(clues: Clues, grid: Grid) -> Result<Self, GridMismatch> {
        if grid.height() != clues.height() {
            return Err(GridMismatch::HeightMismatch {
                expected: clues.height(),
                actual: grid.height(),
            });
        }
        if grid.width() != clues.width() {
            return Err(GridMismatch::WidthMismatch {
                expected: clues.width(),
                actual: grid.width(),
            });
        }
        Ok(Self::from_parts(clues, grid))
    }

    fn from_parts(clues: Clues, grid: Grid) -> Self {
        // 全行・全列を dirty で開始し、種の盤面の情報も最初の巡回で消化する。
        let dirty: VecDeque<LineId> = clues.lines().collect();
        let in_queue = vec![true; clues.height() + clues.width()];
        Self {
            clues,
            grid,
            dirty,
            in_queue,
            steps: 0,
        }
    }

    /// `line` の `dirty`/`in_queue` 用の通し番号。行は `0..height`、
    /// 列は `height..height+width`。
    fn ordinal(&self, line: LineId) -> usize {
        match line {
            LineId::Row(i) => i,
            LineId::Col(j) => self.clues.height() + j,
        }
    }

    fn mark_dirty(&mut self, line: LineId) {
        let ordinal = self.ordinal(line);
        if !self.in_queue[ordinal] {
            self.in_queue[ordinal] = true;
            self.dirty.push_back(line);
        }
    }

    /// 確定 `deduction` を `line` に適用する。新たに塗れたセルごとに直交する
    /// 行・列を dirty にし、1セルでも塗れたら `line` 自身も dirty にする
    /// （自行の分析がさらに狭まり得るため）。
    fn apply(&mut self, line: LineId, deduction: &Deduction) -> Result<bool, Contradiction> {
        let mut changed = false;
        for offset in deduction.range.clone() {
            let (row, col) = line.cell(offset);
            match self.grid.paint(row, col, deduction.color) {
                Ok(true) => {
                    changed = true;
                    self.mark_dirty(line.orthogonal_at(offset));
                }
                Ok(false) => {}
                Err(current) => {
                    return Err(Contradiction::CellConflict {
                        row,
                        col,
                        current,
                        attempted: deduction.color,
                        line,
                        reason: deduction.reason,
                    });
                }
            }
        }
        if changed {
            self.steps += 1;
            self.mark_dirty(line);
        }
        Ok(changed)
    }

    fn analyze(
        &self,
        line: LineId,
        buf: &mut Vec<CellState>,
    ) -> Result<LineAnalysis, Contradiction> {
        self.grid.line_cells(line, buf);
        LineAnalysis::compute(self.clues.blocks(line), buf)
            .map_err(|block| Contradiction::NoPlacement { line, block })
    }

    /// 確定できる限り推論し尽くす。
    ///
    /// 8規則で確定できるマスをすべて埋め切ったら [`Outcome::Solved`]、
    /// 矛盾なく推論が尽きても未確定マスが残る（＝この規則群では一意に
    /// 確定できない）場合は [`Outcome::Stuck`] を返す。どちらの場合も
    /// 盤面は [`Solver::grid`] で読み出せる。矛盾を見つけたら
    /// `Err(Contradiction)` を返す。
    ///
    /// [`Solver::next_step`] を繰り返すのと同じ閉包に到達するが、
    /// 1ステップごとに全行を探索し直さないぶん速い（30×30で数ms）。
    pub fn solve(&mut self) -> Result<Outcome, Contradiction> {
        let mut buf = Vec::new();
        let mut deductions = Vec::new();
        while let Some(line) = self.dirty.pop_front() {
            let ordinal = self.ordinal(line);
            self.in_queue[ordinal] = false;
            let analysis = self.analyze(line, &mut buf)?;
            deductions.clear();
            for rule in RULES {
                rule(&analysis, &mut deductions);
            }
            for deduction in &deductions {
                self.apply(line, deduction)?;
            }
        }
        Ok(if self.grid.is_complete() {
            Outcome::Solved
        } else {
            Outcome::Stuck
        })
    }

    /// 現盤面から、次の1手とその根拠を非破壊で求める。
    ///
    /// 8つの規則を安い順に、各規則を全行・全列（`Row 0..` → `Col 0..`）へ
    /// 試し、最初に見つかった確定を返す。もう何も確定できなければ
    /// `Ok(None)`。分析の過程で矛盾を見つけたら `Err`。
    ///
    /// 返る [`Hint`] の `step` は、次に [`Solver::next_step`] を呼んだ場合に
    /// 実行される操作と同一。
    pub fn hint(&self) -> Result<Option<Hint>, Contradiction> {
        // 行の分析は純関数なので、規則をまたいで使い回せる（遅延計算）。
        let lines: Vec<LineId> = self.clues.lines().collect();
        let mut analyses: Vec<Option<LineAnalysis>> = (0..lines.len()).map(|_| None).collect();
        let mut buf = Vec::new();
        let mut deductions = Vec::new();
        for rule in RULES {
            for (k, &line) in lines.iter().enumerate() {
                if analyses[k].is_none() {
                    analyses[k] = Some(self.analyze(line, &mut buf)?);
                }
                let analysis = analyses[k].as_ref().unwrap();
                deductions.clear();
                rule(analysis, &mut deductions);
                let Some(deduction) = deductions.first().cloned() else {
                    continue;
                };
                let changed = deduction
                    .range
                    .clone()
                    .filter(|&j| analysis.cells[j] != deduction.color.into())
                    .collect();
                let candidates = deduction
                    .range
                    .clone()
                    .map(|j| analysis.candidates[j].clone())
                    .collect();
                let blocks = analysis
                    .blocks
                    .iter()
                    .map(|b| HintBlock {
                        size: b.size,
                        possible_placement: b.possible_placement.clone(),
                    })
                    .collect();
                return Ok(Some(Hint {
                    step: Step {
                        line,
                        deduction,
                        changed,
                    },
                    candidates,
                    blocks,
                }));
            }
        }
        Ok(None)
    }

    /// 最も安い推論ステップを1件だけ実行する。
    ///
    /// [`Solver::hint`] が返す確定をそのまま盤面に適用して返す。もう何も
    /// 確定できなければ `Ok(None)`。可視化やステップ実行向けで、1回ごとに
    /// 全行を探索し直すためフルソルブには [`Solver::solve`] より遅い。
    pub fn next_step(&mut self) -> Result<Option<Step>, Contradiction> {
        let Some(hint) = self.hint()? else {
            return Ok(None);
        };
        self.apply(hint.step.line, &hint.step.deduction)?;
        Ok(Some(hint.step))
    }

    /// パズルの制約。
    pub fn clues(&self) -> &Clues {
        &self.clues
    }

    /// 現在の盤面。
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// セル `(row, col)` の現在の状態（`self.grid().get(row, col)` の別名）。
    pub fn state(&self, row: usize, col: usize) -> CellState {
        self.grid.get(row, col)
    }

    /// 現盤面が制約をすべて満たしているか（[`Grid::satisfies`] 参照）。
    pub fn judge(&self) -> bool {
        self.grid.satisfies(&self.clues)
    }

    /// これまでに盤面を変化させた確定操作の数
    /// （[`Solver::solve`] / [`Solver::next_step`] の内部カウント）。
    pub fn steps_applied(&self) -> usize {
        self.steps
    }

    /// 3面併記のデバッグ表示: 素の盤面、行の分析で確定ブロックIDを添えた
    /// 盤面、列の分析で確定ブロックIDを添えた盤面を横に並べる。黒セルは
    /// 候補ブロックが一意ならそのIDを数字1桁（10以上は `#`）で表示する。
    pub fn debug_display(&self) -> String {
        let confirmed_ids = |line: LineId| -> Option<Vec<Option<usize>>> {
            let mut buf = Vec::new();
            let analysis = self.analyze(line, &mut buf).ok()?;
            Some(
                (0..analysis.n())
                    .map(|j| analysis.confirmed_id(j))
                    .collect(),
            )
        };
        let by_rows: Vec<_> = (0..self.grid.height())
            .map(|i| confirmed_ids(LineId::Row(i)))
            .collect();
        let by_cols: Vec<_> = (0..self.grid.width())
            .map(|j| confirmed_ids(LineId::Col(j)))
            .collect();
        let id_char = |id: Option<usize>| match id {
            Some(id) if id < 10 => char::from_digit(id as u32, 10).unwrap(),
            Some(_) => '#',
            None => 'o',
        };
        let plain_char = |state: CellState| match state {
            CellState::Unconfirmed => '.',
            CellState::White => 'x',
            CellState::Black => 'o',
        };
        let (height, width) = (self.grid.height(), self.grid.width());
        let plain = render_board(height, width, |r, c| plain_char(self.grid.get(r, c)));
        let rows_view = render_board(height, width, |r, c| match self.grid.get(r, c) {
            CellState::Black => id_char(by_rows[r].as_ref().and_then(|ids| ids[c])),
            state => plain_char(state),
        });
        let cols_view = render_board(height, width, |r, c| match self.grid.get(r, c) {
            CellState::Black => id_char(by_cols[c].as_ref().and_then(|ids| ids[r])),
            state => plain_char(state),
        });
        let mut out = String::new();
        for ((a, b), c) in plain.iter().zip(&rows_view).zip(&cols_view) {
            out.push_str(a);
            out.push_str("  ");
            out.push_str(b);
            out.push_str("  ");
            out.push_str(c);
            out.push('\n');
        }
        out
    }
}

/// 現在の盤面のテキスト表示（[`Grid`] の `Display` に委譲）。
impl fmt::Display for Solver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.grid.fmt(f)
    }
}
