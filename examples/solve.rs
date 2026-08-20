//! テキスト形式のパズルを読み込んで解くCLIデモ
//!
//! ```sh
//! cargo run --example solve -- examples/data/15x15.txt
//! cargo run --example solve -- --steps examples/data/15x15.txt
//! ```
//!
//! 入力形式: 空行より前が行の制約（上から）、後が列の制約（左から）。
//! 各行は黒マスの連続長を空白区切りで並べ、空の制約（全マス白）は `-`。
//! `#` で始まる行はコメント。ファイルを省略すると標準入力から読む
//!
//! `--steps` を付けると1手ずつ実行し、各手の根拠を日本語で説明する

use illu_logi_solver_super::*;
use std::fmt::Write as _;
use std::io::Read as _;
use std::process::ExitCode;

fn parse_clue_line(line: &str) -> Result<Vec<usize>, String> {
    if line == "-" {
        return Ok(Vec::new());
    }
    line.split_whitespace()
        .map(|token| {
            token
                .parse::<usize>()
                .map_err(|_| format!("数値として読めません: {token:?}"))
        })
        .collect()
}

fn parse_puzzle(text: &str) -> Result<Clues, String> {
    let mut sections: Vec<Vec<Vec<usize>>> = vec![Vec::new()];
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if line.is_empty() {
            if !sections.last().unwrap().is_empty() {
                sections.push(Vec::new());
            }
            continue;
        }
        sections.last_mut().unwrap().push(parse_clue_line(line)?);
    }
    if sections.last().is_some_and(Vec::is_empty) {
        sections.pop();
    }
    let [rows, cols] = <[_; 2]>::try_from(sections).map_err(|s| {
        format!(
            "行制約・列制約の2セクションが必要です（{}個ありました）",
            s.len()
        )
    })?;
    Clues::new(rows, cols).map_err(|e| format!("制約が不正です: {e}"))
}

/// `Hint` を日本語の説明文にする
fn describe(hint: &Hint) -> String {
    let step = &hint.step;
    let line = match step.line {
        LineId::Row(i) => format!("行{i}"),
        LineId::Col(j) => format!("列{j}"),
    };
    let range = &step.deduction.range;
    let cells = if range.len() == 1 {
        format!("{line} のマス{}", range.start)
    } else {
        format!("{line} のマス{}..{}", range.start, range.end)
    };
    let color = match step.deduction.color {
        Color::Black => "黒",
        Color::White => "白",
    };
    let why = match step.deduction.reason {
        Reason::BlackIfOverlap { id, .. } => {
            let block = &hint.blocks[id];
            format!(
                "ブロック{id}（長さ{}）はマス{}..{}のどこかに入るしかなく、左右どちらに寄せてもここは必ず黒",
                block.size, block.possible_placement.start, block.possible_placement.end
            )
        }
        Reason::BlackIfBounded(l, r) => format!(
            "マス{l}..{r}は両端が区切られていて、ここに入り得るブロックはどれも長さ{}以上なので全体が黒",
            r - l
        ),
        Reason::BlackIfLeftBounded(l, _) => format!(
            "マス{l}が左端で確定しているので、入り得るブロックの最小の長さぶんだけ右へ黒が続く"
        ),
        Reason::BlackIfRightBounded(_, r) => format!(
            "マス{}が右端で確定しているので、入り得るブロックの最小の長さぶんだけ左へ黒が続く",
            r - 1
        ),
        Reason::WhiteIfSegmentComplete(l, r) => format!(
            "マス{l}..{r}の黒のまとまりは、ここに入り得るブロックの最大の長さに達していて、これ以上は延びない"
        ),
        Reason::WhiteIfTooLong { id, .. } => {
            let block = &hint.blocks[id];
            format!(
                "ここを黒にすると隣の黒とつながって、唯一入り得るブロック{id}（長さ{}）より長くなってしまう",
                block.size
            )
        }
        Reason::WhiteIfTooShort(l, r) => format!(
            "マス{l}..{r}は幅{}しかなく、ここに入り得るブロックはどれも収まらない",
            r - l
        ),
        Reason::WhiteIfNoBlockCovers(_, _) => {
            "どのブロックの置き方でもここが黒になることはない".to_string()
        }
    };
    format!("{cells} を{color}に確定: {why}")
}

fn run() -> Result<(), String> {
    let mut steps = false;
    let mut path: Option<String> = None;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--steps" => steps = true,
            "--help" | "-h" => {
                println!("usage: solve [--steps] [FILE]");
                println!(
                    "FILE を省略すると標準入力から読む。形式は examples/data/15x15.txt 参照。"
                );
                return Ok(());
            }
            _ if path.is_none() => path = Some(arg),
            _ => return Err(format!("引数が多すぎます: {arg:?}")),
        }
    }
    let text = match path {
        Some(path) => {
            std::fs::read_to_string(&path).map_err(|e| format!("{path} を読めません: {e}"))?
        }
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| format!("標準入力を読めません: {e}"))?;
            buf
        }
    };
    let clues = parse_puzzle(&text)?;
    println!(
        "{}x{} のパズルを読み込みました。",
        clues.height(),
        clues.width()
    );

    let mut solver = Solver::new(clues);
    let result = if steps {
        let mut count = 0;
        loop {
            match solver.hint() {
                Ok(Some(hint)) => {
                    count += 1;
                    println!("{count:4}. {}", describe(&hint));
                    solver
                        .next_step()
                        .expect("hint が Ok ならその適用も矛盾しない");
                }
                Ok(None) => break Ok(()),
                Err(e) => break Err(e),
            }
        }
    } else {
        solver.solve().map(|_| ())
    };

    match result {
        Ok(()) => {
            println!("{solver}");
            if solver.judge() {
                println!("一意に解けました。");
            } else {
                println!("矛盾はありませんが、この規則群では一意に確定できませんでした。");
            }
            Ok(())
        }
        Err(e) => {
            let mut message = String::new();
            let _ = writeln!(message, "矛盾を検出しました: {e}");
            let _ = write!(message, "そこまでの盤面:\n{solver}");
            Err(message)
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
