//! 8つの行内推論規則。
//!
//! 各規則は「1本の行・列の分析結果 [`LineAnalysis`] を読み、そこから確定できる
//! 塗り [`Deduction`] を列挙する」だけの純関数で、盤面や行の状態を一切
//! 変更しない。規則の中身は総当たり検証済みの旧実装（`illu-logi-solver` の
//! `Line::STEPS`）の逐語移植である。
//!
//! [`RULES`] の並び順は「人間にとって気づきやすい・計算が安い」順で、
//! [`crate::Solver::hint`] はこの順に規則を試して最初に見つかった確定を返す。

use crate::analysis::{LineAnalysis, SetMinMax};
use crate::grid::{CellState, Color};
use std::ops::Range;

/// 確定の根拠。8つの行内推論規則のどれで確定できたか。
///
/// ペイロードは「塗った範囲そのもの」ではなく**根拠の区間・ID**である
/// （両者が偶然一致する variant もあるが、意味としては別物）。例えば
/// [`Reason::BlackIfLeftBounded`] の `(l, r)` は「非白領域の左端 `l` から、
/// 黒だと確定できる右端 `r` まで」という根拠区間であり、塗る範囲
/// （[`Deduction::range`]）はその部分集合になる。
///
/// ペイロードは「アンカー＋同一スナップショットの [`crate::Hint`] の行
/// コンテキストから、その推論の説明を一意に再構成できるか」を基準に絞って
/// ある。再構成できないものだけを昇格させ（[`Reason::BlackIfOverlap`] の
/// `id`、[`Reason::WhiteIfTooLong`] の `id`）、残る6 variant はアンカー
/// `(l, r)` だけで説明が一意に決まる。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// ブロック `id` を配置可能範囲の左端に寄せても右端に寄せても
    /// `[l, r)` は必ず黒になる（最左配置と最右配置の重複部分）。
    BlackIfOverlap {
        /// 重複区間の左端。
        l: usize,
        /// 重複区間の右端（半開区間）。
        r: usize,
        /// 根拠ブロックのID（0始まり）。同じ範囲を複数ブロックが説明し得る
        /// ため、アンカーだけでは一意に再構成できずペイロードに持つ。
        id: usize,
    },
    /// 非白領域 `[l, r)` は両端が白（または盤端）で区切られており、
    /// 中の全セルの候補ブロックの最小サイズが領域長以上なので全体が黒。
    BlackIfBounded(usize, usize),
    /// 黒セルを含む非白領域の左端 `l` が確定しているため、候補ブロックの
    /// 最小サイズぶん右の `r` まで黒が続く。
    BlackIfLeftBounded(usize, usize),
    /// 黒セルを含む非白領域の右端 `r` が確定しているため、候補ブロックの
    /// 最小サイズぶん左の `l` まで黒が続く。
    BlackIfRightBounded(usize, usize),
    /// 黒連続区間 `[l, r)` の長さが候補ブロックの最大サイズと一致し、
    /// これ以上は延びないので両隣は白。
    WhiteIfSegmentComplete(usize, usize),
    /// セル `j` を黒にすると、唯一の候補ブロックのサイズを超えてしまうので白。
    WhiteIfTooLong {
        /// 白だと確定できるセルの位置。
        j: usize,
        /// そのセルの唯一の候補ブロックのID（0始まり）。
        id: usize,
    },
    /// 両端が白（または盤端）で区切られた未確定領域 `[l, r)` が、そこを
    /// 覆い得るブロックの最小サイズより短いので全体が白。
    WhiteIfTooShort(usize, usize),
    /// どのブロックも `[l, r)` を覆えない（候補ブロックが空）ので白。
    WhiteIfNoBlockCovers(usize, usize),
}

/// 1件の確定: 行・列内の「どの範囲を・どちらの色に・なぜ」塗れるか。
///
/// `range`/`color` が「どこを・何色に塗るか」、`reason` が「なぜ塗れると
/// 分かったか」という役割分担。座標は行・列内のオフセット
/// （盤面座標への変換は [`crate::LineId::cell`]）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Deduction {
    /// 塗る範囲（行・列内のオフセットの半開区間）。
    pub range: Range<usize>,
    /// 塗る色。
    pub color: Color,
    /// 確定の根拠。
    pub reason: Reason,
}

/// 1つの推論規則: 分析結果から確定できる塗りを `out` に追加する純関数。
pub(crate) type Rule = fn(&LineAnalysis, &mut Vec<Deduction>);

/// 8規則を「安い・気づきやすい」順に並べた配列。
/// [`crate::Solver::hint`] はこの順で最初に見つかった確定を返す。
pub(crate) const RULES: [Rule; 8] = [
    black_if_overlap,
    black_if_bounded,
    black_if_left_bounded,
    black_if_right_bounded,
    white_if_segment_complete,
    white_if_too_long,
    white_if_too_short,
    white_if_no_block_covers,
];

