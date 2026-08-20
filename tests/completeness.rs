//! 規則セットの完全性の回帰テスト。
//!
//! 8規則の行推論を「完全な行推論（全配置列挙の不動点）」と比較し、
//! 完全行推論なら解けるのに `Stuck` になる盤面（gap）の数が悪化していない
//! ことを確認する。実測: 4x4全列挙 solver_ok=51234 gap=0、
//! 5x5ランダム2万件 solver_ok=14846 gap=0、7x7ランダム1万件
//! solver_ok=6618 gap=2。
//! gap の正体はセル毎の候補を `Range` で持つ表現が候補間の相関を表せない
//! ことによる原理的限界で、規則の追加漏れではない。規則や候補管理を変更
//! した際にここが増えたら推論力が退行している。
//!
//! 各テストの `max_gap` はここに固定した `XorShift` シード列挙で実測した
//! 経験値であり、シードや列挙順を変えると gap の実現値も変わり得る。
//! シードを変更する場合は許容上限も実測し直すこと。
use illu_logi_solver_super::*;

// 現在の states と制約に整合する全配置を列挙し、
// (黒になり得るマスク, 白になり得るマスク) を返す。整合配置ゼロなら None。
fn line_masks(states: &[CellState], blocks: &[usize]) -> Option<(u64, u64)> {
    let mut can_black = 0u64;
    let mut can_white = 0u64;
    let mut found = false;
    // rec(i, k, mask): セル i 以降にブロック k 以降を配置
    #[allow(clippy::too_many_arguments)]
    fn rec(
        i: usize,
        k: usize,
        mask: u64,
        states: &[CellState],
        blocks: &[usize],
        can_black: &mut u64,
        can_white: &mut u64,
        found: &mut bool,
    ) {
        let n = states.len();
        if k == blocks.len() {
            if (i..n).any(|j| states[j] == CellState::Black) {
                return;
            }
            *found = true;
            *can_black |= mask;
            *can_white |= !mask & ((1u64 << n) - 1);
            return;
        }
        let b = blocks[k];
        let mut start = i;
        loop {
            if start + b > n {
                return;
            }
            // start..start+b を黒にできるか
            let ok_black = (start..start + b).all(|j| states[j] != CellState::White);
            // 直後のセルは白にできるか
            let ok_gap = start + b == n || states[start + b] != CellState::Black;
            if ok_black && ok_gap {
                let next = if start + b == n { n } else { start + b + 1 };
                let mut m = mask;
                for j in start..start + b {
                    m |= 1 << j;
                }
                rec(next, k + 1, m, states, blocks, can_black, can_white, found);
            }
            // start を白にして次へ（start が黒確定ならここで打ち切り）
            if states[start] == CellState::Black {
                return;
            }
            start += 1;
        }
    }
    rec(
        0,
        0,
        0,
        states,
        blocks,
        &mut can_black,
        &mut can_white,
        &mut found,
    );
    found.then_some((can_black, can_white))
}

