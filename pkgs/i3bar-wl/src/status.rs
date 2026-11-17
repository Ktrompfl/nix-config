use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::{self, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Block {
    #[serde(default)]
    pub full_text: String,
    pub color: Option<String>,
    pub background: Option<String>,
    pub min_width: Option<Value>,
    pub align: Option<String>,
    pub name: Option<String>,
    pub instance: Option<String>,
    pub separator_block_width: Option<i32>,
    pub separator: Option<bool>,
    #[serde(default)]
    pub urgent: bool,
    pub markup: Option<String>,
}

pub struct Status {
    child: Child,
    stdin: ChildStdin,
    pending: Vec<u8>,
    json: Option<bool>,
    clicks: bool,
    sent: bool,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Copy)]
pub struct Click {
    pub button: u32,
    pub position: (f64, f64),
    pub relative: (f64, f64),
    pub size: (f64, f64),
}

impl Status {
    pub fn spawn(command: &[String], env: &HashMap<String, String>) -> io::Result<(Self, ChildStdout)> {
        let (program, args) = command.split_first().ok_or(io::ErrorKind::InvalidInput)?;
        let mut child =
            Command::new(program).args(args).envs(env).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
        let (stdin, stdout) = (child.stdin.take().expect("piped"), child.stdout.take().expect("piped"));
        let status = Self { child, stdin, pending: vec![], json: None, clicks: false, sent: false, blocks: vec![] };
        Ok((status, stdout))
    }

    pub fn feed(&mut self, bytes: &[u8]) -> bool {
        self.pending.extend_from_slice(bytes);
        let mut changed = false;
        while let Some(end) = self.pending.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=end).collect();
            let line = String::from_utf8_lossy(&line);
            if line.trim().is_empty() {
                continue;
            }
            let blocks = match *self.json.get_or_insert_with(|| line.trim_start().starts_with('{')) {
                false => vec![Block { full_text: line.trim_end().to_string(), ..Block::default() }],
                true => match line.trim().trim_matches(',').trim() {
                    line if line.starts_with('{') => {
                        let header: Value = serde_json::from_str(line).unwrap_or_default();
                        self.clicks = header["click_events"].as_bool().unwrap_or(false);
                        if self.clicks {
                            let _ = self.stdin.write_all(b"[\n");
                        }
                        continue;
                    }
                    line if line.starts_with('[') && line != "[" => match serde_json::from_str(line) {
                        Ok(blocks) => blocks,
                        Err(e) => {
                            log::warn!("bad status line: {e}");
                            continue;
                        }
                    },
                    _ => continue,
                },
            };
            if blocks != self.blocks {
                (self.blocks, changed) = (blocks, true);
            }
        }
        changed
    }

    pub fn stop(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    pub fn click(&mut self, index: usize, click: Click) {
        let Some(block) = self.blocks.get(index).filter(|_| self.clicks) else { return };
        let event = json!({
            "name": block.name,
            "instance": block.instance,
            "button": click.button,
            "x": click.position.0.round() as i32,
            "y": click.position.1.round() as i32,
            "relative_x": click.relative.0.round() as i32,
            "relative_y": click.relative.1.round() as i32,
            "width": click.size.0.round() as i32,
            "height": click.size.1.round() as i32,
            "modifiers": [],
        });
        let comma = if std::mem::replace(&mut self.sent, true) { "," } else { "" };
        let _ = writeln!(self.stdin, "{comma}{event}");
    }
}
