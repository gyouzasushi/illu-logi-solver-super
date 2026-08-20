//! 1本の行・列の純関数的な分析。
//!
//! [`LineAnalysis::compute`] は制約（ブロックサイズ列）と現在のセル状態だけを
//! 入力に取り、各ブロックの配置可能範囲と各セルの候補ブロックID範囲を
//! 不動点まで狭める。

use crate::grid::CellState;
use std::ops::Range;

/// `self` を `v` との min / max で更新し、変化したかを返す。
pub(crate) trait SetMinMax {
    fn setmin(&mut self, v: Self) -> bool;
    fn setmax(&mut self, v: Self) -> bool;
}
impl<T> SetMinMax for T
where
    T: PartialOrd,
{
    fn setmin(&mut self, v: T) -> bool {
        *self > v && {
            *self = v;
            true
        }
    }
    fn setmax(&mut self, v: T) -> bool {
        *self < v && {
            *self = v;
            true
        }
    }
}

/// `pred` を満たすセルの極大連続区間（半開区間）を左から順に列挙する。
pub(crate) fn segments_of(
    cells: &[CellState],
    pred: impl Fn(CellState) -> bool,
) -> Vec<(usize, usize)> {
    let mut segments = Vec::new();
    let mut l = 0;
    while l < cells.len() {
        if !pred(cells[l]) {
            l += 1;
            continue;
        }
        let r = (l..cells.len())
            .find(|&j| !pred(cells[j]))
            .unwrap_or(cells.len());
        segments.push((l, r));
        l = r;
    }
    segments
}

/// 1ブロックの分析結果: 制約に書かれたサイズと、配置され得る範囲。
#[derive(Clone, Debug)]
pub(crate) struct BlockInfo {
    /// ブロックの長さ（制約に書かれた値そのもの）。
    pub(crate) size: usize,
    /// ブロック全体（`size` マス分）が収まり得る範囲（半開区間）。
    /// 先頭マスの取り得る位置は `start..=end - size`。
    pub(crate) possible_placement: Range<usize>,
}

/// あるスナップショット時点での1本の行・列の分析結果。
///
/// [`LineAnalysis::compute`] でのみ作られる不変値。8つの推論規則
/// （[`crate::rules`]）はこれを読むだけの純関数として実装される。
#[derive(Clone, Debug)]
pub(crate) struct LineAnalysis {
    /// 分析に使ったセル状態のスナップショット。
    pub(crate) cells: Vec<CellState>,
    /// ブロックごとの分析結果（制約と同じ並び）。
    pub(crate) blocks: Vec<BlockInfo>,
    /// セルごとの候補ブロックID範囲。このセルが黒だとしたら、どのブロックの
    /// 一部であり得るか（半開区間）。空なら「どのブロックにも属せない＝白」。
    pub(crate) candidates: Vec<Range<usize>>,
    /// 黒マスの極大連続区間。
    pub(crate) black_segments: Vec<(usize, usize)>,
    /// 非白（黒または未確定）マスの極大連続区間。
    pub(crate) non_white_segments: Vec<(usize, usize)>,
    /// 未確定マスの極大連続区間。
    pub(crate) unconfirmed_segments: Vec<(usize, usize)>,
}

