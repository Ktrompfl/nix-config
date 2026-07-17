use futures_util::{AsyncBufReadExt, io::BufReader};
use jay_config::{exec::Command, io::Async, tasks::spawn};

/// Runs `command` once as a long-lived subprocess and passes each line it
/// prints on stdout to `on_line`, for commands that stream updates rather
/// than exiting (e.g. `swaync-client -swb`, which stays subscribed and
/// prints a new line whenever notification state changes).
pub fn capture_lines(command: &mut Command, on_line: impl Fn(String) + 'static) {
    let Ok((read, write)) = uapi::pipe2(uapi::c::O_CLOEXEC) else {
        return;
    };
    let Ok(read) = Async::new(read) else {
        return;
    };
    command.stdout(write).spawn();
    spawn(async move {
        let mut read = BufReader::new(read);
        let mut line = String::new();
        loop {
            line.clear();
            match read.read_line(&mut line).await {
                Ok(0) | Err(_) => return,
                Ok(_) => {}
            }
            on_line(line.trim_end_matches('\n').to_string());
        }
    });
}
