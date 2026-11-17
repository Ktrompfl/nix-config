use resvg::{tiny_skia, usvg};
use smithay_client_toolkit::reexports::calloop::channel::Sender;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, mpsc},
    thread,
    time::Duration,
};
use zbus::{
    MatchRule,
    blocking::{Connection, MessageIterator, Proxy, connection, fdo::DBusProxy, proxy::Builder},
    fdo::RequestNameFlags,
    interface,
    message::{Header, Type},
    names::BusName,
    object_server::SignalEmitter,
    proxy::CacheProperties,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};

const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
const ITEM: &str = "org.kde.StatusNotifierItem";
const MENU: &str = "com.canonical.dbusmenu";
const ICON: u32 = 32;

#[derive(Clone)]
pub struct Item {
    pub id: String,
    owner: String,
    path: String,
    menu: Option<String>,
    pub is_menu: bool,
    pub passive: bool,
    pub icon: Option<Icon>,
    pub label: String,
}

#[derive(Clone)]
pub struct Icon {
    pub width: i32,
    pub height: i32,
    pub data: Arc<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub separator: bool,
    pub toggle: Option<(bool, bool)>,
    pub submenu: bool,
    pub children: Vec<Entry>,
}

pub enum Event {
    Items(Vec<Item>),
    Menu(String, Vec<Entry>),
    Submenu(String, i32, Vec<Entry>),
}

type Job = Box<dyn FnOnce(&Connection, &Sender<Event>) -> zbus::Result<()> + Send>;
type Props = HashMap<String, OwnedValue>;
type Cache = HashMap<(String, String), Option<Icon>>;

pub struct Tray {
    jobs: mpsc::Sender<Job>,
}

struct Watcher {
    items: Arc<Mutex<Vec<String>>>,
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    async fn register_status_notifier_item(
        &self,
        service: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
        let id = match service.starts_with('/') {
            true => format!("{sender}{service}"),
            false => service.to_string(),
        };
        let added = {
            let mut items = self.items.lock().unwrap();
            let added = !items.contains(&id);
            if added {
                items.push(id.clone());
            }
            added
        };
        if added {
            let _ = Self::status_notifier_item_registered(&emitter, &id).await;
        }
    }

    async fn register_status_notifier_host(&self, _service: &str) {}

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.lock().unwrap().clone()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(emitter: &SignalEmitter<'_>, service: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(emitter: &SignalEmitter<'_>, service: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

fn session() -> zbus::Result<Connection> {
    connection::Builder::session()?.method_timeout(Duration::from_secs(2)).build()
}

fn proxy<'a>(conn: &Connection, destination: &'a str, path: &'a str, interface: &'a str) -> zbus::Result<Proxy<'a>> {
    Builder::new(conn)
        .destination(destination)?
        .path(path)?
        .interface(interface)?
        .cache_properties(CacheProperties::No)
        .build()
}

fn split(id: &str) -> (&str, &str) {
    match id.find('/') {
        Some(at) => (&id[..at], &id[at..]),
        None => (id, "/StatusNotifierItem"),
    }
}

fn serve(conn: &Connection, items: &Arc<Mutex<Vec<String>>>) -> bool {
    let server = conn.object_server();
    if server.at(WATCHER_PATH, Watcher { items: items.clone() }).is_err() {
        return false;
    }
    let owned = conn.request_name_with_flags(WATCHER, RequestNameFlags::DoNotQueue.into()).is_ok();
    if !owned {
        let _ = server.remove::<Watcher, _>(WATCHER_PATH);
    }
    owned
}

fn host(conn: &Connection) -> zbus::Result<Vec<String>> {
    let host = format!("org.kde.StatusNotifierHost-{}", std::process::id());
    let watcher = proxy(conn, WATCHER, WATCHER_PATH, WATCHER)?;
    watcher.call_method("RegisterStatusNotifierHost", &host)?;
    watcher.get_property("RegisteredStatusNotifierItems")
}

