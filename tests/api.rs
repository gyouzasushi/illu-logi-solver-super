//! 公開APIの仕様テスト
//!
//! - 実在パズル（10x10〜30x30）のフルソルブ
//! - 矛盾・入力検証のエラー形状（座標が盤面座標であること）
//! - `hint()` と `next_step()` の同一性、`solve()` と逐次実行の閉包一致
//! - `Session` の仕様
use illu_logi_solver_super::*;
use std::time::Instant;

fn clues_for_10x10() -> Clues {
    Clues::new(
        vec![
            vec![5, 1],
            vec![2, 3],
            vec![2, 2, 1],
            vec![3, 2, 2],
            vec![1, 3, 1],
            vec![2, 3],
            vec![1, 3, 1],
            vec![1, 1, 2, 2],
            vec![1, 6, 1],
            vec![5, 2],
        ],
        vec![
            vec![1, 2, 2],
            vec![7],
            vec![2, 1, 1, 2],
            vec![1, 1, 4],
            vec![2, 1, 1, 2],
            vec![9],
            vec![3, 1, 3],
            vec![3, 1],
            vec![1, 1, 1, 1],
            vec![2, 3],
        ],
    )
    .unwrap()
}

fn clues_for_15x15() -> Clues {
    Clues::new(
        vec![
            vec![2, 4, 5],
            vec![4, 1, 1],
            vec![3, 3, 1],
            vec![8, 1],
            vec![1, 3, 1, 5],
            vec![2, 2, 4],
            vec![1, 1, 1, 2, 1, 2],
            vec![1, 7, 2],
            vec![1, 1, 3],
            vec![5, 1, 2, 1, 1],
            vec![3, 4, 1],
            vec![1, 1, 1, 1, 1],
            vec![2, 6],
            vec![10, 1],
            vec![7, 2],
        ],
        vec![
            vec![1, 3, 2, 1],
            vec![1, 2, 1, 3, 2],
            vec![3, 2, 2, 2],
            vec![4, 1, 4],
            vec![2, 4, 1, 3],
            vec![2, 1, 2, 1, 2],
            vec![2, 1, 2, 2, 2],
            vec![4, 2, 1, 1],
            vec![3, 4, 2],
            vec![1, 1, 1, 1, 3],
            vec![1, 4, 3],
            vec![2, 3, 3, 1],
            vec![1, 1, 3, 1, 2],
            vec![1, 1, 3, 1],
            vec![2, 1, 1, 5],
        ],
    )
    .unwrap()
}

