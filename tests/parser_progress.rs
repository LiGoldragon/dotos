//! The parser must always terminate — a misplaced pipe-close (`|]`, `|)`, `|}`)
//! at an object position must not spin `parse_atom` on a zero-width atom (which
//! grows the block vector unboundedly → OOM). Each parse runs in a watchdog
//! thread so a regression hangs the worker, not the test runner; run the test
//! binary under a memory cap so the hung worker is contained, not the machine.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use dotos::Document;

fn parse_terminates(input: &str) -> bool {
    let owned = input.to_string();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = Document::parse(owned);
        let _ = tx.send(());
    });
    rx.recv_timeout(Duration::from_secs(4)).is_ok()
}

#[test]
fn parser_terminates_on_stray_pipe_close() {
    for input in [
        "(a |])",
        "[a |] b]",
        "(|])",
        "( |] )",
        "{ |} }",
        "( |) )",
        "(record [|]x)",
    ] {
        assert!(
            parse_terminates(input),
            "parser did not terminate on {input:?}"
        );
    }
}
