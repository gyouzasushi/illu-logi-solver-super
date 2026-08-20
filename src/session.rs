//! 対話層。ユーザーの盤面と操作履歴だけを真実（ground truth）として持つ、
//! [`crate::Solver`] の薄いラッパー。
//!
//! `Session` はソルバの状態を一切保持しない。[`Session::hint`] /
//! [`Session::deduce`] / [`Session::judge`] を呼ぶたびに、現在の盤面から
//! [`crate::Solver::with_grid`] で使い捨ての `Solver` を毎回新品で構築して
//! 問い合わせる。フルソルブが数msという前提のもと、対話1操作ごとに全体を
//! 再構築しても実用上問題ない。
//!
//! この設計により「未確定への巻き戻しで推論状態が残留する」
//! 「推論し尽くした後の書き込みが伝播しない」といった、可変な推論状態を
//! 外部から書き換えることに起因するバグは発生し得ない。

use crate::clue::Clues;
use crate::grid::{CellState, Grid};
use crate::solver::{Contradiction, Hint, Solver};

/// [`Session::set`] 1回分の記録。undo / rollback の再生に使う。
#[derive(Debug, Clone, Copy)]
struct Edit {
    row: usize,
    col: usize,
    state: CellState,
}

/// ユーザーとの対話用の薄い層。
///
/// 保持するのは制約・盤面・操作履歴のみ。ソルバ状態は一切持たず、
/// 推論が必要になるたびに使い捨ての [`Solver`] を構築する。
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
    /// 制約から空盤面のセッションを作る。制約は [`Clues::new`] で検証済み
    /// なので、この構築は失敗しない。
    pub fn new(clues: Clues) -> Self {
        let grid = Grid::new(clues.height(), clues.width());
        Self {
            clues,
            grid,
            history: Vec::new(),
        }
    }

    /// 検証済みの制約への参照。
    pub fn clues(&self) -> &Clues {
        &self.clues
    }

    /// 現在の盤面への参照。
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// 盤面 `(row, col)` を `state` に書き換え、履歴に記録する。
    ///
    /// 盤面の書き換えと履歴への記録のみ。[`CellState::Unconfirmed`] への
    /// 巻き戻しもただの代入なので常に正しい。
    pub fn set(&mut self, row: usize, col: usize, state: CellState) {
        self.grid.set(row, col, state);
        self.history.push(Edit { row, col, state });
    }

    /// 盤面 `(row, col)` の現在の状態。
    pub fn state(&self, row: usize, col: usize) -> CellState {
        self.grid.get(row, col)
    }

    /// 現盤面を種にした使い捨てソルバを構築する。
    ///
    /// `Session` の盤面は構築時から常に制約と同じ次元なので、
    /// `with_grid` の次元検証は失敗し得ない。
    fn solver(&self) -> Solver {
        Solver::with_grid(self.clues.clone(), self.grid.clone())
            .expect("Session の盤面は常に制約の次元と一致する")
    }

    /// 現盤面から、次の1手とその根拠を非破壊で求める。
    ///
    /// 毎回新品のソルバを構築して [`Solver::hint`] に委譲するため、
    /// 直前の `hint`/`deduce` 呼び出しの影響を受けない。
    pub fn hint(&self) -> Result<Option<Hint>, Contradiction> {
        self.solver().hint()
    }

    /// 現盤面から確定できる範囲まで推論した結果の盤面を返す。
    ///
    /// 内部で新品のソルバに [`Solver::solve`] を走らせるだけで、`Session`
    /// 自体の盤面・履歴は変更しない。一意に確定しきれない場合もそこまでの
    /// 部分盤面を `Ok` で返す（全確定したかは [`Grid::is_complete`] で
    /// 判定できる）。`Err` になるのは矛盾（[`Contradiction`]）のみ。
    pub fn deduce(&self) -> Result<Grid, Contradiction> {
        let mut solver = self.solver();
        solver.solve()?;
        Ok(solver.grid().clone())
    }

    /// 現盤面が制約をすべて満たしているか（[`Grid::satisfies`] 参照）。
    pub fn judge(&self) -> bool {
        self.grid.satisfies(&self.clues)
    }

    /// 制約だけから求めた解答と現盤面を突き合わせ、食い違う確定セルの
    /// 座標 `(row, col)` を返す。
    ///
    /// [`Session::hint`] / [`Session::deduce`] は現盤面（ユーザーの記入を
    /// 含む）を種にソルバを構築するため、ユーザーが誤って置いた黒がそのまま
    /// 推論の前提になり、以後の確定がその誤りを引きずる弱点がある。
    /// `mistakes` はこれを避けるため、盤面を種にせず**制約のみ**から解いた
    /// 結果を「解答」として使う。
    ///
    /// 比較は「ユーザー側が未記入でない」かつ「解答側が確定している」セルに
    /// ついてのみ行う。前者は「書いていないマスは間違いではない」ため、
    /// 後者は「制約だけからはまだ判断がつかないマスについて、記入の是非を
    /// 判定しようがない」ため対象外にする。制約だけでは一意に解けない場合も
    /// エラーにはせず、確定できたセルに限って間違いを検出する（best-effort。
    /// 多くの実用的な盤面では部分確定だけでも誤りを早期に指摘できる）。
    /// 制約自体が矛盾していて解を持たない場合は `Err` を返す。
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
    /// [`Session::rollback`] に渡す `t` の基準になる。
    pub fn turn(&self) -> usize {
        self.history.len()
    }

    /// 直近の [`Session::set`] を1回取り消す。
    pub fn undo(&mut self) {
        self.history.pop();
        self.rebuild_grid();
    }

    /// 履歴を先頭 `t` 件に切り詰め、盤面を再構築する。
    ///
    /// 現在の履歴件数は [`Session::turn`] で取得できる。`t` がそれ以上なら
    /// 何もしない（`Vec::truncate` と同じ挙動）。
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