fn clues_for_30x30() -> Clues {
    Clues::new(
        vec![
            vec![1, 1, 2, 3, 1, 3, 1],
            vec![1, 2, 1, 1, 1, 1, 2, 2, 2],
            vec![4, 3, 1, 2, 2, 1, 5, 1, 1],
            vec![2, 1, 1, 1, 2, 4, 1, 1, 1],
            vec![1, 3, 1, 2, 1, 1, 2],
            vec![1, 2, 6, 3, 1, 1, 7],
            vec![2, 1, 1, 1, 1, 3, 1, 2],
            vec![1, 1, 2, 3, 1, 1, 1, 2],
            vec![6, 7, 2, 4, 1, 2],
            vec![2, 1, 6, 2, 1, 3, 1, 1, 2],
            vec![1, 2, 7, 1, 1, 1, 1, 2, 1],
            vec![2, 1, 1, 1, 2, 2, 1, 1, 1, 3],
            vec![1, 1, 2, 3, 1, 1, 6],
            vec![1, 2, 3, 2, 1, 2, 4],
            vec![2, 4, 1, 4, 4, 1],
            vec![1, 1, 3, 6, 1, 2, 2, 2],
            vec![2, 3, 1, 3, 3, 1, 3, 1, 1],
            vec![1, 2, 1, 1, 1, 1, 1, 3, 1],
            vec![1, 1, 1, 1, 2, 2, 2, 1, 1, 1],
            vec![2, 3, 10, 1, 1, 2],
            vec![1, 9, 1, 5, 1, 3],
            vec![3, 5, 2, 8, 2, 3],
            vec![1, 1, 2, 1, 1, 1, 3, 2, 1],
            vec![2, 3, 3, 4, 1, 4],
            vec![5, 1, 1, 1, 1, 1, 3, 1],
            vec![1, 1, 6, 1, 1, 2, 2, 2],
            vec![3, 1, 1, 3, 1, 5, 3],
            vec![1, 1, 1, 1, 1, 1, 8, 2],
            vec![1, 1, 2, 2, 1, 1, 2, 1, 2, 1],
            vec![5, 3, 1, 1, 4, 1, 2, 1],
        ],
        vec![
            vec![1, 1, 2, 1, 1, 1, 2, 4, 1, 1, 1],
            vec![3, 2, 1, 1, 1, 1, 6, 2],
            vec![2, 1, 4, 3, 1, 2, 2, 1],
            vec![1, 4, 1, 1, 1, 3, 2],
            vec![1, 1, 1, 2, 1, 2, 1, 3, 1, 1, 1],
            vec![2, 2, 1, 1, 4, 5, 1],
            vec![1, 3, 3, 12, 2, 2],
            vec![1, 2, 1, 1, 2, 3, 2, 1, 1, 3],
            vec![2, 6, 7, 2, 1],
            vec![1, 1, 3, 2, 1, 3, 1, 1],
            vec![2, 5, 4, 2, 2, 1, 1],
            vec![4, 5, 2, 2, 3, 2, 1, 1],
            vec![1, 2, 1, 3, 2, 2, 1, 2],
            vec![1, 2, 1, 2, 2, 2, 2, 2, 2],
            vec![4, 1, 5, 3, 1, 1, 1, 1],
            vec![2, 1, 4, 2, 2, 1],
            vec![4, 4, 5, 1, 1, 5],
            vec![1, 4, 1, 4, 1, 4, 1, 1],
            vec![2, 1, 1, 1, 3, 4],
            vec![2, 1, 4, 1, 6, 1],
            vec![1, 2, 3, 2, 2, 4],
            vec![4, 1, 2, 1, 1, 1, 2, 4],
            vec![2, 2, 1, 2, 1, 1, 1, 1, 2, 1],
            vec![1, 1, 1, 1, 4, 2, 2, 1, 1],
            vec![1, 4, 6, 4, 2, 1],
            vec![3, 1, 2, 3, 3, 2, 3, 1, 1],
            vec![1, 2, 1, 1, 2, 1, 1, 2, 1],
            vec![1, 1, 1, 1, 3, 8, 1, 1],
            vec![2, 6, 2, 1, 1, 1, 3],
            vec![4, 1, 1, 3, 1, 3, 1, 2, 2],
        ],
    )
    .unwrap()
}

#[test]
fn test_10x10() {
    let mut solver = Solver::new(clues_for_10x10());
    assert_eq!(solver.solve(), Ok(Outcome::Solved));
    assert!(solver.judge());
}

#[test]
fn test_15x15() {
    let mut solver = Solver::new(clues_for_15x15());
    assert_eq!(solver.solve(), Ok(Outcome::Solved));
    assert!(solver.judge());
}

#[test]
fn test_30x30() {
    let mut solver = Solver::new(clues_for_30x30());
    let start = Instant::now();
    assert_eq!(solver.solve(), Ok(Outcome::Solved));
    let elapsed = start.elapsed();
    assert!(solver.judge());
    // 性能の tripwire。実測は release で ~1ms / debug で ~10ms のオーダー。
    // これを大きく超えたら solve のホットパスが退行している
    let limit_ms = if cfg!(debug_assertions) { 2_000 } else { 50 };
    println!("30x30 solve: {elapsed:?}");
    assert!(
        elapsed.as_millis() < limit_ms,
        "30x30 solve took {elapsed:?} (limit {limit_ms}ms)"
    );
}

#[test]
fn test_indeterminate_puzzle_is_stuck_not_error() {
    // 2通りの解がある盤面。一意に解けないのはエラーではなく Stuck
    let clues = Clues::new(vec![vec![1], vec![1]], vec![vec![1], vec![1]]).unwrap();
    let mut solver = Solver::new(clues);
    assert_eq!(solver.solve(), Ok(Outcome::Stuck));
    assert!(!solver.grid().is_complete());
    assert!(!solver.judge());
}