fn fetch(conn: &Connection, item: &Item, path: &str, id: i32) -> zbus::Result<Vec<Entry>> {
    let menu = proxy(conn, &item.owner, path, MENU)?;
    let _ = menu.call_method("AboutToShow", &id);
    let (_, (_, _, children)): (u32, Layout) = menu.call("GetLayout", &(id, -1i32, Vec::<String>::new()))?;
    Ok(entries(children))
}

fn open(conn: &Connection, sender: &Sender<Event>, item: &Item) -> zbus::Result<()> {
    let Some(path) = &item.menu else { return Ok(()) };
    let _ = sender.send(Event::Menu(item.id.clone(), fetch(conn, item, path, 0)?));
    Ok(())
}

impl Tray {
    pub fn start(sender: Sender<Event>) -> zbus::Result<Self> {
        let (conn, signals) = (session()?, session()?);
        let items = Arc::new(Mutex::new(vec![]));
        let watcher = serve(&conn, &items);
        let _ = conn.request_name(format!("org.kde.StatusNotifierHost-{}", std::process::id()).as_str());
        let (c, s) = (conn.clone(), sender.clone());
        thread::spawn(move || {
            if let Err(e) = host_loop(signals, c, s, items, watcher) {
                log::warn!("tray stopped: {e}");
            }
        });
        let (jobs, queue) = mpsc::channel::<Job>();
        thread::spawn(move || {
            for job in queue {
                if let Err(e) = job(&conn, &sender) {
                    log::debug!("tray call failed: {e}");
                }
            }
        });
        Ok(Self { jobs })
    }

    fn spawn(&self, job: impl FnOnce(&Connection, &Sender<Event>) -> zbus::Result<()> + Send + 'static) {
        let _ = self.jobs.send(Box::new(job));
    }

    pub fn activate(&self, item: &Item, method: &'static str, (x, y): (i32, i32)) {
        let item = item.clone();
        self.spawn(move |conn, sender| {
            match proxy(conn, &item.owner, &item.path, ITEM)?.call_method(method, &(x, y)) {
                Err(_) if method == "Activate" => open(conn, sender, &item),
                result => result.map(drop),
            }
        });
    }

    pub fn scroll(&self, item: &Item, delta: i32, orientation: &'static str) {
        let item = item.clone();
        self.spawn(move |conn, _| {
            proxy(conn, &item.owner, &item.path, ITEM)?.call_method("Scroll", &(delta, orientation)).map(drop)
        });
    }

    pub fn menu(&self, item: &Item) -> bool {
        if item.menu.is_none() {
            return false;
        }
        let item = item.clone();
        self.spawn(move |conn, sender| open(conn, sender, &item));
        true
    }

    pub fn submenu(&self, item: &Item, id: i32) {
        let Some(path) = item.menu.clone() else { return };
        let item = item.clone();
        self.spawn(move |conn, sender| {
            let _ = sender.send(Event::Submenu(item.id.clone(), id, fetch(conn, &item, &path, id)?));
            Ok(())
        });
    }

    pub fn click(&self, item: &Item, entry: i32) {
        let Some(path) = item.menu.clone() else { return };
        let item = item.clone();
        self.spawn(move |conn, _| {
            let event = (entry, "clicked", Value::from(0i32), 0u32);
            proxy(conn, &item.owner, &path, MENU)?.call_method("Event", &event).map(drop)
        });
    }
}

type Layout = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

fn entries(children: Vec<OwnedValue>) -> Vec<Entry> {
    children.into_iter().filter_map(|child| Layout::try_from(child).ok()).filter_map(entry).collect()
}

