use crate::{
    ClientId, State,
    buffer::{self, Buffer},
    client::Data,
};
use gfx::{Dmabuf, Plane, Renderer};
use rustix::fs::{MemfdFlags, SealFlags, fcntl_add_seals, memfd_create};
use std::{
    fs::File,
    io::{self, Write},
    os::fd::{AsFd, OwnedFd},
    rc::Rc,
};
use wire::*;

const MAX_SIZE: i32 = 16384;

#[derive(Default)]
pub struct Params {
    planes: [Option<Plane>; 4],
    modifier: u64,
}

pub struct FormatTable {
    fd: OwnedFd,
    size: u32,
    indices: Vec<u8>,
}

impl FormatTable {
    pub fn new(renderer: &Renderer) -> io::Result<Self> {
        let formats = renderer.formats();
        let fd = memfd_create("getaway-formats", MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING)?;
        let mut file = File::from(fd);
        for &(format, modifier) in formats {
            file.write_all(&format.to_ne_bytes())?;
            file.write_all(&0u32.to_ne_bytes())?;
            file.write_all(&modifier.to_ne_bytes())?;
        }
        let fd = OwnedFd::from(file);
        fcntl_add_seals(&fd, SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE | SealFlags::SEAL)?;
        let indices = (0..formats.len() as u16).flat_map(u16::to_ne_bytes).collect();
        Ok(Self { fd, size: formats.len() as u32 * 16, indices })
    }
}

pub fn bound(st: &mut State, c: ClientId, obj: Obj) {
    if obj.version >= 4 {
        return;
    }
    let formats = st.renderer.formats();
    let Some(client) = st.clients.get_mut(&c) else { return };
    for (i, &(format, modifier)) in formats.iter().enumerate() {
        match obj.version {
            3 => zwp_linux_dmabuf_v1::modifier(&mut client.out, obj, format, (modifier >> 32) as u32, modifier as u32),
            _ if formats[..i].iter().all(|f| f.0 != format) => {
                zwp_linux_dmabuf_v1::format(&mut client.out, obj, format)
            }
            _ => {}
        }
    }
}

impl zwp_linux_dmabuf_v1::Handler for State {
    fn create_params(&mut self, cx: Cx, params_id: Obj) {
        self.set((cx.client, params_id.id), Data::Params(Params::default()));
    }

    fn get_default_feedback(&mut self, cx: Cx, feedback: Obj) {
        send_feedback(self, cx.client, feedback);
    }

    fn get_surface_feedback(&mut self, cx: Cx, feedback: Obj, _: u32) {
        send_feedback(self, cx.client, feedback);
    }
}

impl zwp_linux_dmabuf_feedback_v1::Handler for State {}

fn send_feedback(st: &mut State, c: ClientId, feedback: Obj) {
    let device = st.renderer.render_node().to_ne_bytes();
    let Some(client) = st.clients.get_mut(&c) else { return };
    let (out, table) = (&mut client.out, &st.format_table);
    zwp_linux_dmabuf_feedback_v1::format_table(out, feedback, table.fd.as_fd(), table.size);
    if feedback.version < 6 {
        zwp_linux_dmabuf_feedback_v1::main_device(out, feedback, &device);
    }
    zwp_linux_dmabuf_feedback_v1::tranche_target_device(out, feedback, &device);
    zwp_linux_dmabuf_feedback_v1::tranche_formats(out, feedback, &table.indices);
    let sampling = if feedback.version >= 6 { zwp_linux_dmabuf_feedback_v1::tranche_flags::SAMPLING } else { 0 };
    zwp_linux_dmabuf_feedback_v1::tranche_flags(out, feedback, sampling);
    zwp_linux_dmabuf_feedback_v1::tranche_done(out, feedback);
    zwp_linux_dmabuf_feedback_v1::done(out, feedback);
}

