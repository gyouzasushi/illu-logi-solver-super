//! 制約（各行・各列の黒マスの連続長の並び）と、行・列の識別子
//!
//! パズル入力は [`Clues::new`] で検証済みの [`Clues`] に変換してから使う。
//! 入力ミスがエラーになるのはこの検証だけ

use std::fmt;
use thiserror::Error;

/// 盤面上の1本の行・列を指す識別子
///
/// `Row(i)` は上から `i` 番目の行、`Col(j)` は左から `j` 番目の列（いずれも
/// 0始まり）。行・列内の位置（オフセット）と盤面座標 `(row, col)` の変換は
/// [`LineId::cell`] と [`LineId::orthogonal_at`] で行う
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum LineId {
    /// 行（上から0始まり）
    Row(usize),
    /// 列（左から0始まり）
    Col(usize),
}

impl LineId {
    /// この行・列の `offset` 番目のセルの盤面座標 `(row, col)`
    ///
    /// ```
    /// use illu_logi_solver_super::LineId;
    ///
    /// assert_eq!(LineId::Row(2).cell(5), (2, 5));
    /// assert_eq!(LineId::Col(2).cell(5), (5, 2));
    /// ```
    pub fn cell(self, offset: usize) -> (usize, usize) {
        match self {
            LineId::Row(i) => (i, offset),
            LineId::Col(j) => (offset, j),
        }
    }

    /// `offset` 番目のセルで交差する、直交する行・列
    ///
    /// ```
    /// use illu_logi_solver_super::LineId;
    ///
    /// assert_eq!(LineId::Row(2).orthogonal_at(5), LineId::Col(5));
    /// assert_eq!(LineId::Col(2).orthogonal_at(5), LineId::Row(5));
    /// ```
    pub fn orthogonal_at(self, offset: usize) -> LineId {
        match self {
            LineId::Row(_) => LineId::Col(offset),
            LineId::Col(_) => LineId::Row(offset),
        }
    }
}

impl fmt::Display for LineId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineId::Row(i) => write!(f, "row {i}"),
            LineId::Col(j) => write!(f, "col {j}"),
        }
    }
}

/// [`Clues::new`] の入力検証エラー
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ClueError {
    /// `line` の制約にサイズ0のブロックが含まれていた
    #[error("{line} contains a block of size 0, which is not a valid clue")]
    ZeroBlock {
        /// 不正な制約を持つ行・列
        line: LineId,
    },
    /// `line` の制約が線の長さ `len` に収まらない
    /// （`sum(blocks) + (blocks.len() - 1) > len`）
    #[error(
        "{line} cannot fit in a line of length {len}: blocks require at least sum(blocks) + (blocks.len() - 1) cells"
    )]
    TooLong {
        /// 収まらない制約を持つ行・列
        line: LineId,
        /// その行・列の実際の長さ
        len: usize,
    },
}

/// 検証済みのパズル制約
///
/// 高さ（行の本数）と幅（列の本数）は独立で、長方形の盤面も扱える。
/// 構築できた時点で、全ブロックのサイズが1以上で、どの行・列の制約も
/// 線長に収まることが保証されている
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Clues {
    rows: Vec<Vec<usize>>,
    cols: Vec<Vec<usize>>,
}

impl Clues {
    /// 行の制約（上から順）と列の制約（左から順）から検証して構築する
    ///
    /// 各要素はその行・列の「黒マスの連続長」を左（上）から並べたもので、
    /// 空の `Vec` はその行・列が全マス白であることを表す
    ///
    /// ```
    /// use illu_logi_solver_super::Clues;
    ///
    /// // 2行3列の長方形パズル（行の制約2本、列の制約3本）
    /// let clues = Clues::new(
    ///     vec![vec![3], vec![]],
    ///     vec![vec![1], vec![1], vec![1]],
    /// )
    /// .unwrap();
    /// assert_eq!((clues.height(), clues.width()), (2, 3));
    /// ```
    pub fn new(rows: Vec<Vec<usize>>, cols: Vec<Vec<usize>>) -> Result<Self, ClueError> {
        let clues = Self { rows, cols };
        for line in clues.lines() {
            let blocks = clues.blocks(line);
            if blocks.contains(&0) {
                return Err(ClueError::ZeroBlock { line });
            }
            let min_len = blocks.iter().sum::<usize>() + blocks.len().saturating_sub(1);
            let len = clues.line_len(line);
            if min_len > len {
                return Err(ClueError::TooLong { line, len });
            }
        }
        Ok(clues)
    }

