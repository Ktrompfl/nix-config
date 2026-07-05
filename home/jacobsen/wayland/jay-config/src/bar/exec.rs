use futures_util::AsyncReadExt;
use jay_config::{exec::Command, io::Async, tasks::spawn};

/// Runs `command`, waits for it to exit, and passes its stdout to `on_output`.
pub fn capture(command: &mut Command, on_output: impl FnOnce(String) + 'static) {
    let Ok((read, write)) = uapi::pipe2(uapi::c::O_CLOEXEC) else {
        return;
    };
    let Ok(mut read) = Async::new(read) else {
        return;
    };
    command.stdout(write).spawn();
    spawn(async move {
        let mut output = String::new();
        let _ = read.read_to_string(&mut output).await;
        on_output(output);
    });
}
