//! 盤面。セル状態を持つだけの値型
//!
//! 行・列としての読み出しは [`crate::LineId`] 経由で行う

use crate::clue::{Clues, LineId};
use std::fmt;
use thiserror::Error;

/// 1マスの状態
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CellState {
    /// まだ黒とも白とも確定していない
    Unconfirmed,
    /// 白（塗らない）に確定済み
    White,
    /// 黒（塗る）に確定済み
    Black,
}

/// 推論が書き込む確定色。白か黒のみで、`Unconfirmed` を表現できない
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    /// 白（塗らない）
    White,
    /// 黒（塗る）
    Black,
}

impl From<Color> for CellState {
    fn from(color: Color) -> Self {
        match color {
            Color::White => CellState::White,
            Color::Black => CellState::Black,
        }
    }
}

/// [`Grid::from_rows`] / [`crate::Solver::with_grid`] の次元検証エラー
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum GridMismatch {
    /// `from_rows` に渡した行列の `row` 行目の列数が0行目と食い違う
    #[error("row {row} has {actual} cells but row 0 has {expected}")]
    RaggedRows {
        /// 列数が食い違った行番号
        row: usize,
        /// 0行目の列数
        expected: usize,
        /// 実際の `row` 行目の列数
        actual: usize,
    },
    /// 盤面の高さが制約の高さと一致しない
    #[error("grid has {actual} rows but clues expect {expected}")]
    HeightMismatch {
        /// 制約が期待する高さ
        expected: usize,
        /// 実際の盤面の高さ
        actual: usize,
    },
    /// 盤面の幅が制約の幅と一致しない
    #[error("grid has {actual} columns but clues expect {expected}")]
    WidthMismatch {
        /// 制約が期待する幅
        expected: usize,
        /// 実際の盤面の幅
        actual: usize,
    },
}

/// H×W の盤面。ただのデータ（値型）で、推論の状態は一切持たない
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Grid {
    height: usize,
    width: usize,
    cells: Vec<CellState>,
}

impl Grid {
    /// 全マス未確定の盤面を作る
    pub fn new(height: usize, width: usize) -> Self {
        Self {
            height,
            width,
            cells: vec![CellState::Unconfirmed; height * width],
        }
    }

    /// 行ごとのセル状態から盤面を作る。全行の列数が揃っている必要がある
    pub fn from_rows(rows: Vec<Vec<CellState>>) -> Result<Self, GridMismatch> {
        let height = rows.len();
        let width = rows.first().map_or(0, Vec::len);
        for (row, cells) in rows.iter().enumerate() {
            if cells.len() != width {
                return Err(GridMismatch::RaggedRows {
                    row,
                    expected: width,
                    actual: cells.len(),
                });
            }
        }
        Ok(Self {
            height,
            width,
            cells: rows.into_iter().flatten().collect(),
        })
    }

    /// 盤面の高さ
    pub fn height(&self) -> usize {
        self.height
    }

    /// 盤面の幅
    pub fn width(&self) -> usize {
        self.width
    }

    fn index(&self, row: usize, col: usize) -> usize {
        assert!(row < self.height && col < self.width, "cell out of range");
        row * self.width + col
    }

    /// セル `(row, col)` の現在の状態
    pub fn get(&self, row: usize, col: usize) -> CellState {
        self.cells[self.index(row, col)]
    }

    /// セル `(row, col)` を `state` にする
    /// （[`CellState::Unconfirmed`] への巻き戻しも可）
    pub fn set(&mut self, row: usize, col: usize, state: CellState) {
        let index = self.index(row, col);
        self.cells[index] = state;
    }

    /// 推論結果 `color` をセル `(row, col)` に書き込む
    ///
    /// 戻り値: `Ok(true)` 新たに確定 / `Ok(false)` 既に同色で変化なし /
    /// `Err(current)` 既に逆の色が入っていて矛盾
    pub(crate) fn paint(
        &mut self,
        row: usize,
        col: usize,
        color: Color,
    ) -> Result<bool, CellState> {
        let index = self.index(row, col);
        let current = self.cells[index];
        if current == CellState::Unconfirmed {
            self.cells[index] = color.into();
            Ok(true)
        } else if current == CellState::from(color) {
            Ok(false)
        } else {
            Err(current)
        }
    }

    /// `line` のセル状態を先頭から順に `buf` へ読み出す（`buf` は再利用のため
    /// 呼び出し側が持ち、この関数が clear して詰め直す）
    pub(crate) fn line_cells(&self, line: LineId, buf: &mut Vec<CellState>) {
        buf.clear();
        let len = match line {
            LineId::Row(_) => self.width,
            LineId::Col(_) => self.height,
        };
        for offset in 0..len {
            let (row, col) = line.cell(offset);
            buf.push(self.get(row, col));
        }
    }

