use crate::{State, caps, client::Data};
use backend::OutputId;
use std::{fs::File, os::fd::OwnedFd, os::unix::fs::FileExt};
use wire::*;

impl zwlr_gamma_control_manager_v1::Handler for State {
    fn get_gamma_control(&mut self, cx: Cx, control: Obj, output: u32) -> Res {
        let c = cx.client;
        let output = self.client(c).output(output);
        let taken = |o: OutputId| self.objects(caps::GAMMA).any(|g| matches!(g.1.state, Data::Gamma(x, _) if x == o));
        match output.filter(|&o| !taken(o)).and_then(|o| Some((o, self.backend.gamma_size(o)?))) {
            Some((output, size)) => {
                self.set((c, control.id), Data::Gamma(output, size));
                self.send(c, |out| zwlr_gamma_control_v1::gamma_size(out, control, size));
            }
            None => self.send(c, |out| zwlr_gamma_control_v1::failed(out, control)),
        }
        Ok(())
    }
}

impl zwlr_gamma_control_v1::Handler for State {
    fn set_gamma(&mut self, cx: Cx, fd: OwnedFd) -> Res {
        let Some(&mut Data::Gamma(output, size)) = self.data_mut(cx.key()) else { return Ok(()) };
        let mut bytes = vec![0; size as usize * 6];
        if File::from(fd).read_exact_at(&mut bytes, 0).is_err() {
            return Err(cx.error(zwlr_gamma_control_v1::error::INVALID_GAMMA, "gamma ramps have the wrong size"));
        }
        let ramps = bytes.as_chunks::<2>().0.iter().map(|&b| u16::from_ne_bytes(b)).collect();
        set(self, output, Some(ramps));
        Ok(())
    }
}

pub fn set(st: &mut State, output: OutputId, ramps: Option<Vec<u16>>) {
    st.backend.set_gamma(output, ramps);
    st.render_dirty = true;
}

pub fn failed(st: &mut State, id: OutputId) {
    st.each_object(caps::GAMMA, |_, out, o| {
        if matches!(o.state, Data::Gamma(x, _) if x == id) {
            o.state = Data::None;
            zwlr_gamma_control_v1::failed(out, o.obj);
        }
    });
}
