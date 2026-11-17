use crate::{State, client::Data, seat, xdg};
use wire::*;

const MAX_TOKENS: usize = 256;
const MAX_CLIENT_TOKENS: usize = 16;

impl xdg_activation_v1::Handler for State {
    fn get_activation_token(&mut self, cx: Cx, id: Obj) {
        self.set((cx.client, id.id), Data::Token(None, false));
    }

    fn activate(&mut self, cx: Cx, token: String, surface: u32) {
        if let Some(trusted) = self.tokens.remove(&token)
            && let Some(w) = xdg::window_of(self, (cx.client, surface))
        {
            xdg::activate(self, w, trusted);
        }
    }
}

fn unused(st: &mut State, cx: Cx) -> Result<&mut Option<u32>, Error> {
    match st.data_mut(cx.key()) {
        Some(Data::Token(serial, false)) => Ok(serial),
        _ => Err(cx.error(xdg_activation_token_v1::error::ALREADY_USED, "token was already committed")),
    }
}

impl xdg_activation_token_v1::Handler for State {
    fn set_serial(&mut self, cx: Cx, serial: u32, _: u32) -> Res {
        *unused(self, cx)? = Some(serial);
        Ok(())
    }

    fn set_app_id(&mut self, cx: Cx, _: String) -> Res {
        unused(self, cx).map(drop)
    }

    fn set_surface(&mut self, cx: Cx, _: u32) -> Res {
        unused(self, cx).map(drop)
    }

    fn commit(&mut self, cx: Cx) -> Res {
        let serial = *unused(self, cx)?;
        let c = cx.client;
        self.set(cx.key(), Data::Token(None, true));
        let prefix = format!("getaway-{c}-");
        if self.tokens.keys().filter(|t| t.starts_with(&prefix)).count() >= MAX_CLIENT_TOKENS {
            self.tokens.retain(|t, _| !t.starts_with(&prefix));
        }
        if self.tokens.len() >= MAX_TOKENS {
            self.tokens.clear();
        }
        let token = format!("{prefix}{}", self.next_id());
        let trusted = serial.is_some_and(|s| seat::recent_input(self, c, s)) || self.seat.keyboard_client() == Some(c);
        self.tokens.insert(token.clone(), trusted);
        self.send(c, |out| xdg_activation_token_v1::done(out, cx.obj, &token));
        Ok(())
    }
}
