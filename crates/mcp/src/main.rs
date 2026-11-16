use mdm_app::TaskCoordinator;
use rustcatch_mcp::McpServer;
use std::io::{self, BufRead, Write};

fn main() -> io::Result<()> {
    let server = McpServer::default_stdio_Read_add_control();
    let mut coordinator = TaskCoordinator::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let l = line?;
        if l.trim().is_empty() {
            continue;
        }
        let req: serde_json::Value = serde_json::from_str(&l).unwrap_or_default();
        let tool = req["tool"].as_str().unwrap_or("");
        let args = &req["arguments"];
        let out = match server.invoke_tool(&mut coordinator, tool, args, 1_794_815_091_000) {
            Ok(val) => serde_json::json!({ "status": "ok", "result": val }),
            Err(err) => serde_json::json!({ "status": "error", "error": err }),
        };
        writeln!(stdout, "{}", out)?;
        stdout.flush()?;
    }
    Ok(())
}
