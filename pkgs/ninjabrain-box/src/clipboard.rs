//! Watching the Wayland clipboard, and deciding what the bot gets to see.
//!
//! Reading a selection needs the data-control protocol, which every
//! compositor worth running has in one of its two spellings: the newer
//! `ext_data_control_v1` and wlroots' original `zwlr_data_control_v1`. They
//! are the same protocol with different names, so both are supported and the
//! newer one is preferred.

use std::collections::HashMap;
use std::io::Read;
use std::os::fd::{AsFd, OwnedFd};

use wayland_client::backend::ObjectId;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::{self, ExtDataControlDeviceV1},
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::{self, ExtDataControlOfferV1},
};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::{self, ZwlrDataControlDeviceV1},
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_offer_v1::{self, ZwlrDataControlOfferV1},
};

use crate::app::App;

/// Text flavours, best first.
const MIME_TYPES: &[&str] = &[
    "text/plain;charset=utf-8",
    "text/plain",
    "UTF8_STRING",
    "STRING",
    "TEXT",
];

/// The selection watcher, over whichever protocol the compositor offers.
pub struct Clipboard {
    device: Device,
    /// Mime types each live offer has advertised so far.
    offers: HashMap<ObjectId, Vec<String>>,
}

enum Device {
    Ext(ExtDataControlDeviceV1),
    Wlr(ZwlrDataControlDeviceV1),
}

impl Clipboard {
    /// Binds the watcher, or returns why it could not be.
    pub fn new(
        globals: &GlobalList,
        queue: &QueueHandle<App>,
        seat: &WlSeat,
    ) -> Result<Clipboard, String> {
        let device = if let Ok(manager) =
            globals.bind::<ExtDataControlManagerV1, _, _>(queue, 1..=1, ())
        {
            Device::Ext(manager.get_data_device(seat, queue, ()))
        } else if let Ok(manager) =
            globals.bind::<ZwlrDataControlManagerV1, _, _>(queue, 1..=2, ())
        {
            Device::Wlr(manager.get_data_device(seat, queue, ()))
        } else {
            return Err(
                "the compositor offers neither ext_data_control_v1 nor \
                 zwlr_data_control_v1, so measurements cannot be picked up"
                    .into(),
            );
        };
        Ok(Clipboard {
            device,
            offers: HashMap::new(),
        })
    }

    fn remember(&mut self, offer: ObjectId) {
        self.offers.insert(offer, Vec::new());
    }

    fn advertise(&mut self, offer: &ObjectId, mime_type: String) {
        if let Some(types) = self.offers.get_mut(offer) {
            types.push(mime_type);
        }
    }

    /// The best text flavour this offer has, if any.
    fn best_mime(&self, offer: &ObjectId) -> Option<&'static str> {
        let advertised = self.offers.get(offer)?;
        MIME_TYPES
            .iter()
            .find(|wanted| advertised.iter().any(|have| have == *wanted))
            .copied()
    }

    fn forget(&mut self, offer: &ObjectId) {
        self.offers.remove(offer);
    }
}

impl Device {
    fn destroy(&self) {
        match self {
            Device::Ext(device) => device.destroy(),
            Device::Wlr(device) => device.destroy(),
        }
    }
}

impl Drop for Clipboard {
    fn drop(&mut self) {
        self.device.destroy();
    }
}

/// Hands the selection to a reader thread.
///
/// The compositor writes the selection into a pipe we give it, so the write
/// end has to reach it -- which means flushing before our copy is dropped,
/// since the descriptor only travels when the request is actually sent.
fn receive(connection: &Connection, sink: &calloop::channel::Sender<String>, take: impl FnOnce(&OwnedFd)) {
    let Ok((read, write)) = rustix::pipe::pipe() else {
        return;
    };
    take(&write);
    let _ = connection.flush();
    drop(write);

    let sink = sink.clone();
    std::thread::spawn(move || {
        let mut file = std::fs::File::from(read);
        let mut text = String::new();
        // A selection is a line of coordinates; anything larger is not one.
        if file
            .by_ref()
            .take(64 * 1024)
            .read_to_string(&mut text)
            .is_ok()
        {
            let _ = sink.send(text);
        }
    });
}

