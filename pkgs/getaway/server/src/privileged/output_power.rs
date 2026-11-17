use crate::{State, caps, client::Data, output};
use backend::OutputId;
use wire::*;

impl zwlr_output_power_manager_v1::Handler for State {
    fn get_output_power(&mut self, cx: Cx, control: Obj, output: u32) {
        let c = cx.client;
        match self.client(c).output(output).and_then(|o| output::find(self, o)).map(|o| (o.id, mode(o.off))) {
            Some((output, mode)) => {
                self.set((c, control.id), Data::Power(output));
                self.send(c, |out| zwlr_output_power_v1::mode(out, control, mode));
            }
            None => self.send(c, |out| zwlr_output_power_v1::failed(out, control)),
        }
    }
}

impl zwlr_output_power_v1::Handler for State {
    fn set_mode(&mut self, cx: Cx, mode: u32) -> Res {
        use zwlr_output_power_v1::mode::*;
        let Some(&mut Data::Power(output)) = self.data_mut(cx.key()) else { return Ok(()) };
        match mode {
            ON | OFF => set(self, output, mode == OFF),
            _ => return Err(cx.error(zwlr_output_power_v1::error::INVALID_MODE, "unknown power mode")),
        }
        Ok(())
    }
}

fn mode(off: bool) -> u32 {
    if off { zwlr_output_power_v1::mode::OFF } else { zwlr_output_power_v1::mode::ON }
}

fn controls(st: &mut State, id: OutputId, mut f: impl FnMut(&mut Out, &mut wire::Object<Data>)) {
    st.each_object(caps::OUTPUT_POWER, |_, out, o| {
        if matches!(o.state, Data::Power(x) if x == id) {
            f(out, o);
        }
    });
}

pub fn set(st: &mut State, id: OutputId, off: bool) {
    let Some(o) = output::find_mut(st, id).filter(|o| o.off != off) else { return };
    o.off = off;
    output::reconfigure(st);
    controls(st, id, |out, o| zwlr_output_power_v1::mode(out, o.obj, mode(off)));
}

pub fn output_removed(st: &mut State, id: OutputId) {
    controls(st, id, |out, o| {
        o.state = Data::None;
        zwlr_output_power_v1::failed(out, o.obj);
    });
}