impl LineAnalysis {
    /// 制約 `block_sizes` と現在のセル状態 `cells` から分析を計算する。
    ///
    /// ブロックの配置可能範囲とセルの候補ブロックID範囲を、互いに狭め合いが
    /// 止まる（不動点）まで反復する。あるブロックがどこにも収まらなくなったら
    /// そのブロックの番号を `Err` で返す（行レベルの矛盾）。
    ///
    /// 各ループは `id` を `blocks[id]` と `min_starts[id]`/`max_ends[id]` の
    /// 両方の添字に使っており、`enumerate()` 化すると可読性が落ちるため
    /// `needless_range_loop` を許容する。
    #[allow(clippy::needless_range_loop)]
    pub(crate) fn compute(block_sizes: &[usize], cells: &[CellState]) -> Result<Self, usize> {
        let n = cells.len();
        let num_blocks = block_sizes.len();
        let mut candidates = vec![0..num_blocks; n];
        let mut blocks: Vec<BlockInfo> = block_sizes
            .iter()
            .map(|&size| BlockInfo {
                size,
                possible_placement: 0..n,
            })
            .collect();
        let black_segments = segments_of(cells, |s| s == CellState::Black);

        loop {
            let mut changed = false;

            /* 左に寄せる */
            let mut min_starts = vec![0; num_blocks];
            let mut j = n;
            for id in (0..num_blocks).rev() {
                j = (1..=j)
                    .rfind(|&j| cells[j - 1] == CellState::Black && candidates[j - 1].end <= id + 1)
                    .unwrap_or(0);
                if j > blocks[id].size {
                    min_starts[id].setmax(j - blocks[id].size);
                }
            }
            let mut j = n;
            for id in (0..num_blocks).rev() {
                j = (1..=j).rfind(|&j| candidates[j - 1].end <= id).unwrap_or(0);
                min_starts[id].setmax(j);
            }
            let mut l = 0;
            for id in 0..num_blocks {
                l.setmax(min_starts[id]);
                let mut r = l + blocks[id].size;
                if r <= n {
                    while let Some(j) = (l..r).rfind(|&j| cells[j] == CellState::White) {
                        l = j + 1;
                        r = l + blocks[id].size;
                        if r > n {
                            break;
                        }
                    }
                }
                blocks[id].possible_placement.start = l;
                for j in 0..l.min(n) {
                    changed |= candidates[j].end.setmin(id);
                }
                l = r + 1;
            }

            /* 右に寄せる */
            let mut max_ends = vec![n; num_blocks];
            let mut j = 0;
            for id in 0..num_blocks {
                j = (j..n)
                    .find(|&j| cells[j] == CellState::Black && candidates[j].start >= id)
                    .unwrap_or(n);
                if j + blocks[id].size <= n {
                    max_ends[id].setmin(j + blocks[id].size);
                }
            }
            let mut j = 0;
            for id in 0..num_blocks {
                j = (j..n).find(|&j| candidates[j].start > id).unwrap_or(n);
                max_ends[id].setmin(j);
            }
            let mut r = n;
            for id in (0..num_blocks).rev() {
                r.setmin(max_ends[id]);
                let mut l = r.wrapping_sub(blocks[id].size);
                if l < n {
                    while let Some(j) = (l..r).find(|&j| cells[j] == CellState::White) {
                        r = j;
                        l = r.wrapping_sub(blocks[id].size);
                        if l >= n {
                            break;
                        }
                    }
                }
                blocks[id].possible_placement.end = r;
                for j in r..n {
                    changed |= candidates[j].start.setmax(id + 1);
                }
                r = l.wrapping_sub(1);
            }

            /* 黒セグメント内の候補をそろえる: 連続する黒マスは同じブロックに
             * 属するので、セグメント内の候補範囲は共通部分に狭められる */
            for &(l, r) in &black_segments {
                let mut lo = 0;
                let mut hi = num_blocks;
                for j in l..r {
                    lo.setmax(candidates[j].start);
                    hi.setmin(candidates[j].end);
                }
                for j in l..r {
                    changed |= candidates[j].start.setmax(lo);
                    changed |= candidates[j].end.setmin(hi);
                }
            }

            if !changed {
                break;
            }
        }

        for (id, block) in blocks.iter().enumerate() {
            if block.possible_placement.start + block.size > block.possible_placement.end {
                // ブロック `id` がどこにも収まらない（行レベルの矛盾）。
                return Err(id);
            }
        }

        Ok(Self {
            non_white_segments: segments_of(cells, |s| s != CellState::White),
            unconfirmed_segments: segments_of(cells, |s| s == CellState::Unconfirmed),
            cells: cells.to_vec(),
            blocks,
            candidates,
            black_segments,
        })
    }

