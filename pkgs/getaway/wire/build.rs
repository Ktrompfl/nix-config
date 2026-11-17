use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use std::{env, fmt::Write, fs, path::Path};

#[derive(Default)]
struct Interface {
    name: String,
    version: u32,
    requests: Vec<Message>,
    events: Vec<Message>,
    enums: Vec<Enum>,
}

struct Message {
    name: String,
    since: u32,
    destructor: bool,
    args: Vec<Arg>,
}

struct Arg {
    name: String,
    ty: String,
    interface: Option<String>,
    nullable: bool,
}

struct Enum {
    name: String,
    entries: Vec<(String, String)>,
}

const PROTOCOLS: &[&str] = &[
    "wayland/wayland.xml",
    "wayland-protocols/stable/xdg-shell/xdg-shell.xml",
    "wayland-protocols/stable/viewporter/viewporter.xml",
    "wayland-protocols/stable/presentation-time/presentation-time.xml",
    "wayland-protocols/stable/linux-dmabuf/linux-dmabuf-v1.xml",
    "wayland-protocols/staging/alpha-modifier/alpha-modifier-v1.xml",
    "wayland-protocols/staging/commit-timing/commit-timing-v1.xml",
    "wayland-protocols/staging/content-type/content-type-v1.xml",
    "wayland-protocols/staging/cursor-shape/cursor-shape-v1.xml",
    "wayland-protocols/staging/ext-data-control/ext-data-control-v1.xml",
    "wayland-protocols/staging/ext-foreign-toplevel-list/ext-foreign-toplevel-list-v1.xml",
    "wayland-protocols/staging/ext-idle-notify/ext-idle-notify-v1.xml",
    "wayland-protocols/staging/ext-image-capture-source/ext-image-capture-source-v1.xml",
    "wayland-protocols/staging/ext-image-copy-capture/ext-image-copy-capture-v1.xml",
    "wayland-protocols/staging/ext-session-lock/ext-session-lock-v1.xml",
    "wayland-protocols/staging/ext-workspace/ext-workspace-v1.xml",
    "wayland-protocols/staging/fifo/fifo-v1.xml",
    "wayland-protocols/staging/fractional-scale/fractional-scale-v1.xml",
    "wayland-protocols/staging/linux-drm-syncobj/linux-drm-syncobj-v1.xml",
    "wayland-protocols/staging/pointer-warp/pointer-warp-v1.xml",
    "wayland-protocols/staging/single-pixel-buffer/single-pixel-buffer-v1.xml",
    "wayland-protocols/staging/tearing-control/tearing-control-v1.xml",
    "wayland-protocols/staging/xdg-activation/xdg-activation-v1.xml",
    "wayland-protocols/staging/xdg-dialog/xdg-dialog-v1.xml",
    "wayland-protocols/staging/xdg-system-bell/xdg-system-bell-v1.xml",
    "wayland-protocols/staging/xdg-toplevel-drag/xdg-toplevel-drag-v1.xml",
    "wayland-protocols/staging/xdg-toplevel-icon/xdg-toplevel-icon-v1.xml",
    "wayland-protocols/staging/xdg-toplevel-tag/xdg-toplevel-tag-v1.xml",
    "wayland-protocols/unstable/idle-inhibit/idle-inhibit-unstable-v1.xml",
    "wayland-protocols/unstable/pointer-constraints/pointer-constraints-unstable-v1.xml",
    "wayland-protocols/unstable/pointer-gestures/pointer-gestures-unstable-v1.xml",
    "wayland-protocols/unstable/primary-selection/primary-selection-unstable-v1.xml",
    "wayland-protocols/unstable/relative-pointer/relative-pointer-unstable-v1.xml",
    "wayland-protocols/unstable/xdg-decoration/xdg-decoration-unstable-v1.xml",
    "wayland-protocols/unstable/xdg-output/xdg-output-unstable-v1.xml",
    "wlr-protocols/unstable/wlr-data-control-unstable-v1.xml",
    "wlr-protocols/unstable/wlr-foreign-toplevel-management-unstable-v1.xml",
    "wlr-protocols/unstable/wlr-gamma-control-unstable-v1.xml",
    "wlr-protocols/unstable/wlr-layer-shell-unstable-v1.xml",
    "wlr-protocols/unstable/wlr-output-management-unstable-v1.xml",
    "wlr-protocols/unstable/wlr-output-power-management-unstable-v1.xml",
    "wlr-protocols/unstable/wlr-virtual-pointer-unstable-v1.xml",
    "plasma-wayland-protocols/server-decoration.xml",
    "wlroots/virtual-keyboard-unstable-v1.xml",
];

const UNSERVED: &[&str] = &["wl_shell", "wl_shell_surface"];

fn main() {
    println!("cargo:rerun-if-env-changed=PROTOCOL_DIR");
    let dir = env::var("PROTOCOL_DIR").expect("PROTOCOL_DIR is set by the flake devshell");
    let mut interfaces = vec![];
    for protocol in PROTOCOLS {
        let path = Path::new(&dir).join(protocol);
        let xml = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        interfaces.extend(parse(&xml).into_iter().filter(|i| !UNSERVED.contains(&i.name.as_str())));
    }
    let mut code = String::new();
    interfaces.iter().for_each(|i| emit(&mut code, i));
    emit_dispatch(&mut code, &interfaces);
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("protocols.rs"), code).unwrap();
}

