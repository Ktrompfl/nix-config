use crate::State;
use backend::{Gesture, GestureKind};
use wire::*;

impl zwp_pointer_gestures_v1::Handler for State {
    fn get_swipe_gesture(&mut self, _: Cx, _: Obj, _: u32) {}

    fn get_pinch_gesture(&mut self, _: Cx, _: Obj, _: u32) {}

    fn get_hold_gesture(&mut self, _: Cx, _: Obj, _: u32) {}
}

impl zwp_pointer_gesture_swipe_v1::Handler for State {}

impl zwp_pointer_gesture_pinch_v1::Handler for State {}

impl zwp_pointer_gesture_hold_v1::Handler for State {}

pub fn event(st: &mut State, gesture: Gesture) {
    if let Gesture::Begin(kind, _) = gesture {
        st.seat.gesture = crate::seat::pointer_target(st).map(|surface| (kind, surface));
    }
    let locked = crate::privileged::session_lock::locked(st);
    let Some((kind, surface)) = st.seat.gesture.filter(|g| !locked || crate::seat::pointer_target(st) == Some(g.1))
    else {
        return;
    };
    if let Gesture::End(_) = gesture {
        st.seat.gesture = None;
    }
    let (serial, time) = (st.next_serial(), st.time());
    let Some(client) = st.clients.get_mut(&surface.0) else { return };
    let interface = match kind {
        GestureKind::Swipe => Interface::ZwpPointerGestureSwipeV1,
        GestureKind::Pinch => Interface::ZwpPointerGesturePinchV1,
        GestureKind::Hold => Interface::ZwpPointerGestureHoldV1,
    };
    client.each(interface, |out, obj| match (kind, gesture) {
        (GestureKind::Swipe, Gesture::Begin(_, n)) => {
            zwp_pointer_gesture_swipe_v1::begin(out, obj, serial, time, surface.1, n)
        }
        (GestureKind::Swipe, Gesture::Update { dx, dy, .. }) => {
            zwp_pointer_gesture_swipe_v1::update(out, obj, time, dx.into(), dy.into())
        }
        (GestureKind::Swipe, Gesture::End(c)) => zwp_pointer_gesture_swipe_v1::end(out, obj, serial, time, c.into()),
        (GestureKind::Pinch, Gesture::Begin(_, n)) => {
            zwp_pointer_gesture_pinch_v1::begin(out, obj, serial, time, surface.1, n)
        }
        (GestureKind::Pinch, Gesture::Update { dx, dy, scale, rotation }) => {
            zwp_pointer_gesture_pinch_v1::update(out, obj, time, dx.into(), dy.into(), scale.into(), rotation.into())
        }
        (GestureKind::Pinch, Gesture::End(c)) => zwp_pointer_gesture_pinch_v1::end(out, obj, serial, time, c.into()),
        (GestureKind::Hold, Gesture::Begin(_, n)) => {
            zwp_pointer_gesture_hold_v1::begin(out, obj, serial, time, surface.1, n)
        }
        (GestureKind::Hold, Gesture::End(c)) => zwp_pointer_gesture_hold_v1::end(out, obj, serial, time, c.into()),
        (GestureKind::Hold, Gesture::Update { .. }) => {}
    });
}
