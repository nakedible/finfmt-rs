#[path = "codecs/nibss.rs"]
mod nibss;

use std::env;
use std::io::{self, Read, Write};

use finfmt::CompositeFmt;

fn main() {
    if let Err(err) = run() {
        let _ = writeln!(io::stderr(), "{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mode = env::args().nth(1).ok_or_else(|| "usage: nibss <to-json|from-json>".to_string())?;
    let mut input = Vec::new();
    io::stdin()
        .read_to_end(&mut input)
        .map_err(|err| format!("stdin read failed: {err}"))?;
    match mode.as_str() {
        "to-json" => {
            let mut wire = input.as_slice();
            let mut scratch = vec![0u8; input.len().max(4096)];
            let value =
                nibss::NibssMessageFmt::decode(&mut wire, scratch.as_mut_slice()).map_err(|err| format!("decode failed: {err:?}"))?;
            if !wire.is_empty() {
                return Err("decode failed: trailing bytes remain".into());
            }
            serde_json::to_writer(io::stdout().lock(), &value).map_err(|err| format!("json write failed: {err}"))?;
            Ok(())
        }
        "from-json" => {
            let value: nibss::NibssMessage = serde_json::from_slice(&input).map_err(|err| format!("json parse failed: {err}"))?;
            let mut output = vec![0u8; 8192];
            let total = output.len();
            let mut out_ptr = output.as_mut_slice();
            let mut scratch = vec![0u8; 8192];
            nibss::NibssMessageFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value)
                .map_err(|err| format!("encode failed: {err:?}"))?;
            let used = total - out_ptr.len();
            io::stdout()
                .lock()
                .write_all(&output[..used])
                .map_err(|err| format!("stdout write failed: {err}"))?;
            Ok(())
        }
        _ => Err("usage: nibss <to-json|from-json>".into()),
    }
}