    /// この行・列のマス数。
    pub(crate) fn n(&self) -> usize {
        self.cells.len()
    }

    /// セル `j` の候補ブロックのうち最小のサイズ。候補が空なら `None`。
    /// ブロック数は小さいので素朴に走査する。
    pub(crate) fn min_possible_size(&self, j: usize) -> Option<usize> {
        self.candidates[j]
            .clone()
            .map(|id| self.blocks[id].size)
            .min()
    }

    /// セル `j` の候補ブロックのうち最大のサイズ。候補が空なら `None`。
    pub(crate) fn max_possible_size(&self, j: usize) -> Option<usize> {
        self.candidates[j]
            .clone()
            .map(|id| self.blocks[id].size)
            .max()
    }

    /// セル `j` の候補ブロックがただ1つに確定していればその ID。
    pub(crate) fn confirmed_id(&self, j: usize) -> Option<usize> {
        (self.candidates[j].len() == 1).then(|| self.candidates[j].start)
    }

    /// セル `j` を含む非白領域の左端。
    pub(crate) fn non_white_left(&self, j: usize) -> usize {
        let mut l = j;
        while l > 0 && self.cells[l - 1] != CellState::White {
            l -= 1;
        }
        l
    }

    /// セル `j` を含む非白領域の右端（半開区間の終端）。
    pub(crate) fn non_white_right(&self, j: usize) -> usize {
        let mut r = j;
        while r < self.n() && self.cells[r] != CellState::White {
            r += 1;
        }
        r
    }

    /// 黒マス `j` を含む黒連続区間の長さ。
    pub(crate) fn black_run_size(&self, j: usize) -> usize {
        let mut l = j;
        while l > 0 && self.cells[l - 1] == CellState::Black {
            l -= 1;
        }
        let mut r = j;
        while r < self.n() && self.cells[r] == CellState::Black {
            r += 1;
        }
        r - l
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use CellState::{Black, Unconfirmed, White};

    fn cells_from(pattern: &str) -> Vec<CellState> {
        pattern
            .chars()
            .filter(|&c| c != '|')
            .map(|c| match c {
                '.' => Unconfirmed,
                'x' => White,
                'o' => Black,
                _ => panic!("unknown cell char {c:?}"),
            })
            .collect()
    }

    #[test]
    fn test_segments_of() {
        let cells = cells_from("oo.xo");
        assert_eq!(segments_of(&cells, |s| s == Black), vec![(0, 2), (4, 5)]);
        assert_eq!(segments_of(&cells, |s| s != White), vec![(0, 3), (4, 5)]);
        assert_eq!(segments_of(&cells, |s| s == Unconfirmed), vec![(2, 3)]);
        assert_eq!(segments_of(&[], |_| true), vec![]);
    }

    #[test]
    fn test_confirmed_ids() {
        let cells = cells_from("xxooo|ooxxx|...xo|xoxxo");
        let analysis = LineAnalysis::compute(&[5, 1, 1, 1], &cells).unwrap();
        for j in 0..20 {
            match j {
                2..=6 => assert_eq!(analysis.confirmed_id(j), Some(0)),
                14 => assert_eq!(analysis.confirmed_id(j), Some(1)),
                16 => assert_eq!(analysis.confirmed_id(j), Some(2)),
                19 => assert_eq!(analysis.confirmed_id(j), Some(3)),
                _ => assert_eq!(analysis.confirmed_id(j), None, "j={j}"),
            }
        }
    }

    #[test]
    fn test_no_placement() {
        // 幅2に長さ3のブロックは収まらない…は Clues 検証で弾かれる形なので、
        // 白確定によって収まらなくなるケースを見る。
        let cells = cells_from("x.x");
        assert!(matches!(LineAnalysis::compute(&[2], &cells), Err(0)));
    }
}