fn entry((id, mut props, children): Layout) -> Option<Entry> {
    let mut take = |key: &str| props.remove(key);
    let string = |v: Option<OwnedValue>| v.and_then(|v| String::try_from(v).ok());
    let flag = |v: Option<OwnedValue>| v.and_then(|v| bool::try_from(v).ok());
    if flag(take("visible")) == Some(false) {
        return None;
    }
    let label = string(take("label")).unwrap_or_default();
    let toggle = match string(take("toggle-type")).as_deref() {
        Some(kind @ ("checkmark" | "radio")) => {
            let on = take("toggle-state").and_then(|v| i32::try_from(v).ok()) == Some(1);
            Some((kind == "radio", on))
        }
        _ => None,
    };
    let children = entries(children);
    Some(Entry {
        id,
        label: mnemonic(&label),
        enabled: flag(take("enabled")).unwrap_or(true),
        separator: string(take("type")).as_deref() == Some("separator"),
        toggle,
        submenu: !children.is_empty() || string(take("children-display")).as_deref() == Some("submenu"),
        children,
    })
}

fn mnemonic(label: &str) -> String {
    let mut out = String::new();
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '_' if chars.peek() == Some(&'_') => out.push(chars.next().unwrap_or('_')),
            '_' => {}
            c => out.push(c),
        }
    }
    out
}

fn host_loop(
    signals: Connection,
    conn: Connection,
    sender: Sender<Event>,
    registered: Arc<Mutex<Vec<String>>>,
    mut watcher: bool,
) -> zbus::Result<()> {
    let iterator = MessageIterator::from(&signals);
    let bus = DBusProxy::new(&signals)?;
    for (interface, member) in [(ITEM, None), (WATCHER, None), ("org.freedesktop.DBus", Some("NameOwnerChanged"))] {
        let rule = MatchRule::builder().msg_type(Type::Signal).interface(interface)?;
        let rule = match member {
            Some(member) => rule.member(member)?,
            None => rule,
        };
        bus.add_match_rule(rule.build())?;
    }
    let dbus = DBusProxy::new(&conn)?;
    let me = conn.unique_name().map(|n| n.to_string()).unwrap_or_default();
    let mut cache = Cache::new();
    let initial = match watcher {
        true => registered.lock().unwrap().clone(),
        false => host(&conn).unwrap_or_default(),
    };
    let mut items: Vec<Item> = initial.iter().filter_map(|id| load(&conn, &dbus, id, None, &mut cache)).collect();
    let _ = sender.send(Event::Items(items.clone()));
    for message in iterator {
        let message = message?;
        let header = message.header();
        if header.message_type() != Type::Signal {
            continue;
        }
        let interface = header.interface().map(|i| i.as_str().to_string()).unwrap_or_default();
        let member = header.member().map(|m| m.as_str().to_string()).unwrap_or_default();
        match (interface.as_str(), member.as_str()) {
            (WATCHER, "StatusNotifierItemRegistered") => {
                let id: String = message.body().deserialize()?;
                items.retain(|i| i.id != id);
                items.extend(load(&conn, &dbus, &id, None, &mut cache));
            }
            (WATCHER, "StatusNotifierItemUnregistered") => {
                let id: String = message.body().deserialize()?;
                items.retain(|i| i.id != id);
            }
            (ITEM, member) => {
                let icon = matches!(member, "NewIcon" | "NewAttentionIcon" | "NewStatus" | "NewIconThemePath");
                let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
                let path = header.path().map(|p| p.to_string()).unwrap_or_default();
                for item in items.iter_mut().filter(|i| i.owner == sender && i.path == path) {
                    if let Some(fresh) = load(&conn, &dbus, &item.id, (!icon).then_some(&*item), &mut cache) {
                        *item = fresh;
                    }
                }
            }
            (_, "NameOwnerChanged") => {
                let (name, _, new): (String, String, String) = message.body().deserialize()?;
                if name == WATCHER && !watcher && new.is_empty() && serve(&conn, &registered) {
                    log::info!("tray: took over {WATCHER}");
                    (watcher, items) = (true, vec![]);
                } else if name == WATCHER && !watcher && !new.is_empty() && new != me {
                    let ids = host(&conn).unwrap_or_default();
                    items = ids.iter().filter_map(|id| load(&conn, &dbus, id, None, &mut cache)).collect();
                } else if !new.is_empty() {
                    continue;
                }
                let gone: Vec<String> =
                    items.iter().filter(|i| i.owner == name || split(&i.id).0 == name).map(|i| i.id.clone()).collect();
                items.retain(|i| !gone.contains(&i.id));
                if watcher {
                    registered.lock().unwrap().retain(|id| !gone.contains(id) && split(id).0 != name);
                    let server = conn.object_server();
                    if let Ok(iface) = server.interface::<_, Watcher>(WATCHER_PATH) {
                        for id in &gone {
                            let _ =
                                zbus::block_on(Watcher::status_notifier_item_unregistered(iface.signal_emitter(), id));
                        }
                    }
                }
            }
            _ => continue,
        }
        let _ = sender.send(Event::Items(items.clone()));
    }
    Ok(())
}

