use backend::OutputId;
use std::{
    collections::BTreeMap,
    env,
    fmt::Write as _,
    fs::{self, File},
    io::{BufWriter, Write},
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct Profile {
    stats: Option<Stats>,
    trace: Option<Trace>,
    unpresented: Option<Instant>,
    latched: BTreeMap<OutputId, Instant>,
}

struct Stats {
    interval: Duration,
    since: Instant,
    cpu: Duration,
    wakeups: u64,
    commits: u64,
    outputs: BTreeMap<OutputId, Frames>,
}

#[derive(Default)]
struct Frames {
    name: String,
    cpu: Samples,
    gpu: Samples,
    latency: Samples,
    scanout: u32,
}

#[derive(Default)]
struct Samples {
    count: u32,
    sum: Duration,
    max: Duration,
}

impl Samples {
    fn add(&mut self, d: Duration) {
        (self.count, self.sum, self.max) = (self.count + 1, self.sum + d, self.max.max(d));
    }

    fn show(&self) -> String {
        let ms = |d: Duration| d.as_secs_f64() * 1e3;
        match self.count {
            0 => "-".into(),
            n => format!("{:.2}/{:.2}ms", ms(self.sum / n), ms(self.max)),
        }
    }
}

struct Trace {
    out: BufWriter<File>,
    start: Instant,
    flushed: Instant,
    named: Vec<u32>,
}

impl Trace {
    fn name(&mut self, track: u32, name: &str) {
        if !self.named.contains(&track) {
            self.named.push(track);
            let args = format!(r#"{{"name":"{name}"}}"#);
            let _ = writeln!(self.out, r#"{{"name":"thread_name","ph":"M","pid":1,"tid":{track},"args":{args}}},"#);
        }
    }
}

impl Drop for Trace {
    fn drop(&mut self) {
        let _ = writeln!(self.out, r#"{{"name":"process_name","ph":"M","pid":1,"args":{{"name":"getaway"}}}}]"#);
    }
}

pub struct Counts {
    pub clients: usize,
    pub surfaces: usize,
    pub textures: usize,
    pub queued: usize,
}

impl Profile {
    pub fn from_env() -> Self {
        let stats = env::var("GETAWAY_STATS").ok().and_then(|s| s.parse::<f64>().ok()).map(|secs| Stats {
            interval: Duration::from_secs_f64(secs.max(0.1)),
            since: Instant::now(),
            cpu: cpu_time(),
            wakeups: 0,
            commits: 0,
            outputs: BTreeMap::new(),
        });
        let trace = env::var_os("GETAWAY_TRACE").and_then(|path| match File::create(&path) {
            Ok(file) => {
                let mut out = BufWriter::new(file);
                let _ = out.write_all(b"[\n");
                let mut trace = Trace { out, start: Instant::now(), flushed: Instant::now(), named: vec![] };
                trace.name(0, "event loop");
                Some(trace)
            }
            Err(e) => {
                log::warn!("cannot create trace {}: {e}", path.display());
                None
            }
        });
        Self { stats, trace, ..Self::default() }
    }

    pub fn span(&mut self, name: &str, track: u32, start: Instant) {
        let Some(t) = &mut self.trace else { return };
        let (ts, dur) = (start.duration_since(t.start).as_micros(), start.elapsed().as_micros());
        let _ = writeln!(t.out, r#"{{"name":"{name}","ph":"X","ts":{ts},"dur":{dur},"pid":1,"tid":{track}}},"#);
    }

    fn instant(&mut self, name: &str, track: u32) {
        let Some(t) = &mut self.trace else { return };
        let ts = t.start.elapsed().as_micros();
        let _ = writeln!(t.out, r#"{{"name":"{name}","ph":"i","s":"t","ts":{ts},"pid":1,"tid":{track}}},"#);
    }

    pub fn wakeup(&mut self) {
        self.stats.iter_mut().for_each(|s| s.wakeups += 1);
    }

    pub fn commit(&mut self) {
        self.unpresented.get_or_insert_with(Instant::now);
        self.stats.iter_mut().for_each(|s| s.commits += 1);
        self.instant("commit", 0);
    }

    pub fn rendered(&mut self, output: OutputId, name: &str, start: Instant, scanout: bool) {
        let cpu = start.elapsed();
        let latched = self.unpresented.unwrap_or(start);
        self.latched.insert(output, latched);
        if let Some(t) = &mut self.trace {
            t.name(100 + output, &format!("render {name}"));
            t.name(200 + output, &format!("gpu {name}"));
        }
        self.span("render", 100 + output, start);
        if let Some(stats) = &mut self.stats {
            let frames = stats.outputs.entry(output).or_default();
            if frames.name != name {
                frames.name = name.to_string();
            }
            frames.cpu.add(cpu);
            frames.scanout += u32::from(scanout);
        }
    }

    pub fn gpu(&mut self, output: OutputId, start: Instant, gpu: Duration) {
        if let Some(t) = &mut self.trace {
            let (ts, dur) = (start.duration_since(t.start).as_micros(), gpu.as_micros());
            let _ =
                writeln!(t.out, r#"{{"name":"gpu","ph":"X","ts":{ts},"dur":{dur},"pid":1,"tid":{}}},"#, 200 + output);
        }
        if let Some(stats) = &mut self.stats {
            stats.outputs.entry(output).or_default().gpu.add(gpu);
        }
    }

    pub fn all_latched(&mut self) {
        self.unpresented = None;
    }

    pub fn presented(&mut self, output: OutputId) {
        let Some(latched) = self.latched.remove(&output) else { return };
        self.instant("present", 100 + output);
        if let Some(frames) = self.stats.as_mut().map(|s| s.outputs.entry(output).or_default()) {
            frames.latency.add(latched.elapsed());
        }
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.stats.as_ref().map(|s| s.since + s.interval)
    }

    pub fn tick(&mut self, counts: Counts) {
        if let Some(t) = self.trace.as_mut().filter(|t| t.flushed.elapsed() > Duration::from_millis(500)) {
            let _ = t.out.flush();
            t.flushed = Instant::now();
        }
        let Some(stats) = self.stats.as_mut().filter(|s| s.since.elapsed() >= s.interval) else { return };
        let (elapsed, cpu) = (stats.since.elapsed().as_secs_f64(), cpu_time());
        let rate = |n: u64| n as f64 / elapsed;
        let Counts { clients, surfaces, textures, queued } = counts;
        let mut line = format!(
            "cpu {:.1}% rss {:.1}MiB wakeups {:.0}/s commits {:.0}/s clients {clients} surfaces {surfaces} \
             textures {textures} queued {queued}",
            (cpu - stats.cpu).as_secs_f64() / elapsed * 100.0,
            rss() as f64 / f64::from(1 << 20),
            rate(stats.wakeups),
            rate(stats.commits),
        );
        for f in stats.outputs.values() {
            let fps = f64::from(f.cpu.count) / elapsed;
            let (cpu, gpu, latency) = (f.cpu.show(), f.gpu.show(), f.latency.show());
            let _ =
                write!(line, " | {} {fps:.1}fps scanout {} cpu {cpu} gpu {gpu} latency {latency}", f.name, f.scanout);
        }
        log::info!("stats: {line}");
        *stats = Stats { since: Instant::now(), cpu, wakeups: 0, commits: 0, outputs: BTreeMap::new(), ..*stats };
    }
}

fn cpu_time() -> Duration {
    let stat = fs::read_to_string("/proc/self/stat").unwrap_or_default();
    let fields: Vec<&str> = stat.rsplit_once(") ").map_or(vec![], |(_, rest)| rest.split(' ').collect());
    let ticks: u64 = [11, 12].iter().filter_map(|&i| fields.get(i)?.parse::<u64>().ok()).sum();
    Duration::from_secs_f64(ticks as f64 / rustix::param::clock_ticks_per_second() as f64)
}

fn rss() -> u64 {
    let statm = fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: u64 = statm.split(' ').nth(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    pages * rustix::param::page_size() as u64
}
