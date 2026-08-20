//! イラストロジック（ノノグラム、お絵かきロジック）のルールベースソルバ。
//!
//! 制約（各行・各列の黒マスの連続長の並び）を [`Clues::new`] で検証して
//! [`Solver`] に渡すと、人間が使う8つの行内推論規則を適用して盤面を埋める。
//! 背理法や二択試行のような探索は行わない。そのぶん、確定した各マスについて
//! 「なぜそう言えるか」（[`Reason`]）を常に説明でき、ヒント機能
//! （[`Solver::hint`]）はこの資産の上に成り立つ。
//!
//! # 構成
//!
//! - [`Clues`] — 検証済みの制約。パズル入力はまずここに「パース」され、
//!   以降のAPIは失敗しない（parse, don't validate）。
//! - [`Grid`] — 盤面。セル状態の唯一の所有者となる値型。
//! - [`Solver`] — 推論エンジン。フルソルブ（[`Solver::solve`]）と、
//!   1手ずつのステップ実行・ヒント（[`Solver::next_step`] /
//!   [`Solver::hint`]。両者は構成上、常に同じ手を返す）。
//! - [`Session`] — 対話層。ユーザーの盤面と操作履歴だけを真実として持ち、
//!   問い合わせのたびに使い捨ての `Solver` を構築する。
//!
//! # クイックスタート
//!
//! ```
//! use illu_logi_solver_super::{Clues, Outcome, Solver};
//!
//! let clues = Clues::new(
//!     // 行の制約（上から）と列の制約（左から）。
//!     vec![vec![2, 1], vec![3], vec![2, 2], vec![1, 2], vec![1, 1]],
//!     vec![vec![3, 1], vec![4], vec![1, 1], vec![2], vec![1, 2]],
//! )
//! .unwrap();
//! let mut solver = Solver::new(clues);
//! assert_eq!(solver.solve().unwrap(), Outcome::Solved);
//! println!("{solver}");
//! ```
//!
//! 詳しい使い方（制約の与え方、`Solver` と `Session` の使い分け）は
//! リポジトリの README を参照。
#![warn(missing_docs)]

mod analysis;
mod clue;
mod grid;
mod rules;
mod session;
mod solver;

pub use clue::{ClueError, Clues, LineId};
pub use grid::{CellState, Color, Grid, GridMismatch};
pub use rules::{Deduction, Reason};
pub use session::Session;
pub use solver::{Contradiction, Hint, HintBlock, Outcome, Solver, Step};
