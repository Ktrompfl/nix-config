//! Watching the Wayland clipboard, and deciding what the bot gets to see.
//!
//! Reading a selection needs the data-control protocol, which every
//! compositor worth running has in one of its two spellings: the newer
//! `ext_data_control_v1` and wlroots' original `zwlr_data_control_v1`. They
//! are the same protocol with different names, so both are supported and the
//! newer one is preferred.

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::io::Read;
use std::os::fd::{AsFd, OwnedFd};

use wayland_client::backend::ObjectId;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::ExtDataControlDeviceV1,
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::ExtDataControlOfferV1,
};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::ZwlrDataControlDeviceV1,
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_offer_v1::ZwlrDataControlOfferV1,
};

use super::app::App;

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
    /// Whether the selection that was already there when the watcher bound has
    /// been seen and ignored.
    ///
    /// Data control hands a new watcher the current selection immediately.
    /// That one is not a measurement anybody has just taken -- it is whatever
    /// happened to be on the clipboard, quite possibly an F3+C from a previous
    /// session -- and taking it would put a throw nobody made at the front of
    /// the list.
    primed: bool,
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
    ) -> Result<Clipboard> {
        let device = if let Ok(manager) =
            globals.bind::<ExtDataControlManagerV1, _, _>(queue, 1..=1, ())
        {
            Device::Ext(manager.get_data_device(seat, queue, ()))
        } else if let Ok(manager) =
            globals.bind::<ZwlrDataControlManagerV1, _, _>(queue, 1..=2, ())
        {
            Device::Wlr(manager.get_data_device(seat, queue, ()))
        } else {
            return Err(anyhow!(
                "the compositor offers neither ext_data_control_v1 nor \
                 zwlr_data_control_v1, so measurements cannot be picked up"
            ));
        };
        Ok(Clipboard {
            device,
            offers: HashMap::new(),
            primed: false,
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

    /// Whether a selection should be acted on. The first one never is.
    fn accepts(&mut self) -> bool {
        std::mem::replace(&mut self.primed, true)
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

/// Data control comes in two spellings that differ only in name, so one
/// implementation is written against whichever module path is handed in.
macro_rules! data_control {
    ($manager:ty, $device:ty, $offer:ty, $module:ident) => {
        impl Dispatch<$manager, ()> for App {
            fn event(
                _: &mut App,
                _: &$manager,
                _: <$manager as Proxy>::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<App>,
            ) {
            }
        }

        impl Dispatch<$device, ()> for App {
            fn event(
                app: &mut App,
                _: &$device,
                event: $module::device::Event,
                _: &(),
                connection: &Connection,
                _: &QueueHandle<App>,
            ) {
                let Some(clipboard) = app.clipboard.as_mut() else {
                    return;
                };
                match event {
                    $module::device::Event::DataOffer { id } => clipboard.remember(id.id()),
                    $module::device::Event::Selection { id: Some(offer) } => {
                        if clipboard.accepts() {
                            if let Some(mime) = clipboard.best_mime(&offer.id()) {
                                receive(connection, &app.clipboard_sink, |fd| {
                                    offer.receive(mime.to_owned(), fd.as_fd())
                                });
                            }
                        }
                        clipboard.forget(&offer.id());
                        offer.destroy();
                    }
                    $module::device::Event::Selection { id: None } => {
                        clipboard.accepts();
                    }
                    _ => {}
                }
            }

            wayland_client::event_created_child!(App, $device, [
                $module::device::EVT_DATA_OFFER_OPCODE => ($offer, ()),
            ]);
        }

        impl Dispatch<$offer, ()> for App {
            fn event(
                app: &mut App,
                offer: &$offer,
                event: $module::offer::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<App>,
            ) {
                if let (Some(clipboard), $module::offer::Event::Offer { mime_type }) =
                    (app.clipboard.as_mut(), event)
                {
                    clipboard.advertise(&offer.id(), mime_type);
                }
            }
        }
    };
}

mod ext {
    pub use wayland_protocols::ext::data_control::v1::client::ext_data_control_device_v1 as device;
    pub use wayland_protocols::ext::data_control::v1::client::ext_data_control_offer_v1 as offer;
}

mod wlr {
    pub use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1 as device;
    pub use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1 as offer;
}

data_control!(
    ExtDataControlManagerV1,
    ExtDataControlDeviceV1,
    ExtDataControlOfferV1,
    ext
);
data_control!(
    ZwlrDataControlManagerV1,
    ZwlrDataControlDeviceV1,
    ZwlrDataControlOfferV1,
    wlr
);