    /// 全マスが確定している（未確定マスがない）かどうか
    pub fn is_complete(&self) -> bool {
        self.cells.iter().all(|&s| s != CellState::Unconfirmed)
    }

    /// `line` 上の黒マスの連続長の並び（先頭から順）
    pub fn runs(&self, line: LineId) -> Vec<usize> {
        let len = match line {
            LineId::Row(_) => self.width,
            LineId::Col(_) => self.height,
        };
        let mut runs = Vec::new();
        let mut run = 0;
        for offset in 0..len {
            let (row, col) = line.cell(offset);
            if self.get(row, col) == CellState::Black {
                run += 1;
            } else if run > 0 {
                runs.push(run);
                run = 0;
            }
        }
        if run > 0 {
            runs.push(run);
        }
        runs
    }

    /// この盤面が `clues` を満たしているかどうか
    ///
    /// 全マスが確定しており、かつ各行・各列の黒マスの連続長の並びが制約と
    /// 一致するときに `true`。未確定マスが残っていれば必ず `false`
    /// （黒の並びがたまたま揃っていても「解けた」とは見なさない）
    pub fn satisfies(&self, clues: &Clues) -> bool {
        self.height == clues.height()
            && self.width == clues.width()
            && self.is_complete()
            && clues
                .lines()
                .all(|line| self.runs(line) == clues.blocks(line))
    }
}

/// 盤面のテキスト表示。`.` 未確定 / `x` 白 / `o` 黒。5行・5列ごとに
/// 区切り線を挟む
impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in render_board(self.height, self.width, |r, c| match self.get(r, c) {
            CellState::Unconfirmed => '.',
            CellState::White => 'x',
            CellState::Black => 'o',
        }) {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

/// 1盤面をテキスト行のリストとして描画する。`cell(row, col)` はセルの
/// 表示文字（1文字幅）を返す。5行・5列ごとに区切り線／区切り文字を挟む
pub(crate) fn render_board(
    height: usize,
    width: usize,
    cell: impl Fn(usize, usize) -> char,
) -> Vec<String> {
    let separator = {
        let mut line = String::new();
        for col in 0..width {
            if col > 0 && col % 5 == 0 {
                line.push(' ');
            }
            line.push('-');
        }
        line
    };
    let mut lines = Vec::with_capacity(height + height / 5);
    for row in 0..height {
        if row > 0 && row % 5 == 0 {
            lines.push(separator.clone());
        }
        let mut line = String::new();
        for col in 0..width {
            if col > 0 && col % 5 == 0 {
                line.push('|');
            }
            line.push(cell(row, col));
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_detects_conflicts() {
        let mut grid = Grid::new(2, 3);
        assert_eq!(grid.paint(0, 1, Color::Black), Ok(true));
        assert_eq!(grid.paint(0, 1, Color::Black), Ok(false));
        assert_eq!(grid.paint(0, 1, Color::White), Err(CellState::Black));
        assert_eq!(grid.get(0, 1), CellState::Black);
    }

    #[test]
    fn line_cells_and_runs_respect_orientation() {
        // 2x3: 行と列で読み出しが転置されることを非正方形で確認する
        let grid = Grid::from_rows(vec![
            vec![CellState::Black, CellState::White, CellState::Black],
            vec![CellState::Black, CellState::Black, CellState::White],
        ])
        .unwrap();
        let mut buf = Vec::new();
        grid.line_cells(LineId::Row(1), &mut buf);
        assert_eq!(
            buf,
            vec![CellState::Black, CellState::Black, CellState::White]
        );
        grid.line_cells(LineId::Col(0), &mut buf);
        assert_eq!(buf, vec![CellState::Black, CellState::Black]);
        assert_eq!(grid.runs(LineId::Row(0)), vec![1, 1]);
        assert_eq!(grid.runs(LineId::Col(2)), vec![1]);
    }

    #[test]
    fn satisfies_requires_completeness() {
        let clues = Clues::new(vec![vec![], vec![]], vec![vec![], vec![]]).unwrap();
        let mut grid = Grid::new(2, 2);
        // 黒の並びは制約（すべて空）と一致するが、未確定マスが残る間は
        // 満たしたと見なさない
        assert!(!grid.satisfies(&clues));
        for r in 0..2 {
            for c in 0..2 {
                grid.set(r, c, CellState::White);
            }
        }
        assert!(grid.satisfies(&clues));
    }

    #[test]
    fn from_rows_rejects_ragged_rows() {
        let result = Grid::from_rows(vec![
            vec![CellState::Unconfirmed; 2],
            vec![CellState::Unconfirmed; 3],
        ]);
        assert_eq!(
            result,
            Err(GridMismatch::RaggedRows {
                row: 1,
                expected: 2,
                actual: 3
            })
        );
    }
}