fn get<T: TryFrom<OwnedValue>>(props: &mut Props, key: &str) -> Option<T> {
    props.remove(key)?.try_into().ok()
}

fn load(conn: &Connection, dbus: &DBusProxy, id: &str, keep: Option<&Item>, cache: &mut Cache) -> Option<Item> {
    let (service, path) = split(id);
    let owner = match service.starts_with(':') {
        true => service.to_string(),
        false => dbus.get_name_owner(BusName::try_from(service).ok()?).ok()?.to_string(),
    };
    let mut props: Props =
        proxy(conn, &owner, path, "org.freedesktop.DBus.Properties").ok()?.call("GetAll", &ITEM).ok()?;
    let status: String = get(&mut props, "Status").unwrap_or_default();
    let menu = get::<OwnedObjectPath>(&mut props, "Menu").map(|p| p.to_string()).filter(|p| p != "/");
    let is_menu = get(&mut props, "ItemIsMenu").unwrap_or(false);
    let icon = match keep {
        Some(item) => item.icon.clone(),
        None => icon(&mut props, status == "NeedsAttention", cache),
    };
    let title: String = get(&mut props, "Title").unwrap_or_default();
    let label = if title.is_empty() { get(&mut props, "Id").unwrap_or_default() } else { title };
    let passive = status == "Passive";
    Some(Item { id: id.to_string(), owner, path: path.to_string(), menu, is_menu, passive, icon, label })
}

fn icon(props: &mut Props, attention: bool, cache: &mut Cache) -> Option<Icon> {
    let extra: String = get(props, "IconThemePath").unwrap_or_default();
    let mut find = |name: &str, pixmap: &str| {
        let name: String = get(props, name).unwrap_or_default();
        let themed = (!name.is_empty()).then(|| {
            if cache.len() > 256 {
                cache.clear();
            }
            let key = (name.clone(), extra.clone());
            let lookup = || themed(&name, &extra).or_else(|| themed(&format!("{name}-symbolic"), &extra));
            cache.entry(key).or_insert_with(lookup).clone()
        });
        themed.flatten().or_else(|| pixmap_icon(get(props, pixmap)?))
    };
    let attention = attention.then(|| find("AttentionIconName", "AttentionIconPixmap")).flatten();
    attention.or_else(|| find("IconName", "IconPixmap"))
}

fn pixmap_icon(pixmaps: Vec<(i32, i32, Vec<u8>)>) -> Option<Icon> {
    let fits = |p: &&(i32, i32, Vec<u8>)| p.0 > 0 && p.1 > 0 && p.2.len() == (p.0 * p.1 * 4) as usize;
    let (width, height, argb) = pixmaps.iter().filter(fits).min_by_key(|p| (p.0 - ICON as i32).abs())?;
    let data = argb
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let a = u32::from(p[0]);
            let m = |c: u8| ((u32::from(c) * a + 127) / 255) as u8;
            [m(p[3]), m(p[2]), m(p[1]), p[0]]
        })
        .collect();
    Some(Icon { width: *width, height: *height, data: Arc::new(data) })
}