impl zwp_linux_buffer_params_v1::Handler for State {
    fn add(&mut self, cx: Cx, fd: OwnedFd, plane_idx: u32, offset: u32, stride: u32, hi: u32, lo: u32) -> Res {
        use zwp_linux_buffer_params_v1::error::*;
        let error = |code, message| Err(cx.error(code, message));
        let Some(Data::Params(params)) = self.data_mut(cx.key()) else {
            return error(ALREADY_USED, "params already used");
        };
        let Some(plane) = params.planes.get_mut(plane_idx as usize) else {
            return error(PLANE_IDX, "plane index out of bounds");
        };
        if plane.is_some() {
            return error(PLANE_SET, "plane already set");
        }
        let modifier = (u64::from(hi) << 32) | u64::from(lo);
        let first = params.planes.iter().all(Option::is_none);
        if cx.obj.version >= 5 && !first && params.modifier != modifier {
            return error(INVALID_FORMAT, "planes use different modifiers");
        }
        params.planes[plane_idx as usize] = Some(Plane { fd, offset, stride });
        params.modifier = modifier;
        Ok(())
    }

    fn create(&mut self, cx: Cx, width: i32, height: i32, format: u32, flags: u32) -> Res {
        let dmabuf = take(self, cx, (width, height), format)?;
        match import(self, cx.client, dmabuf, None, flags) {
            Ok(id) => self.send(cx.client, |out| zwp_linux_buffer_params_v1::created(out, cx.obj, id)),
            Err(e) => {
                log::warn!("dmabuf import failed: {e}");
                self.send(cx.client, |out| zwp_linux_buffer_params_v1::failed(out, cx.obj));
            }
        }
        Ok(())
    }

    fn create_immed(&mut self, cx: Cx, buffer_id: Obj, width: i32, height: i32, format: u32, flags: u32) -> Res {
        let dmabuf = take(self, cx, (width, height), format)?;
        let invalid = |e| cx.error(zwp_linux_buffer_params_v1::error::INVALID_WL_BUFFER, e);
        import(self, cx.client, dmabuf, Some(buffer_id.id), flags).map_err(invalid)?;
        Ok(())
    }

    fn set_sampling_device(&mut self, cx: Cx, device: Vec<u8>) -> Res {
        if device.len() != size_of::<libc::dev_t>() {
            return Err(cx.error(zwp_linux_buffer_params_v1::error::INVALID_DEV_T_SIZE, "device is not a dev_t"));
        }
        Ok(())
    }
}

fn take(st: &mut State, cx: Cx, size: (i32, i32), format: u32) -> Result<Dmabuf, Error> {
    use zwp_linux_buffer_params_v1::error::*;
    let obj = cx.obj;
    let Some(Data::Params(params)) = st.data_mut(cx.key()).map(std::mem::take) else {
        return Err(Error::new(obj.id, ALREADY_USED, "params already used"));
    };
    let count = params.planes.iter().take_while(|p| p.is_some()).count();
    if count == 0 || params.planes[count..].iter().any(Option::is_some) {
        return Err(Error::new(obj.id, INCOMPLETE, "missing planes"));
    }
    if !(1..=MAX_SIZE).contains(&size.0) || !(1..=MAX_SIZE).contains(&size.1) {
        return Err(Error::new(obj.id, INVALID_DIMENSIONS, format!("invalid size {}x{}", size.0, size.1)));
    }
    let planes = params.planes.into_iter().flatten().collect();
    Ok(Dmabuf { size, format, modifier: params.modifier, planes })
}

fn import(st: &mut State, c: ClientId, dmabuf: Dmabuf, id: Option<u32>, flags: u32) -> Result<u32, String> {
    if flags != 0 {
        return Err(format!("unsupported buffer flags {flags:#x}"));
    }
    let texture = st.renderer.import(&dmabuf, false).map_err(|e| e.to_string())?;
    let id = id.unwrap_or_else(|| st.client(c).add_server(Interface::WlBuffer, 1, Data::None).id);
    let buffer = Buffer::new(id, buffer::Data::Dmabuf(dmabuf), Some(texture), st.garbage.clone());
    st.set((c, id), Data::Buffer(Rc::new(buffer)));
    Ok(id)
}
