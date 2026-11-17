use crate::{State, base::KDE_SERVER_SIDE, xdg};
use wire::*;

impl org_kde_kwin_server_decoration_manager::Handler for State {
    fn create(&mut self, cx: Cx, decoration: Obj, _: u32) {
        self.send(cx.client, |out| org_kde_kwin_server_decoration::mode(out, decoration, KDE_SERVER_SIDE));
    }
}

impl org_kde_kwin_server_decoration::Handler for State {
    fn request_mode(&mut self, cx: Cx, _: u32) {
        self.send(cx.client, |out| org_kde_kwin_server_decoration::mode(out, cx.obj, KDE_SERVER_SIDE));
    }
}

impl xdg_wm_dialog_v1::Handler for State {
    fn get_xdg_dialog(&mut self, _: Cx, _: Obj, _: u32) {}
}

impl xdg_dialog_v1::Handler for State {
    fn set_modal(&mut self, _: Cx) {}

    fn unset_modal(&mut self, _: Cx) {}
}

impl xdg_toplevel_icon_manager_v1::Handler for State {
    fn create_icon(&mut self, _: Cx, _: Obj) {}

    fn set_icon(&mut self, _: Cx, _: u32, _: Option<u32>) {}
}

impl xdg_toplevel_icon_v1::Handler for State {
    fn set_name(&mut self, _: Cx, _: String) {}

    fn add_buffer(&mut self, _: Cx, _: u32, _: i32) {}
}

impl xdg_toplevel_tag_manager_v1::Handler for State {
    fn set_toplevel_tag(&mut self, cx: Cx, toplevel: u32, tag: String) {
        if let Some(key) = xdg::toplevel_object(self, cx.client, toplevel) {
            xdg::set_tag(self, key, tag);
        }
    }

    fn set_toplevel_description(&mut self, _: Cx, _: u32, _: String) {}
}

impl xdg_system_bell_v1::Handler for State {
    fn ring(&mut self, _: Cx, _: Option<u32>) {}
}