fn themed(name: &str, extra: &str) -> Option<Icon> {
    if name.starts_with('/') {
        return file(Path::new(name));
    }
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(env::var_os("HOME")?).join(".local/share")));
    let data_dirs = env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    let mut bases: Vec<PathBuf> = vec![];
    bases.extend((!extra.is_empty()).then(|| PathBuf::from(extra)));
    bases.extend(data_home.iter().map(|d| d.join("icons")));
    bases.extend(data_dirs.split(':').filter(|d| !d.is_empty()).map(|d| Path::new(d).join("icons")));
    let themes = [theme_name(), Some("hicolor"), Some("Adwaita")];
    let sizes = ["scalable", "32x32", "48x48", "64x64", "24x24", "22x22", "128x128", "256x256", "16x16", "symbolic"];
    let kinds =
        ["apps", "status", "devices", "panel", "actions", "places", "categories", "emblems", "legacy", "mimetypes"];
    for base in &bases {
        for ext in ["svg", "png"] {
            if let Some(icon) = file(&base.join(format!("{name}.{ext}"))) {
                return Some(icon);
            }
        }
        for theme in themes.iter().flatten() {
            for size in sizes {
                for kind in kinds {
                    for ext in ["svg", "png"] {
                        if let Some(icon) = file(&base.join(theme).join(size).join(kind).join(format!("{name}.{ext}")))
                        {
                            return Some(icon);
                        }
                    }
                }
            }
        }
    }
    let pixmaps = data_dirs.split(':').map(|d| Path::new(d).join("pixmaps").join(format!("{name}.png")));
    pixmaps.into_iter().find_map(|p| file(&p))
}

fn theme_name() -> Option<&'static str> {
    static THEME: OnceLock<Option<String>> = OnceLock::new();
    let theme = THEME.get_or_init(|| {
        let config = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| Some(PathBuf::from(env::var_os("HOME")?).join(".config")))?;
        let text = fs::read_to_string(config.join("gtk-3.0/settings.ini")).ok()?;
        let line = text.lines().find_map(|l| l.trim().strip_prefix("gtk-icon-theme-name"))?;
        Some(line.trim_start_matches([' ', '=']).trim().trim_matches('"').to_string())
    });
    theme.as_deref()
}

fn file(path: &Path) -> Option<Icon> {
    match path.extension()?.to_str()? {
        "png" => png(path),
        "svg" => svg(path),
        _ => None,
    }
}

fn png(path: &Path) -> Option<Icon> {
    let mut file = fs::File::open(path).ok()?;
    let mut image = cairo::ImageSurface::create_from_png(&mut file).ok()?;
    if !matches!(image.format(), cairo::Format::ARgb32 | cairo::Format::Rgb24) {
        let converted = cairo::ImageSurface::create(cairo::Format::ARgb32, image.width(), image.height()).ok()?;
        let cr = cairo::Context::new(&converted).ok()?;
        cr.set_source_surface(&image, 0.0, 0.0).ok()?;
        cr.paint().ok()?;
        drop(cr);
        image = converted;
    }
    image.flush();
    let (width, height, stride) = (image.width(), image.height(), image.stride() as usize);
    let data = image.data().ok()?;
    let rows = data.chunks(stride).take(height as usize);
    let pixels = rows.flat_map(|row| &row[..width as usize * 4]).copied().collect();
    Some(Icon { width, height, data: Arc::new(pixels) })
}

fn svg(path: &Path) -> Option<Icon> {
    let tree = usvg::Tree::from_data(&fs::read(path).ok()?, &usvg::Options::default()).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(ICON, ICON)?;
    let size = tree.size();
    let scale = ICON as f32 / size.width().max(size.height());
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let symbolic = path.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.ends_with("-symbolic"));
    let tint = |c: u8, a: u8| (u32::from(c) * u32::from(a) / 255) as u8;
    let pixels = pixmap.data().as_chunks::<4>().0.iter();
    let data = match symbolic {
        true => pixels.flat_map(|p| [tint(0x84, p[3]), tint(0x99, p[3]), tint(0xa8, p[3]), p[3]]).collect(),
        false => pixels.flat_map(|p| [p[2], p[1], p[0], p[3]]).collect(),
    };
    Some(Icon { width: ICON as i32, height: ICON as i32, data: Arc::new(data) })
}
