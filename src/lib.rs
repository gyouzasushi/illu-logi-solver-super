//! イラストロジック（ノノグラム、お絵かきロジック）のルールベースソルバ
//!
//! 制約（各行・各列の黒マスの連続長の並び）を [`Clues::new`] で検証して
//! [`Solver`] に渡すと、人間が使う8つの行内推論規則で盤面を埋める。
//! 背理法などの探索はしない。かわりに、確定した各マスの根拠（[`Reason`]）を
//! [`Solver::hint`] で説明できる
//!
//! # 構成
//!
//! - [`Clues`] — 検証済みの制約。入力の検証はここに集約されている
//! - [`Grid`] — 盤面
//! - [`Solver`] — 推論エンジン。フルソルブは [`solve`](Solver::solve)、
//!   ステップ実行とヒントは [`next_step`](Solver::next_step) と
//!   [`hint`](Solver::hint)（両者は常に同じ手を返す）
//! - [`Session`] — 対話層。ユーザーの盤面と操作履歴を持ち、
//!   問い合わせのたびに使い捨ての `Solver` を作る
//!
//! # クイックスタート
//!
//! ```
//! use illu_logi_solver_super::{Clues, Outcome, Solver};
//!
//! let clues = Clues::new(
//!     // 行の制約（上から）と列の制約（左から）
//!     vec![vec![2, 1], vec![3], vec![2, 2], vec![1, 2], vec![1, 1]],
//!     vec![vec![3, 1], vec![4], vec![1, 1], vec![2], vec![1, 2]],
//! )
//! .unwrap();
//! let mut solver = Solver::new(clues);
//! assert_eq!(solver.solve().unwrap(), Outcome::Solved);
//! println!("{solver}");
//! ```
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