#[test]
fn test_no_solution() {
    let mut solver = Solver::new(
        Clues::new(
            vec![vec![], vec![3, 1], vec![], vec![], vec![]],
            vec![vec![1, 3], vec![], vec![], vec![], vec![]],
        )
        .unwrap(),
    );
    assert!(matches!(
        solver.solve(),
        Err(Contradiction::CellConflict { .. } | Contradiction::NoPlacement { .. })
    ));

    let mut solver = Solver::new(
        Clues::new(
            vec![vec![5], vec![1], vec![], vec![], vec![]],
            vec![vec![5], vec![], vec![], vec![], vec![]],
        )
        .unwrap(),
    );
    assert!(matches!(
        solver.solve(),
        Err(Contradiction::CellConflict { .. } | Contradiction::NoPlacement { .. })
    ));
}

// 矛盾エラーの座標は盤面座標のまま報告される（転置されない）
//
// 列0の制約 [1, 3] は列0をちょうど埋める配置（黒・白・黒黒黒）を要求するが、
// 行0・行2〜4の制約が空（全マス白）のため、どこにも配置できない
#[test]
fn test_contradiction_coordinates_are_not_transposed() {
    let mut solver = Solver::new(
        Clues::new(
            vec![vec![], vec![3, 1], vec![], vec![], vec![]],
            vec![vec![1, 3], vec![], vec![], vec![], vec![]],
        )
        .unwrap(),
    );
    match solver.solve() {
        Err(Contradiction::NoPlacement { line, .. }) => {
            assert_eq!(line, LineId::Col(0));
        }
        Err(Contradiction::CellConflict { row: _, col, .. }) => {
            // CellConflict として現れる場合も、矛盾セルは列0上（盤面座標）
            assert_eq!(col, 0);
        }
        other => panic!("expected a contradiction, got {other:?}"),
    }
}

#[test]
fn test_invalid_clues_are_rejected() {
    // ブロックサイズ 0 は拒否される
    assert_eq!(
        Clues::new(vec![vec![0]], vec![vec![1]]),
        Err(ClueError::ZeroBlock {
            line: LineId::Row(0)
        })
    );
    // sum + gaps > 線長 は拒否される
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
    // with_grid は盤面の次元と制約の次元の整合を検証する
    let clues = Clues::new(vec![vec![1]], vec![vec![1]]).unwrap();
    assert_eq!(
        Solver::with_grid(clues.clone(), Grid::new(2, 1)).err(),
        Some(GridMismatch::HeightMismatch {
            expected: 1,
            actual: 2
        })
    );
    assert_eq!(
        Solver::with_grid(clues, Grid::new(1, 2)).err(),
        Some(GridMismatch::WidthMismatch {
            expected: 1,
            actual: 2
        })
    );
}