/// 確定を `out` に積む。ただし新規性のないもの（空範囲、または範囲内の
/// 全セルが既に目標色）は捨てる。
///
/// このフィルタは「規則が何かを返した ＝ 新しい確定がある」という
/// [`crate::Solver::hint`] の前提そのものなので、規則は必ずこのヘルパを
/// 経由して確定を報告すること（直接 `out.push` すると、既に塗り終えた
/// 確定を毎回返し続けて hint が前へ進まなくなる）。
fn emit(
    analysis: &LineAnalysis,
    out: &mut Vec<Deduction>,
    range: Range<usize>,
    color: Color,
    reason: Reason,
) {
    if range.is_empty() {
        return;
    }
    if range
        .clone()
        .all(|j| analysis.cells[j] == CellState::from(color))
    {
        return;
    }
    out.push(Deduction {
        range,
        color,
        reason,
    });
}

// 最左配置と最右配置の重複部分は必ず黒。
fn black_if_overlap(a: &LineAnalysis, out: &mut Vec<Deduction>) {
    for (id, block) in a.blocks.iter().enumerate() {
        let l = block.possible_placement.start;
        let r = block.possible_placement.end;
        if r < l + block.size {
            continue;
        }
        let (l, r) = (r - block.size, l + block.size);
        emit(
            a,
            out,
            l..r,
            Color::Black,
            Reason::BlackIfOverlap { l, r, id },
        );
    }
}

// 両端が確定した非白領域で全セルの候補ブロックの最小サイズが領域長以上なら全体が黒。
fn black_if_bounded(a: &LineAnalysis, out: &mut Vec<Deduction>) {
    for &(l, r) in &a.non_white_segments {
        if (l..r).all(|j| a.cells[j] == CellState::Unconfirmed) {
            continue;
        }
        let size = r - l;
        if (l..r).all(|j| a.min_possible_size(j).is_none_or(|m| m >= size)) {
            emit(a, out, l..r, Color::Black, Reason::BlackIfBounded(l, r));
        }
    }
}

// 非白領域の左端が確定しているとき、最小ブロックサイズ分だけ右へ黒を延ばせる。
fn black_if_left_bounded(a: &LineAnalysis, out: &mut Vec<Deduction>) {
    for &(j, _) in &a.black_segments {
        let l = a.non_white_left(j);
        let r_max = a.non_white_right(j);
        let mut r = j;
        let mut min = (l..=r)
            .map(|j| a.min_possible_size(j).unwrap_or(0))
            .min()
            .unwrap_or(0);
        while r < r_max && {
            min.setmin(a.min_possible_size(r).unwrap_or(0));
            min
        } > r - l
        {
            r += 1;
        }
        emit(a, out, j..r, Color::Black, Reason::BlackIfLeftBounded(l, r));
    }
}

// 非白領域の右端が確定しているとき、最小ブロックサイズ分だけ左へ黒を延ばせる。
fn black_if_right_bounded(a: &LineAnalysis, out: &mut Vec<Deduction>) {
    for &(_, seg_r) in &a.black_segments {
        let j = seg_r - 1;
        let r = a.non_white_right(j);
        let l_min = a.non_white_left(j);
        let mut l = j;
        let mut min = (l..r)
            .map(|j| a.min_possible_size(j).unwrap_or(0))
            .min()
            .unwrap_or(0);
        while l > l_min && {
            min.setmin(a.min_possible_size(l).unwrap_or(0));
            min
        } > r - l
        {
            l -= 1;
        }
        emit(
            a,
            out,
            l..j,
            Color::Black,
            Reason::BlackIfRightBounded(l, r),
        );
    }
}

// 黒連続区間の長さが候補ブロックの最大サイズと一致すれば両隣は白。
fn white_if_segment_complete(a: &LineAnalysis, out: &mut Vec<Deduction>) {
    for &(l, r) in &a.black_segments {
        if (l == 0 || a.cells[l - 1] == CellState::White)
            && (r == a.n() || a.cells[r] == CellState::White)
        {
            continue;
        }
        let size = r - l;
        for j in l..r {
            if a.max_possible_size(j) == Some(size) {
                if l > 0 {
                    emit(
                        a,
                        out,
                        l - 1..l,
                        Color::White,
                        Reason::WhiteIfSegmentComplete(l, r),
                    );
                }
                if r < a.n() {
                    emit(
                        a,
                        out,
                        r..r + 1,
                        Color::White,
                        Reason::WhiteIfSegmentComplete(l, r),
                    );
                }
                break;
            }
        }
    }
}

