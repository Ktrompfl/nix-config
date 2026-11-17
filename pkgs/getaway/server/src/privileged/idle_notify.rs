use crate::{State, caps, client::Data};
use std::time::{Duration, Instant};
use wire::*;

pub struct Notification {
    timeout: Duration,
    input_only: bool,
    idled: bool,
    start: Instant,
    inhibited: bool,
}

impl ext_idle_notifier_v1::Handler for State {
    fn get_idle_notification(&mut self, cx: Cx, id: Obj, timeout: u32, _: u32) {
        notification(self, cx, id, timeout, false);
    }

    fn get_input_idle_notification(&mut self, cx: Cx, id: Obj, timeout: u32, _: u32) {
        notification(self, cx, id, timeout, true);
    }
}

fn notification(st: &mut State, cx: Cx, obj: Obj, timeout: u32, input_only: bool) {
    let timeout = Duration::from_millis(timeout.into());
    let (start, inhibited) = (Instant::now(), inhibited(st));
    let n = Notification { timeout, input_only, idled: false, start, inhibited };
    st.set((cx.client, obj.id), Data::Notification(n));
}

impl ext_idle_notification_v1::Handler for State {}

fn notifications(st: &mut State, mut f: impl FnMut(&mut Out, Obj, &mut Notification)) {
    st.each_object(caps::IDLE_NOTIFY, |_, out, o| {
        if let Data::Notification(n) = &mut o.state {
            f(out, o.obj, n);
        }
    });
}

pub fn activity(st: &mut State) {
    st.last_input = Instant::now();
    notifications(st, |out, obj, n| {
        if std::mem::take(&mut n.idled) {
            ext_idle_notification_v1::resumed(out, obj);
        }
    });
}

fn inhibited(st: &State) -> bool {
    st.surfaces.values().any(|s| s.inhibitors > 0 && !s.outputs.is_empty())
}

fn due(n: &Notification, inhibited: bool, last_input: Instant) -> Option<Instant> {
    (!n.idled && (n.input_only || !inhibited)).then(|| last_input.max(n.start) + n.timeout)
}

pub fn deadline(st: &State) -> Option<Instant> {
    let inhibited = inhibited(st);
    let notifications = st.objects(caps::IDLE_NOTIFY);
    notifications
        .filter_map(|(_, o)| if let Data::Notification(n) = &o.state { Some(n) } else { None })
        .filter_map(|n| due(n, inhibited, st.last_input))
        .min()
}

pub fn check(st: &mut State) {
    let (now, inhibited, last_input) = (Instant::now(), inhibited(st), st.last_input);
    notifications(st, |out, obj, n| {
        if std::mem::replace(&mut n.inhibited, inhibited) && !inhibited && !n.input_only {
            n.start = now;
        }
        if due(n, inhibited, last_input).is_some_and(|d| d <= now) {
            n.idled = true;
            ext_idle_notification_v1::idled(out, obj);
        }
    });
}