fn attr(e: &BytesStart, name: &str) -> Option<String> {
    let value = e.try_get_attribute(name).unwrap()?;
    Some(value.unescape_value().unwrap().into_owned())
}

fn parse(xml: &str) -> Vec<Interface> {
    let mut reader = Reader::from_str(xml);
    let mut interfaces: Vec<Interface> = vec![];
    let mut in_event = false;
    loop {
        let e = match reader.read_event().unwrap() {
            Event::Start(e) | Event::Empty(e) => e,
            Event::Eof => return interfaces,
            _ => continue,
        };
        let current = interfaces.last_mut();
        match e.name().as_ref() {
            b"interface" => interfaces.push(Interface {
                name: attr(&e, "name").unwrap(),
                version: attr(&e, "version").unwrap().parse().unwrap(),
                ..Default::default()
            }),
            tag @ (b"request" | b"event") => {
                in_event = tag == b"event";
                let message = Message {
                    name: attr(&e, "name").unwrap(),
                    since: attr(&e, "since").map_or(1, |s| s.parse().unwrap()),
                    destructor: attr(&e, "type").as_deref() == Some("destructor"),
                    args: vec![],
                };
                let current = current.unwrap();
                match in_event {
                    true => current.events.push(message),
                    false => current.requests.push(message),
                }
            }
            b"arg" => {
                let current = current.unwrap();
                let messages = if in_event { &mut current.events } else { &mut current.requests };
                messages.last_mut().unwrap().args.push(Arg {
                    name: attr(&e, "name").unwrap(),
                    ty: attr(&e, "type").unwrap(),
                    interface: attr(&e, "interface"),
                    nullable: attr(&e, "allow-null").as_deref() == Some("true"),
                });
            }
            b"enum" => current.unwrap().enums.push(Enum { name: attr(&e, "name").unwrap(), entries: vec![] }),
            b"entry" => {
                let entry = (attr(&e, "name").unwrap(), attr(&e, "value").unwrap());
                current.unwrap().enums.last_mut().unwrap().entries.push(entry);
            }
            _ => {}
        }
    }
}

fn emit(code: &mut String, i: &Interface) {
    let name = &i.name;
    writeln!(code, "pub mod {name} {{\n#![allow(clippy::all, unused)]\nuse crate::*;").unwrap();
    writeln!(code, "pub const NAME: &str = \"{name}\";\npub const VERSION: u32 = {};", i.version).unwrap();
    emit_requests(code, &i.requests);
    for (opcode, event) in i.events.iter().enumerate() {
        emit_event(code, opcode, event);
    }
    for e in &i.enums {
        writeln!(code, "pub mod {} {{", ident(&e.name)).unwrap();
        for (entry, value) in &e.entries {
            let prefix = if entry.starts_with(|c: char| c.is_ascii_digit()) { "_" } else { "" };
            writeln!(code, "pub const {prefix}{}: u32 = {value};", entry.to_uppercase()).unwrap();
        }
        code.push_str("}\n");
    }
    code.push_str("}\n");
}

fn emit_requests(code: &mut String, requests: &[Message]) {
    code.push_str("pub trait Handler {\n");
    for r in requests {
        let name = ident(&r.name);
        if r.destructor {
            let params: String = r.args.iter().map(|a| format!(", _: {}", request_arg(a).0)).collect();
            writeln!(code, "fn {name}(&mut self, _: Cx{params}) -> impl Reply {{}}").unwrap();
            continue;
        }
        let params: String = r.args.iter().map(|a| format!(", {}: {}", ident(&a.name), request_arg(a).0)).collect();
        writeln!(code, "fn {name}(&mut self, cx: Cx{params}) -> impl Reply;").unwrap();
    }
    code.push_str(
        "}\npub fn dispatch<H: Handler + Server>(h_: &mut H, cx_: Cx, m_: &mut Msg) -> Result<bool, Error> {\n",
    );
    let since: Vec<_> = requests.iter().map(|r| r.since).collect();
    writeln!(code, "let since_: [u32; {}] = {since:?};", since.len()).unwrap();
    code.push_str("let bad_ = |e: Malformed| Error::malformed(cx_.obj.id, NAME, e);\n");
    code.push_str("if since_.get(m_.opcode as usize).is_none_or(|&s| cx_.obj.version < s) {\n");
    code.push_str("return Err(bad_(Malformed::Opcode));\n}\nlet a_ = &mut m_.args;\nmatch m_.opcode {\n");
    for (opcode, r) in requests.iter().enumerate() {
        writeln!(code, "{opcode} => {{").unwrap();
        let names: Vec<_> = r.args.iter().map(|a| ident(&a.name)).collect();
        for (a, name) in r.args.iter().zip(&names) {
            writeln!(code, "let {name} = a_.{}.map_err(bad_)?;", request_arg(a).1).unwrap();
        }
        for (a, name) in r.args.iter().zip(&names) {
            if let ("new_id", Some(interface)) = (a.ty.as_str(), &a.interface) {
                let interface = camel(interface);
                writeln!(
                    code,
                    "let {name} = h_.connection(cx_.client).add({name}, Interface::{interface}, cx_.obj.version, cx_.data)?;"
                )
                .unwrap();
            }
        }
        let shown = vec!["{:?}"; names.len()].join(", ");
        let values: String = names.iter().map(|n| format!(", {n}")).collect();
        writeln!(code, "log::trace!(\"{{}} -> {{}}@{{}}.{}({shown})\", cx_.client, NAME, cx_.obj.id{values});", r.name)
            .unwrap();
        writeln!(code, "h_.{}(cx_{values}).reply()?;\nOk({})\n}}", ident(&r.name), r.destructor).unwrap();
    }
    code.push_str("_ => unreachable!(),\n}\n}\n");
}

