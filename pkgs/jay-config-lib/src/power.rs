//! A fuzzel menu for locking, suspending, logging out and powering off.

use std::io::{self, ErrorKind, PipeReader, Read, Write};

use jay_config::{exec::Command, io::Async, tasks};

use crate::{exec, idle};

/// label and action of each entry, in menu order
fn entries() -> Vec<(&'static str, fn())> {
    vec![
        ("Lock", || exec("swaylock", &["--daemonize"])),
        ("Suspend", idle::suspend),
        (
            if idle::inhibited() {
                "Allow idle"
            } else {
                "Inhibit idle"
            },
            idle::toggle_inhibitor,
        ),
        // stops the session's services before jay itself
        ("Log out", || {
            exec("systemctl", &["--user", "stop", "jay.service"])
        }),
        ("Reboot", || exec("systemctl", &["reboot"])),
        ("Reboot to UEFI", || {
            exec("systemctl", &["reboot", "--firmware-setup"])
        }),
        ("Shut down", || exec("systemctl", &["poweroff"])),
    ]
}

async fn read_to_end(pipe: PipeReader) -> io::Result<Vec<u8>> {
    let mut pipe = Async::new(pipe)?;
    let mut buf = vec![];
    loop {
        pipe.readable().await?;
        match pipe.as_mut().read_to_end(&mut buf) {
            Ok(_) => return Ok(buf),
            Err(e) if e.kind() == ErrorKind::WouldBlock => {}
            Err(e) => return Err(e),
        }
    }
}

async fn run() -> io::Result<()> {
    let entries = entries();
    let (stdin, mut input) = io::pipe()?;
    let (output, stdout) = io::pipe()?;
    // a few lines, so this cannot block on a full pipe
    for (label, _) in &entries {
        writeln!(input, "{label}")?;
    }
    drop(input);
    Command::new("fuzzel")
        .arg("--dmenu")
        .arg("--index")
        .arg(&format!("--lines={}", entries.len()))
        .arg("--prompt=power: ")
        .stdin(stdin)
        .stdout(stdout)
        .spawn();
    // fuzzel prints nothing when the menu is dismissed
    let selection = String::from_utf8_lossy(&read_to_end(output).await?).into_owned();
    if let Ok(index) = selection.trim().parse::<usize>()
        && let Some((_, action)) = entries.get(index)
    {
        action();
    }
    Ok(())
}

pub fn menu() {
    tasks::spawn(async {
        if let Err(e) = run().await {
            log::error!("power menu failed: {e}");
        }
    });
}