/// Whether this is the line Minecraft's F3+C puts on the clipboard.
///
/// The bot will take several formats, but only one of them is what the game
/// produces, and the overlay owns the clipboard for the bot now: everything
/// that is not a measurement should stay out of it.
///
/// ```text
/// /execute in minecraft:overworld run tp @s 123.45 68.00 -456.78 -12.34 -31.00
/// ```
pub fn is_measurement(text: &str) -> bool {
    let text = text.trim();
    let mut token = text.split_whitespace();
    let head = [
        token.next(),
        token.next(),
        token.next(),
        token.next(),
        token.next(),
        token.next(),
    ];
    let [Some("/execute"), Some("in"), Some(dimension), Some("run"), Some("tp"), Some("@s")] = head
    else {
        return false;
    };
    if !matches!(
        dimension,
        "minecraft:overworld" | "minecraft:the_nether" | "minecraft:the_end"
    ) {
        return false;
    }
    // x, y, z, yaw, pitch, and nothing after them.
    let numbers: Vec<&str> = token.collect();
    numbers.len() == 5 && numbers.iter().all(|n| n.parse::<f64>().is_ok())
}

// --- ext_data_control_v1 ---

impl Dispatch<ExtDataControlManagerV1, ()> for App {
    fn event(
        _: &mut App,
        _: &ExtDataControlManagerV1,
        _: <ExtDataControlManagerV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
    }
}

impl Dispatch<ExtDataControlDeviceV1, ()> for App {
    fn event(
        app: &mut App,
        _: &ExtDataControlDeviceV1,
        event: ext_data_control_device_v1::Event,
        _: &(),
        connection: &Connection,
        _: &QueueHandle<App>,
    ) {
        let Some(clipboard) = app.clipboard.as_mut() else {
            return;
        };
        match event {
            ext_data_control_device_v1::Event::DataOffer { id } => clipboard.remember(id.id()),
            ext_data_control_device_v1::Event::Selection { id: Some(offer) } => {
                if let Some(mime) = clipboard.best_mime(&offer.id()) {
                    receive(connection, &app.clipboard_sink, |fd| {
                        offer.receive(mime.to_owned(), fd.as_fd())
                    });
                }
                clipboard.forget(&offer.id());
                offer.destroy();
            }
            _ => {}
        }
    }

    wayland_client::event_created_child!(App, ExtDataControlDeviceV1, [
        ext_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ExtDataControlOfferV1, ()),
    ]);
}

impl Dispatch<ExtDataControlOfferV1, ()> for App {
    fn event(
        app: &mut App,
        offer: &ExtDataControlOfferV1,
        event: ext_data_control_offer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        if let (Some(clipboard), ext_data_control_offer_v1::Event::Offer { mime_type }) =
            (app.clipboard.as_mut(), event)
        {
            clipboard.advertise(&offer.id(), mime_type);
        }
    }
}

// --- zwlr_data_control_v1 ---

impl Dispatch<ZwlrDataControlManagerV1, ()> for App {
    fn event(
        _: &mut App,
        _: &ZwlrDataControlManagerV1,
        _: <ZwlrDataControlManagerV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
    }
}

impl Dispatch<ZwlrDataControlDeviceV1, ()> for App {
    fn event(
        app: &mut App,
        _: &ZwlrDataControlDeviceV1,
        event: zwlr_data_control_device_v1::Event,
        _: &(),
        connection: &Connection,
        _: &QueueHandle<App>,
    ) {
        let Some(clipboard) = app.clipboard.as_mut() else {
            return;
        };
        match event {
            zwlr_data_control_device_v1::Event::DataOffer { id } => clipboard.remember(id.id()),
            zwlr_data_control_device_v1::Event::Selection { id: Some(offer) } => {
                if let Some(mime) = clipboard.best_mime(&offer.id()) {
                    receive(connection, &app.clipboard_sink, |fd| {
                        offer.receive(mime.to_owned(), fd.as_fd())
                    });
                }
                clipboard.forget(&offer.id());
                offer.destroy();
            }
            _ => {}
        }
    }

    wayland_client::event_created_child!(App, ZwlrDataControlDeviceV1, [
        zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ZwlrDataControlOfferV1, ()),
    ]);
}

impl Dispatch<ZwlrDataControlOfferV1, ()> for App {
    fn event(
        app: &mut App,
        offer: &ZwlrDataControlOfferV1,
        event: zwlr_data_control_offer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        if let (Some(clipboard), zwlr_data_control_offer_v1::Event::Offer { mime_type }) =
            (app.clipboard.as_mut(), event)
        {
            clipboard.advertise(&offer.id(), mime_type);
        }
    }
}