// このセルを黒にすると唯一の候補ブロックのサイズを超えるなら白。
fn white_if_too_long(a: &LineAnalysis, out: &mut Vec<Deduction>) {
    for j in 0..a.n() {
        if !(a.cells[j] == CellState::Unconfirmed && a.candidates[j].len() == 1) {
            continue;
        }
        let id = a.candidates[j].start;
        let mut size = 1;
        if j > 0 && a.cells[j - 1] == CellState::Black {
            size += a.black_run_size(j - 1);
        }
        if j + 1 < a.n() && a.cells[j + 1] == CellState::Black {
            size += a.black_run_size(j + 1);
        }
        if size > a.blocks[id].size {
            emit(
                a,
                out,
                j..j + 1,
                Color::White,
                Reason::WhiteIfTooLong { j, id },
            );
        }
    }
}

// 両端が確定した未確定領域の最小ブロックサイズが領域長を超えるなら全体が白。
fn white_if_too_short(a: &LineAnalysis, out: &mut Vec<Deduction>) {
    for &(l, r) in &a.unconfirmed_segments {
        if l > 0 && a.cells[l - 1] != CellState::White {
            continue;
        }
        if r < a.n() && a.cells[r] != CellState::White {
            continue;
        }
        if (l..r).any(|j| a.min_possible_size(j).unwrap_or(0) > r - l) {
            emit(a, out, l..r, Color::White, Reason::WhiteIfTooShort(l, r));
        }
    }
}