fn emit_dispatch(code: &mut String, interfaces: &[Interface]) {
    let variants: Vec<_> = interfaces.iter().map(|i| (camel(&i.name), &i.name)).collect();
    code.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum Interface {\n");
    variants.iter().for_each(|(v, _)| writeln!(code, "{v},").unwrap());
    code.push_str("}\nimpl Interface {\npub fn name(self) -> &'static str {\nmatch self {\n");
    variants.iter().for_each(|(v, m)| writeln!(code, "Self::{v} => {m}::NAME,").unwrap());
    code.push_str(
        "}\n}\n}\npub fn dispatch<H>(h: &mut H, client: u64, m: &mut Msg) -> Result<bool, Error>\nwhere\nH: Server,\n",
    );
    variants.iter().for_each(|(_, m)| writeln!(code, "H: {m}::Handler,").unwrap());
    code.push_str("{\nlet (interface, cx) = h.connection(client).lookup(client, m.id)?;\nmatch interface {\n");
    variants.iter().for_each(|(v, m)| writeln!(code, "Interface::{v} => {m}::dispatch(h, cx, m),").unwrap());
    code.push_str("}\n}\n");
}

fn emit_event(code: &mut String, opcode: usize, e: &Message) {
    let params: String = e.args.iter().map(|a| format!(", {}: {}", ident(&a.name), event_arg(a).0)).collect();
    writeln!(code, "pub fn {}(out_: &mut Out, obj_: Obj{params}) {{", ident(&e.name)).unwrap();
    writeln!(code, "if obj_.version < {} {{ return; }}\nlet m_ = out_.begin(obj_.id, {opcode});", e.since).unwrap();
    for a in &e.args {
        writeln!(code, "{};", event_arg(a).1).unwrap();
    }
    code.push_str("out_.end(m_);\n}\n");
}

fn request_arg(a: &Arg) -> (&'static str, &'static str) {
    match (a.ty.as_str(), a.nullable, a.interface.is_some()) {
        ("int", ..) => ("i32", "int()"),
        ("uint", ..) => ("u32", "uint()"),
        ("fixed", ..) => ("Fixed", "fixed()"),
        ("string", true, _) => ("Option<String>", "string_opt()"),
        ("string", false, _) => ("String", "string()"),
        ("object", true, _) => ("Option<u32>", "object_opt()"),
        ("object", false, _) => ("u32", "object()"),
        ("new_id", _, true) => ("Obj", "object()"),
        ("new_id", _, false) => ("NewId", "new_id()"),
        ("array", ..) => ("Vec<u8>", "array()"),
        ("fd", ..) => ("OwnedFd", "fd()"),
        (ty, ..) => panic!("unknown type {ty}"),
    }
}

fn event_arg(a: &Arg) -> (&'static str, String) {
    let name = ident(&a.name);
    let (ty, expr) = match (a.ty.as_str(), a.nullable) {
        ("int", _) => ("i32", format!("int({name})")),
        ("uint", _) | ("new_id", _) | ("object", false) => ("u32", format!("uint({name})")),
        ("object", true) => ("Option<u32>", format!("uint({name}.unwrap_or(0))")),
        ("fixed", _) => ("Fixed", format!("fixed({name})")),
        ("string", true) => ("Option<&str>", format!("string({name})")),
        ("string", false) => ("&str", format!("string(Some({name}))")),
        ("array", _) => ("&[u8]", format!("array({name})")),
        _ => ("BorrowedFd<'_>", format!("fd({name})")),
    };
    (ty, format!("out_.{expr}"))
}

fn camel(name: &str) -> String {
    name.split('_').map(|w| w[..1].to_uppercase() + &w[1..]).collect()
}

fn ident(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "as", "async", "await", "box", "const", "crate", "dyn", "enum", "fn", "gen", "impl", "in", "let", "loop",
        "match", "mod", "move", "ref", "static", "struct", "super", "trait", "try", "type", "use", "where",
    ];
    match name {
        "self" => "self_".into(),
        n if KEYWORDS.contains(&n) => format!("r#{n}"),
        n => n.into(),
    }
}