    /// 盤面の高さ（行の本数）
    pub fn height(&self) -> usize {
        self.rows.len()
    }

    /// 盤面の幅（列の本数）
    pub fn width(&self) -> usize {
        self.cols.len()
    }

    /// `line` の制約（黒マスの連続長の並び）
    pub fn blocks(&self, line: LineId) -> &[usize] {
        match line {
            LineId::Row(i) => &self.rows[i],
            LineId::Col(j) => &self.cols[j],
        }
    }

    /// `line` のマス数（行なら幅、列なら高さ）
    pub fn line_len(&self, line: LineId) -> usize {
        match line {
            LineId::Row(_) => self.width(),
            LineId::Col(_) => self.height(),
        }
    }

    /// 全行・全列を `Row(0..height)`、`Col(0..width)` の順に列挙する
    pub fn lines(&self) -> impl Iterator<Item = LineId> + use<> {
        let (height, width) = (self.height(), self.width());
        (0..height)
            .map(LineId::Row)
            .chain((0..width).map(LineId::Col))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_id_is_the_single_transposition_point() {
        assert_eq!(LineId::Row(3).cell(7), (3, 7));
        assert_eq!(LineId::Col(3).cell(7), (7, 3));
        assert_eq!(LineId::Row(3).orthogonal_at(7), LineId::Col(7));
        assert_eq!(LineId::Col(3).orthogonal_at(7), LineId::Row(7));
        // cell と orthogonal_at の整合: 直交する行・列は同じセルで交差する
        let line = LineId::Row(3);
        let (r, c) = line.cell(7);
        assert_eq!(line.orthogonal_at(7).cell(3), (r, c));
    }

    #[test]
    fn validation() {
        // ブロックサイズ 0 は拒否される
        assert_eq!(
            Clues::new(vec![vec![0]], vec![vec![1]]),
            Err(ClueError::ZeroBlock {
                line: LineId::Row(0)
            })
        );
        // sum(blocks) + (blocks.len() - 1) > 線長 は拒否される
        // (3 + 1 + 1 = 5 > 幅3)
        assert_eq!(
            Clues::new(
                vec![vec![3, 1], vec![], vec![]],
                vec![vec![], vec![], vec![]],
            ),
            Err(ClueError::TooLong {
                line: LineId::Row(0),
                len: 3
            })
        );
        // ぴったり収まる場合は拒否されない (3 + 1 + 1 = 5 <= 幅5)
        assert!(Clues::new(vec![vec![3, 1]; 5], vec![vec![]; 5]).is_ok());
        // 行と列の本数が食い違っても長方形として受け入れられる
        assert!(Clues::new(vec![vec![1]], vec![vec![1]; 2]).is_ok());
        // 列側の検証も行われる（列の線長は高さ）
        assert_eq!(
            Clues::new(vec![vec![]], vec![vec![2]]),
            Err(ClueError::TooLong {
                line: LineId::Col(0),
                len: 1
            })
        );
    }

    #[test]
    fn lines_enumerates_rows_then_cols() {
        let clues = Clues::new(vec![vec![]; 2], vec![vec![]; 3]).unwrap();
        assert_eq!(
            clues.lines().collect::<Vec<_>>(),
            vec![
                LineId::Row(0),
                LineId::Row(1),
                LineId::Col(0),
                LineId::Col(1),
                LineId::Col(2),
            ]
        );
    }
}