// どのブロックにも属せないセルは白。
fn white_if_no_block_covers(a: &LineAnalysis, out: &mut Vec<Deduction>) {
    let mut l = 0;
    while l < a.n() {
        l = (l..a.n())
            .find(|&j| a.candidates[j].is_empty())
            .unwrap_or(a.n());
        let r = (l..a.n())
            .find(|&j| !a.candidates[j].is_empty())
            .unwrap_or(a.n());
        emit(
            a,
            out,
            l..r,
            Color::White,
            Reason::WhiteIfNoBlockCovers(l, r),
        );
        l = r;
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

    /// 旧実装の Line 単体テストを模すハーネス。セル状態と制約を持ち、
    /// `run` で「分析 → 指定規則を実行 → 確定をセルに適用」を1回分行う
    /// （旧実装の update_possible_id → 規則 → flush_queue に相当）。
    struct Fixture {
        blocks: Vec<usize>,
        cells: Vec<CellState>,
    }
    impl Fixture {
        fn new(blocks: Vec<usize>, pattern: &str) -> Self {
            Self {
                blocks,
                cells: cells_from(pattern),
            }
        }
        fn analysis(&self) -> LineAnalysis {
            LineAnalysis::compute(&self.blocks, &self.cells).unwrap()
        }
        fn run(&mut self, rules: &[Rule]) {
            let analysis = self.analysis();
            let mut deductions = Vec::new();
            for rule in rules {
                rule(&analysis, &mut deductions);
            }
            for d in deductions {
                for j in d.range {
                    assert_ne!(
                        self.cells[j],
                        match d.color {
                            Color::White => Black,
                            Color::Black => White,
                        },
                        "rule deduced a contradiction at {j}"
                    );
                    self.cells[j] = d.color.into();
                }
            }
        }
        fn assert_cells(&self, pattern: &str) {
            assert_eq!(self.cells, cells_from(pattern));
        }
        fn confirmed_id(&self, j: usize) -> Option<usize> {
            self.analysis().confirmed_id(j)
        }
    }

    #[test]
    fn test_black_if_overlap() {
        let mut f = Fixture::new(vec![4], ".....");
        f.run(&[black_if_overlap]);
        f.assert_cells(".ooo.");
        assert_eq!(f.confirmed_id(1), Some(0));
        assert_eq!(f.confirmed_id(2), Some(0));
        assert_eq!(f.confirmed_id(3), Some(0));

        let mut f = Fixture::new(vec![3, 1], ".....");
        f.run(&[black_if_overlap]);
        f.assert_cells("ooo.o");
        assert_eq!(f.confirmed_id(0), Some(0));
        assert_eq!(f.confirmed_id(1), Some(0));
        assert_eq!(f.confirmed_id(2), Some(0));
        assert_eq!(f.confirmed_id(4), Some(1));

        // ブロック0（サイズ1）はセル0にしか置けないので黒に確定する。
        let mut f = Fixture::new(vec![1, 2], ".xoo.");
        f.run(&[black_if_overlap]);
        f.assert_cells("oxoo.");

        let mut f = Fixture::new(vec![3, 2, 2], "..........");
        f.run(&[black_if_overlap]);
        f.assert_cells(".oo..o..o.");
        assert_eq!(f.confirmed_id(1), Some(0));
        assert_eq!(f.confirmed_id(2), Some(0));
        assert_eq!(f.confirmed_id(5), Some(1));
        assert_eq!(f.confirmed_id(8), Some(2));

        let mut f = Fixture::new(vec![1, 2, 5, 1, 1], "....xoo.oo....x..x..");
        for _ in 0..2 {
            f.run(&[black_if_overlap]);
            f.assert_cells("....xoo.oo....x..x..");
            assert_eq!(f.confirmed_id(5), None);
            assert_eq!(f.confirmed_id(6), None);
            assert_eq!(f.confirmed_id(8), Some(2));
            assert_eq!(f.confirmed_id(9), Some(2));
        }
    }

    #[test]
    fn test_white_if_segment_complete() {
        let mut f = Fixture::new(vec![2, 2], "....oo....");
        f.run(&[white_if_segment_complete]);
        f.assert_cells("...xoox...");

        // 左右が既に白で確定済みのセグメントは正しくスキップされる。
        let mut f = Fixture::new(vec![2, 2], "xoox.....");
        f.run(&[white_if_segment_complete]);
        f.assert_cells("xoox.....");
    }

    #[test]
    fn test_white_if_no_block_covers() {
        let mut f = Fixture::new(vec![2, 2], ".o......o.");
        f.run(&[black_if_overlap, white_if_no_block_covers]);
        f.assert_cells(".o.xxxx.o.");
        assert_eq!(f.confirmed_id(1), Some(0));
        assert_eq!(f.confirmed_id(8), Some(1));
    }

    #[test]
    fn test_black_if_left_bounded() {
        let mut f = Fixture::new(vec![2, 2], "....xo....");
        f.run(&[black_if_left_bounded]);
        f.assert_cells("....xoo...");
        assert_eq!(f.confirmed_id(5), None);
        assert_eq!(f.confirmed_id(6), None);

        let mut f = Fixture::new(vec![2, 6, 5, 2, 1], "oox......x..o.................");
        f.run(&[black_if_overlap, black_if_left_bounded]);
        f.assert_cells("oox......x..ooo...............");
        assert_eq!(f.confirmed_id(0), Some(0));
        assert_eq!(f.confirmed_id(1), Some(0));
        assert_eq!(f.confirmed_id(12), None);
        assert_eq!(f.confirmed_id(13), None);
        assert_eq!(f.confirmed_id(14), None);

        let mut f = Fixture::new(vec![1, 2, 5, 1, 1], "....xoo.oo....x..x..");
        f.run(&[black_if_overlap, black_if_overlap, black_if_left_bounded]);
        f.assert_cells("....xoo.oo....x..x..");
        assert_eq!(f.confirmed_id(5), None);
        assert_eq!(f.confirmed_id(6), None);
        assert_eq!(f.confirmed_id(8), Some(2));
        assert_eq!(f.confirmed_id(9), Some(2));
    }

    #[test]
    fn test_black_if_right_bounded() {
        let mut f = Fixture::new(vec![2, 2], "....ox....");
        f.run(&[black_if_right_bounded]);
        f.assert_cells("...oox....");
        assert_eq!(f.confirmed_id(3), None);
        assert_eq!(f.confirmed_id(4), None);

        let mut f = Fixture::new(vec![2, 2], ".ox.......");
        f.run(&[black_if_right_bounded]);
        f.assert_cells("oox.......");
        assert_eq!(f.confirmed_id(0), Some(0));
        assert_eq!(f.confirmed_id(1), Some(0));
    }

    #[test]
    fn test_black_if_bounded() {
        let mut f = Fixture::new(vec![3, 3], "...x.o.x...");
        f.run(&[black_if_bounded]);
        f.assert_cells("...xooox...");
        assert_eq!(f.confirmed_id(4), None);
        assert_eq!(f.confirmed_id(5), None);
        assert_eq!(f.confirmed_id(6), None);
    }

    #[test]
    fn test_white_if_too_long() {
        let mut f = Fixture::new(vec![1, 2], "..o.......");
        f.run(&[white_if_too_long]);
        f.assert_cells(".xo.......");
        assert_eq!(f.confirmed_id(2), None);
    }

    #[test]
    fn test_white_if_too_short() {
        let mut f = Fixture::new(vec![1, 2, 2], ".....ox.x......");
        f.run(&[white_if_too_short]);
        f.assert_cells(".....oxxx......");
        assert_eq!(f.confirmed_id(5), None);
    }

    // emit の新規性フィルタ: 既に塗り終えた確定は報告されない。
    // これが破れると hint()/next_step() が同じ確定を返し続けて前へ進まなくなる。
    #[test]
    fn rules_do_not_reemit_applied_deductions() {
        let mut f = Fixture::new(vec![4], ".....");
        f.run(&RULES);
        f.assert_cells(".ooo.");
        // 2周目: 盤面はもう変わらないので、新規の確定は出ない。
        let analysis = f.analysis();
        let mut deductions = Vec::new();
        for rule in RULES {
            rule(&analysis, &mut deductions);
        }
        assert_eq!(deductions, vec![]);
    }
}
