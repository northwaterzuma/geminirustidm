use rustcatch_nmhost::NativeHostSession;
use std::io;

fn main() -> io::Result<()> {
    let origin = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "chrome-extension://rustcatchdevid000000000000000000/".to_string());
    let allowed = vec![
        "chrome-extension://rustcatchdevid000000000000000000/".to_string(),
        "chrome-extension://rustcatchprodid00000000000000000/".to_string(),
    ];
    let mut session = NativeHostSession::new(allowed, origin);
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    while let Some(frame_res) = NativeHostSession::read_native_frame(&mut reader)? {
        let resp = match frame_res {
            Ok(bytes) => session.handle_raw_frame(&bytes, 1_794_815_091_000),
            Err(err) => mdm_ipc::BrowserBridgeResponse::error("frame-err", err.code, err.message, false, None),
        };
        NativeHostSession::write_native_frame(&mut writer, &resp)?;
    }
    Ok(())
}