// 完全な行推論の不動点。全確定なら Some(true)、途中で止まれば Some(false)、矛盾は None。
// axis/j は grid[i][j] と grid[j][i] を切り替えて使うため、enumerate() 化は適さない。
#[allow(clippy::needless_range_loop)]
fn dp_fixpoint(rows: &[Vec<usize>], cols: &[Vec<usize>]) -> Option<bool> {
    let n = rows.len();
    let mut grid = vec![vec![CellState::Unconfirmed; n]; n];
    loop {
        let mut changed = false;
        for axis in 0..2 {
            for i in 0..n {
                let states: Vec<CellState> = (0..n)
                    .map(|j| if axis == 0 { grid[i][j] } else { grid[j][i] })
                    .collect();
                let blocks = if axis == 0 { &rows[i] } else { &cols[i] };
                let (cb, cw) = line_masks(&states, blocks)?;
                for j in 0..n {
                    let b = cb >> j & 1 == 1;
                    let w = cw >> j & 1 == 1;
                    let new = match (b, w) {
                        (true, false) => CellState::Black,
                        (false, true) => CellState::White,
                        (true, true) => continue,
                        (false, false) => return None,
                    };
                    let cell = if axis == 0 {
                        &mut grid[i][j]
                    } else {
                        &mut grid[j][i]
                    };
                    if *cell != new {
                        *cell = new;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            let solved = grid
                .iter()
                .all(|row| row.iter().all(|&s| s != CellState::Unconfirmed));
            return Some(solved);
        }
    }
}

fn clue_lists_of(grid: &[Vec<bool>]) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let n = grid.len();
    let runs = |cells: &mut dyn Iterator<Item = bool>| -> Vec<usize> {
        let mut v = Vec::new();
        let mut run = 0;
        for cell in cells {
            if cell {
                run += 1;
            } else if run > 0 {
                v.push(run);
                run = 0;
            }
        }
        if run > 0 {
            v.push(run);
        }
        v
    };
    (
        (0..n)
            .map(|y| runs(&mut (0..n).map(|x| grid[y][x])))
            .collect(),
        (0..n)
            .map(|x| runs(&mut (0..n).map(|y| grid[y][x])))
            .collect(),
    )
}

struct XorShift(u64);
impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn run_experiment(grids: impl Iterator<Item = Vec<Vec<bool>>>, label: &str, max_gap: u64) {
    let mut total = 0u64;
    let mut solver_ok = 0u64;
    let mut both_stuck = 0u64; // 完全行推論でも未確定 → 規則セットのせいではない
    let mut gap = 0u64; // 完全行推論なら解けるのに Stuck → 規則の不足
    for grid in grids {
        total += 1;
        let (rows, cols) = clue_lists_of(&grid);
        let mut solver = Solver::new(Clues::new(rows.clone(), cols.clone()).unwrap());
        match solver.solve() {
            Ok(Outcome::Solved) => solver_ok += 1,
            Ok(Outcome::Stuck) => match dp_fixpoint(&rows, &cols) {
                Some(true) => gap += 1,
                Some(false) => both_stuck += 1,
                None => panic!("dp contradiction on solvable grid: {grid:?}"),
            },
            Err(e) => panic!("contradiction on solvable grid: {grid:?}\n{e}"),
        }
    }
    println!("{label}: total={total} solver_ok={solver_ok} both_stuck={both_stuck} gap={gap}");
    assert!(
        gap <= max_gap,
        "{label}: gap={gap} > {max_gap}: 完全行推論なら解ける盤面の取りこぼしが増えた（推論力の退行）"
    );
}

#[test]
fn completeness_4x4_exhaustive() {
    let n = 4;
    run_experiment(
        (0u64..1 << (n * n)).map(|bits| {
            (0..n)
                .map(|y| (0..n).map(|x| bits >> (y * n + x) & 1 == 1).collect())
                .collect()
        }),
        "4x4 exhaustive",
        0,
    );
}

#[test]
fn completeness_5x5_random() {
    let mut rng = XorShift(0x243F6A8885A308D3);
    run_experiment(
        (0..20_000).map(move |_| {
            (0..5)
                .map(|_| {
                    let bits = rng.next();
                    (0..5).map(|x| bits >> x & 1 == 1).collect()
                })
                .collect()
        }),
        "5x5 random",
        0,
    );
}

#[test]
fn completeness_7x7_random() {
    let mut rng = XorShift(0x9E3779B97F4A7C15);
    run_experiment(
        (0..10_000).map(move |_| {
            (0..7)
                .map(|_| {
                    let bits = rng.next();
                    (0..7).map(|x| bits >> x & 1 == 1).collect()
                })
                .collect()
        }),
        "7x7 random",
        2,
    );
}

// gap の中身を目視したいとき用:
// cargo test --test completeness show_gap_cases -- --ignored --nocapture
#[test]
#[ignore]
fn show_gap_cases() {
    let mut rng = XorShift(0x9E3779B97F4A7C15);
    for iter in 0..10_000 {
        let grid: Vec<Vec<bool>> = (0..7)
            .map(|_| {
                let bits = rng.next();
                (0..7).map(|x| bits >> x & 1 == 1).collect()
            })
            .collect();
        let (rows, cols) = clue_lists_of(&grid);
        let mut solver = Solver::new(Clues::new(rows.clone(), cols.clone()).unwrap());
        if solver.solve() == Ok(Outcome::Stuck) && dp_fixpoint(&rows, &cols) == Some(true) {
            println!("=== gap case iter {iter} ===");
            println!("rows: {rows:?}");
            println!("cols: {cols:?}");
            println!("solver stuck at:\n{}", solver.debug_display());
        }
    }
}
