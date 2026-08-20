//! 総当たりによる健全性テスト。
//!
//! 実在の盤面から制約を生成してソルバに与える。盤面が実在する以上、
//! - `Err(Contradiction)` が返るのは確実にバグ
//! - `Ok(Solved)` が返るなら `judge()` を満たさなければならない
//! - `Ok(Stuck)` の場合も、確定済みの黒セルは元盤面と一致しなければならない
//!   （白は複数解の可能性があるため元盤面とは比較しない）
//!
//! 長方形（`height != width`）のケースも含める。正方形の盤面だけでは
//! 高さ・幅の取り違え（転置バグ）をテストが検出できない
//! （正方形は転置しても同じ形になるため）。
//!
//! シード・件数は旧実装（`illu-logi-solver`）の同名テストと同一で、
//! 同じ盤面集合に対して同じ保証が成り立つことを確認している。
use illu_logi_solver_super::*;

fn clues_of(grid: &[Vec<bool>]) -> Clues {
    let height = grid.len();
    let width = grid[0].len();
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
    let rows = (0..height)
        .map(|y| runs(&mut (0..width).map(|x| grid[y][x])))
        .collect();
    let cols = (0..width)
        .map(|x| runs(&mut (0..height).map(|y| grid[y][x])))
        .collect();
    Clues::new(rows, cols).expect("clues generated from a real grid are always valid")
}

fn check(grid: &[Vec<bool>]) {
    let height = grid.len();
    let width = grid[0].len();
    let mut solver = Solver::new(clues_of(grid));
    match solver.solve() {
        Ok(Outcome::Solved) => {
            assert!(solver.judge(), "judge failed for solvable grid: {grid:?}");
        }
        Ok(Outcome::Stuck) => {
            for i in 0..height {
                for j in 0..width {
                    if solver.state(i, j) == CellState::Black {
                        assert!(grid[i][j], "wrong Black at ({i},{j}) for grid: {grid:?}");
                    }
                }
            }
        }
        Err(e) => {
            panic!("unexpected solve() error on solvable grid: {grid:?}\n{e}");
        }
    }
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

fn random_grid(height: usize, width: usize, rng: &mut XorShift) -> Vec<Vec<bool>> {
    (0..height)
        .map(|_| {
            let bits = rng.next();
            (0..width).map(|x| bits >> x & 1 == 1).collect()
        })
        .collect()
}

#[test]
fn brute_force_4x4_exhaustive() {
    let n = 4;
    for bits in 0u64..(1 << (n * n)) {
        let grid: Vec<Vec<bool>> = (0..n)
            .map(|y| (0..n).map(|x| bits >> (y * n + x) & 1 == 1).collect())
            .collect();
        check(&grid);
    }
}

#[test]
fn brute_force_5x5_random() {
    let mut rng = XorShift(0x243F6A8885A308D3);
    for _ in 0..20_000 {
        check(&random_grid(5, 5, &mut rng));
    }
}

#[test]
fn brute_force_7x7_random() {
    let mut rng = XorShift(0x9E3779B97F4A7C15);
    for _ in 0..10_000 {
        check(&random_grid(7, 7, &mut rng));
    }
}

#[test]
fn brute_force_10x10_random() {
    let mut rng = XorShift(0xB5026F5AA96619E9);
    for _ in 0..2_000 {
        check(&random_grid(10, 10, &mut rng));
    }
}

// 長方形（height != width）盤面。i/j の取り違え（転置バグ）は正方形では
// 検出できないため、非正方形での回帰テストを必須で持つ。
#[test]
fn brute_force_5x8_random() {
    let mut rng = XorShift(0x1D4E228DAB7A0F63);
    for _ in 0..20_000 {
        check(&random_grid(5, 8, &mut rng));
    }
}

#[test]
fn brute_force_7x3_random() {
    let mut rng = XorShift(0x7A2D2B1E5C4F9861);
    for _ in 0..20_000 {
        check(&random_grid(7, 3, &mut rng));
    }
}
