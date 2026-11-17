use std::collections::VecDeque;

const MAX_CONFIGURES: usize = 32;

#[derive(Default)]
pub struct Configures<T> {
    sent: VecDeque<(u32, T)>,
    last: Option<T>,
    acked: Option<u32>,
}

impl<T: Copy> Configures<T> {
    pub fn last(&self) -> Option<T> {
        self.last
    }

    pub fn acked(&self) -> Option<u32> {
        self.acked
    }

    pub fn send(&mut self, serial: u32, state: T) {
        if self.sent.len() == MAX_CONFIGURES {
            self.sent.pop_front();
        }
        self.sent.push_back((serial, state));
        self.last = Some(state);
    }

    pub fn ack(&mut self, serial: u32) -> bool {
        let after = self.acked.and_then(|a| self.position(a)).map_or(0, |i| i + 1);
        let valid = self.sent.range(after..).any(|c| c.0 == serial);
        if valid {
            self.acked = Some(serial);
        }
        valid
    }

    pub fn get(&self, serial: u32) -> Option<T> {
        self.position(serial).map(|i| self.sent[i].1)
    }

    pub fn unmap(&mut self) {
        (self.last, self.acked) = (None, None);
    }

    fn position(&self, serial: u32) -> Option<usize> {
        self.sent.iter().position(|c| c.0 == serial)
    }
}

pub fn reached(acked: u32, serial: u32) -> bool {
    acked.wrapping_sub(serial) < 1 << 31
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack_consumes_older_configures() {
        let mut c = Configures::default();
        assert!(!c.ack(1));
        (1..=3).for_each(|s| c.send(s, s * 10));
        assert!(c.ack(2));
        assert!(!c.ack(2));
        assert!(!c.ack(1));
        assert!(c.ack(3));
        assert_eq!((c.acked(), c.get(3), c.last()), (Some(3), Some(30), Some(30)));
    }

    #[test]
    fn unmap_forgets_the_ack_but_not_the_serials() {
        let mut c = Configures::default();
        c.send(1, ());
        assert!(c.ack(1));
        c.unmap();
        assert_eq!((c.acked(), c.last()), (None, None));
        assert!(c.ack(1));
    }

    #[test]
    fn queue_is_bounded() {
        let mut c = Configures::default();
        (0..100).for_each(|s| c.send(s, s));
        assert!(!c.ack(0));
        assert!(c.ack(99 - MAX_CONFIGURES as u32 + 1));
    }

    #[test]
    fn reached_wraps() {
        assert!(reached(5, 5) && reached(6, 5) && !reached(4, 5));
        assert!(reached(1, u32::MAX) && !reached(u32::MAX, 1));
    }
}
