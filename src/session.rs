//! 対話層。ユーザーの盤面と操作履歴だけを真実（ground truth）として持つ、
//! [`crate::Solver`] の薄いラッパー
//!
//! ソルバの状態は一切保持せず、問い合わせのたびに現在の盤面から使い捨ての
//! [`crate::Solver`] を構築する（フルソルブが数msなので、対話1操作ごとに
//! 再構築してよい）

use crate::clue::Clues;
use crate::grid::{CellState, Grid};
use crate::solver::{Contradiction, Hint, Solver};

/// [`Session::set`] 1回分の記録。undo / rollback の再生に使う
#[derive(Debug, Clone, Copy)]
struct Edit {
    row: usize,
    col: usize,
    state: CellState,
}

/// ユーザーとの対話用の薄い層
///
/// 保持するのは制約・盤面・操作履歴のみ。ソルバ状態は一切持たず、
/// 推論が必要になるたびに使い捨ての [`Solver`] を構築する
///
/// ```
/// use illu_logi_solver_super::{CellState, Clues, Session};
///
/// let clues = Clues::new(vec![vec![2], vec![]], vec![vec![1], vec![1]]).unwrap();
/// let mut session = Session::new(clues);
/// session.set(0, 0, CellState::Black);
/// session.set(0, 1, CellState::Black);
/// session.set(1, 0, CellState::White);
/// session.set(1, 1, CellState::White);
/// assert!(session.judge());
/// assert_eq!(session.mistakes().unwrap(), Vec::new());
/// ```
pub struct Session {
    clues: Clues,
    grid: Grid,
    history: Vec<Edit>,
}

impl Session {
    /// 制約から空盤面のセッションを作る
    pub fn new(clues: Clues) -> Self {
        let grid = Grid::new(clues.height(), clues.width());
        Self {
            clues,
            grid,
            history: Vec::new(),
        }
    }

    /// パズルの制約
    pub fn clues(&self) -> &Clues {
        &self.clues
    }

    /// 現在の盤面
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// 盤面 `(row, col)` を `state` に書き換え、履歴に記録する
    /// （[`CellState::Unconfirmed`] への巻き戻しも可）
    pub fn set(&mut self, row: usize, col: usize, state: CellState) {
        self.grid.set(row, col, state);
        self.history.push(Edit { row, col, state });
    }

    /// 盤面 `(row, col)` の現在の状態
    pub fn state(&self, row: usize, col: usize) -> CellState {
        self.grid.get(row, col)
    }

    /// 現盤面を種にした使い捨てソルバを構築する
    ///
    /// `Session` の盤面は構築時から常に制約と同じ次元なので、
    /// `with_grid` の次元検証は失敗し得ない
    fn solver(&self) -> Solver {
        Solver::with_grid(self.clues.clone(), self.grid.clone())
            .expect("Session の盤面は常に制約の次元と一致する")
    }

    /// 現盤面から、次の1手とその根拠を非破壊で求める
    ///
    /// 毎回新品のソルバを構築して [`Solver::hint`] に委譲するため、
    /// 直前の `hint`/`deduce` 呼び出しの影響を受けない
    pub fn hint(&self) -> Result<Option<Hint>, Contradiction> {
        self.solver().hint()
    }

    /// 現盤面から確定できる範囲まで推論した結果の盤面を返す
    ///
    /// 内部で新品のソルバに [`Solver::solve`] を走らせるだけで、`Session`
    /// 自体の盤面・履歴は変更しない。一意に確定しきれない場合もそこまでの
    /// 部分盤面を `Ok` で返す（全確定したかは [`Grid::is_complete`] で
    /// 判定できる）。`Err` になるのは矛盾（[`Contradiction`]）のみ
    pub fn deduce(&self) -> Result<Grid, Contradiction> {
        let mut solver = self.solver();
        solver.solve()?;
        Ok(solver.grid().clone())
    }

    /// 現盤面が制約をすべて満たしているか（[`Grid::satisfies`] 参照）
    pub fn judge(&self) -> bool {
        self.grid.satisfies(&self.clues)
    }

    /// 制約だけから求めた解答と現盤面を突き合わせ、食い違う確定セルの
    /// 座標 `(row, col)` を返す
    ///
    /// [`Session::hint`] / [`Session::deduce`] と違い、ユーザーの誤記入が
    /// 判定の前提に混ざらない
    ///
    /// - 比較するのは、ユーザーが記入済みかつ解答側も確定しているセルのみ
    /// - 制約だけでは一意に解けなくても、確定できたセルに限って検出する
    /// - 制約自体が解を持たない場合は `Err`
    pub fn mistakes(&self) -> Result<Vec<(usize, usize)>, Contradiction> {
        let mut solver = Solver::new(self.clues.clone());
        solver.solve()?;
        let answer = solver.grid();
        let mut mistakes = Vec::new();
        for row in 0..self.grid.height() {
            for col in 0..self.grid.width() {
                let user = self.grid.get(row, col);
                let correct = answer.get(row, col);
                if user != CellState::Unconfirmed
                    && correct != CellState::Unconfirmed
                    && user != correct
                {
                    mistakes.push((row, col));
                }
            }
        }
        Ok(mistakes)
    }

    /// これまでの [`Session::set`] の回数（＝現在の履歴件数）。
    /// [`Session::rollback`] に渡す `t` の基準になる
    pub fn turn(&self) -> usize {
        self.history.len()
    }

    /// 直近の [`Session::set`] を1回取り消す
    pub fn undo(&mut self) {
        self.history.pop();
        self.rebuild_grid();
    }

    /// 履歴を先頭 `t` 件に切り詰め、盤面を再構築する
    ///
    /// 現在の履歴件数は [`Session::turn`] で取得できる。`t` がそれ以上なら
    /// 何もしない（`Vec::truncate` と同じ挙動）
    pub fn rollback(&mut self, t: usize) {
        self.history.truncate(t);
        self.rebuild_grid();
    }

    fn rebuild_grid(&mut self) {
        self.grid = Grid::new(self.clues.height(), self.clues.width());
        for edit in &self.history {
            self.grid.set(edit.row, edit.col, edit.state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_and_rollback_replay_history() {
        let clues = Clues::new(vec![vec![1], vec![1]], vec![vec![1], vec![1]]).unwrap();
        let mut session = Session::new(clues);
        session.set(0, 0, CellState::Black);
        session.set(0, 1, CellState::White);
        session.set(1, 0, CellState::White);

        session.undo();
        assert_eq!(session.state(1, 0), CellState::Unconfirmed);
        assert_eq!(session.state(0, 0), CellState::Black);

        session.set(1, 0, CellState::White);
        session.set(1, 1, CellState::Black);
        assert!(session.judge());

        session.rollback(1);
        assert_eq!(session.state(0, 0), CellState::Black);
        assert_eq!(session.state(0, 1), CellState::Unconfirmed);
        assert_eq!(session.state(1, 0), CellState::Unconfirmed);
        assert_eq!(session.state(1, 1), CellState::Unconfirmed);
    }
}
