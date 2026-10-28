//! A fuzzel menu for locking, suspending, logging out and powering off.

use std::io::{self, ErrorKind, PipeReader, Read, Write};

use jay_config::{exec::Command, io::Async, tasks};

use crate::{exec, idle};

/// label, icon from the icon theme, and action of each entry, in menu order
fn entries() -> Vec<(&'static str, &'static str, fn())> {
    vec![
        ("Lock", "system-lock-screen", || {
            exec("swaylock", &["--daemonize"])
        }),
        ("Suspend", "system-suspend", idle::suspend),
        (
            if idle::inhibited() {
                "Allow idle"
            } else {
                "Inhibit idle"
            },
            "caffeine",
            idle::toggle_inhibitor,
        ),
        // stops the session's services before jay itself
        ("Log out", "system-log-out", || {
            exec("systemctl", &["--user", "stop", "jay.service"])
        }),
        ("Reboot", "system-reboot", || exec("systemctl", &["reboot"])),
        ("Reboot to UEFI", "firmware-manager", || {
            exec("systemctl", &["reboot", "--firmware-setup"])
        }),
        ("Shut down", "system-shutdown", || {
            exec("systemctl", &["poweroff"])
        }),
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
    for (label, icon, _) in &entries {
        writeln!(input, "{label}\0icon\x1f{icon}")?;
    }
    drop(input);
    Command::new("fuzzel")
        .arg("--dmenu")
        .arg("--index")
        .arg(&format!("--lines={}", entries.len()))
        .arg("--prompt=$ ")
        .stdin(stdin)
        .stdout(stdout)
        .spawn();
    // fuzzel prints nothing when the menu is dismissed
    let selection = String::from_utf8_lossy(&read_to_end(output).await?).into_owned();
    if let Ok(index) = selection.trim().parse::<usize>()
        && let Some((_, _, action)) = entries.get(index)
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