#[test]
fn test_hint_is_self_consistent() {
    let clues = Clues::new(
        vec![vec![2, 1], vec![3], vec![2, 2], vec![1, 2], vec![1, 1]],
        vec![vec![3, 1], vec![4], vec![1, 1], vec![2], vec![1, 2]],
    )
    .unwrap();
    let mut solver = Solver::new(clues.clone());
    let hint = solver.hint().unwrap().expect("a hint should be available");
    // candidates は塗る範囲と同じ並び・同じ長さで、各セルの候補ブロックが揃う
    assert_eq!(hint.candidates.len(), hint.step.deduction.range.len());
    assert!(hint.candidates.iter().all(|ids| !ids.is_empty()));
    // blocks は step.line の制約と同じ本数・同じサイズ列で揃っている
    let expected_sizes = clues.blocks(hint.step.line);
    assert_eq!(
        hint.blocks.iter().map(|b| b.size).collect::<Vec<_>>(),
        expected_sizes
    );
    assert!(
        hint.blocks
            .iter()
            .all(|b| b.possible_placement.start <= b.possible_placement.end)
    );
    // changed は空でない（新規性フィルタを通った確定なので必ず何か塗れる）
    assert!(!hint.step.changed.is_empty());
    solver.solve().unwrap();
    assert!(solver.hint().unwrap().is_none());
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

fn random_clues(height: usize, width: usize, rng: &mut XorShift) -> Clues {
    let grid: Vec<Vec<bool>> = (0..height)
        .map(|_| {
            let bits = rng.next();
            (0..width).map(|x| bits >> x & 1 == 1).collect()
        })
        .collect();
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
    Clues::new(
        (0..height)
            .map(|y| runs(&mut (0..width).map(|x| grid[y][x])))
            .collect(),
        (0..width)
            .map(|x| runs(&mut (0..height).map(|y| grid[y][x])))
            .collect(),
    )
    .unwrap()
}

// hint() は next_step() が次に行う操作と常に同一
#[test]
fn property_hint_equals_next_step() {
    let mut rng = XorShift(0x0123456789ABCDEF);
    let mut fixtures = vec![clues_for_10x10()];
    for _ in 0..30 {
        fixtures.push(random_clues(7, 7, &mut rng));
        fixtures.push(random_clues(5, 8, &mut rng));
    }
    for clues in fixtures {
        let mut solver = Solver::new(clues);
        for iteration in 0.. {
            assert!(iteration < 10_000, "stepping did not terminate");
            let hint = solver.hint().unwrap();
            let step = solver.next_step().unwrap();
            assert_eq!(hint.map(|h| h.step), step);
            if step.is_none() {
                break;
            }
        }
    }
}

// solve() と next_step() の逐次実行は同じ閉包（最終盤面）に到達する
#[test]
fn property_solve_equals_stepwise_closure() {
    let mut rng = XorShift(0xFEDCBA9876543210);
    let mut fixtures = vec![clues_for_10x10()];
    for _ in 0..30 {
        fixtures.push(random_clues(7, 7, &mut rng));
        fixtures.push(random_clues(5, 8, &mut rng));
    }
    for clues in fixtures {
        let mut batch = Solver::new(clues.clone());
        let batch_outcome = batch.solve().unwrap();

        let mut stepwise = Solver::new(clues);
        for iteration in 0.. {
            assert!(iteration < 10_000, "stepping did not terminate");
            if stepwise.next_step().unwrap().is_none() {
                break;
            }
        }
        assert_eq!(batch.grid(), stepwise.grid());
        assert_eq!(
            batch_outcome == Outcome::Solved,
            stepwise.grid().is_complete()
        );
    }
}

// with_grid: 確定済みセルを種にすると、そこから推論が進む
#[test]
fn test_with_grid_seeds_are_used() {
    let clues = Clues::new(vec![vec![1], vec![1]], vec![vec![1], vec![1]]).unwrap();
    let mut grid = Grid::new(2, 2);
    grid.set(0, 0, CellState::Black);
    let mut solver = Solver::with_grid(clues, grid).unwrap();
    assert_eq!(solver.solve(), Ok(Outcome::Solved));
    assert_eq!(solver.state(0, 1), CellState::White);
    assert_eq!(solver.state(1, 0), CellState::White);
    assert_eq!(solver.state(1, 1), CellState::Black);
}

#[test]
fn test_session_rollback() {
    // 正解盤面から、矛盾なく置ける値を拾って set に使う
    let mut solved = Solver::new(clues_for_10x10());
    solved.solve().unwrap();
    let correct = |i: usize, j: usize| solved.state(i, j);

    let mut session = Session::new(clues_for_10x10());
    session.set(0, 0, correct(0, 0));
    session.set(0, 1, correct(0, 1));
    session.set(1, 1, correct(1, 1));
    assert_eq!(session.state(1, 1), correct(1, 1));

    // 履歴を先頭1件に切り詰めると、それ以降の set は巻き戻る
    session.rollback(1);
    assert_eq!(session.state(0, 0), correct(0, 0));
    assert_eq!(session.state(0, 1), CellState::Unconfirmed);
    assert_eq!(session.state(1, 1), CellState::Unconfirmed);

    // 巻き戻し後も deduce/judge は現盤面から使い捨てソルバで正しく動く
    let grid = session.deduce().expect("この10x10は一意に解けるはず");
    assert_eq!(grid, solved.grid().clone());
}

// 黒 → 未確定 → 白 と書き換えても、その後正解を置けば judge は真になる
// （巻き戻しが後の判定に影響を残さない）
#[test]
fn test_session_judge_after_unconfirmed_rollback() {
    let clues = Clues::new(vec![vec![1], vec![1]], vec![vec![1], vec![1]]).unwrap();
    let mut session = Session::new(clues);
    session.set(0, 0, CellState::Black);
    session.set(0, 0, CellState::Unconfirmed);
    session.set(0, 0, CellState::White);
    session.set(0, 1, CellState::Black);
    session.set(1, 1, CellState::White);
    session.set(1, 0, CellState::Black);
    assert!(session.judge());
}

// 一度 deduce し尽くした後に置いた set も、次の deduce にそのまま反映される
#[test]
fn test_session_set_after_deduce_propagates() {
    let clues = Clues::new(vec![vec![1], vec![1]], vec![vec![1], vec![1]]).unwrap();
    let mut session = Session::new(clues);
    // 2通りの解があり確定しないが、deduce は部分盤面（全 Unconfirmed）を Ok で返す
    let grid = session.deduce().unwrap();
    assert!(!grid.is_complete());

    session.set(0, 0, CellState::Black); // 正解の1つを教える
    let grid = session.deduce().unwrap();
    assert_eq!(grid.get(0, 0), CellState::Black);
    assert_eq!(grid.get(0, 1), CellState::White);
    assert_eq!(grid.get(1, 0), CellState::White);
    assert_eq!(grid.get(1, 1), CellState::Black);
}

// Session::mistakes（間違い検出）のテスト
//
// 一意に解ける2x2: row0=[2]（両方黒）、row1=[]（両方白）
fn clues_for_deterministic_2x2() -> Clues {
    Clues::new(vec![vec![2], vec![]], vec![vec![1], vec![1]]).unwrap()
}

#[test]
fn test_mistakes_no_mistakes_when_grid_matches_clues() {
    let mut session = Session::new(clues_for_deterministic_2x2());
    session.set(0, 0, CellState::Black);
    session.set(0, 1, CellState::Black);
    session.set(1, 0, CellState::White);
    // (1,1) は未記入のまま
    assert_eq!(session.mistakes().unwrap(), Vec::new());
}

#[test]
fn test_mistakes_detects_wrong_cells() {
    let mut session = Session::new(clues_for_deterministic_2x2());
    session.set(0, 0, CellState::White); // 誤り: 制約上は必ず Black
    session.set(0, 1, CellState::Black); // 正しい
    session.set(1, 0, CellState::Black); // 誤り: 制約上は必ず White
    assert_eq!(session.mistakes().unwrap(), vec![(0, 0), (1, 0)]);
}

#[test]
fn test_mistakes_ignores_unfilled_cells() {
    let session = Session::new(clues_for_deterministic_2x2());
    // 何も置いていない盤面はどのマスも「間違い」ではない
    assert_eq!(session.mistakes().unwrap(), Vec::new());
}

// 制約だけからは一切確定しないパズルでは、mistakes は誰の記入も
// 「間違い」と指摘しない（Session::mistakes のdocコメント参照）
#[test]
fn test_mistakes_reports_nothing_when_clues_alone_are_fully_ambiguous() {
    let clues = Clues::new(vec![vec![1], vec![1]], vec![vec![1], vec![1]]).unwrap();
    let mut session = Session::new(clues);
    session.set(0, 0, CellState::Black);
    session.set(1, 1, CellState::Black);
    assert_eq!(session.mistakes().unwrap(), Vec::new());
}

// 制約自体がどうやっても満たせない場合、mistakes はエラーを伝播する
#[test]
fn test_mistakes_propagates_error_when_clues_are_unsatisfiable() {
    let session = Session::new(
        Clues::new(
            vec![vec![], vec![3, 1], vec![], vec![], vec![]],
            vec![vec![1, 3], vec![], vec![], vec![], vec![]],
        )
        .unwrap(),
    );
    assert!(session.mistakes().is_err());
}

// Session::hint は現盤面（ユーザーの記入込み）を前提に次の1手を返す
#[test]
fn test_session_hint_reflects_user_grid() {
    let clues = Clues::new(vec![vec![1], vec![1]], vec![vec![1], vec![1]]).unwrap();
    let mut session = Session::new(clues);
    // まっさらな盤面では何も確定できない
    assert!(session.hint().unwrap().is_none());
    // 1マス教えると次の1手が出る
    session.set(0, 0, CellState::Black);
    let hint = session.hint().unwrap().expect("hint should be available");
    assert!(!hint.step.changed.is_empty());
}
