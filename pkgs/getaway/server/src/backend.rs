use backend::{Event, Mode, OutputId};
use kms::Kms;
use nested::Nested;
use std::{error::Error, os::fd::BorrowedFd};

pub enum Backend {
    Nested(Box<Nested>),
    Kms(Box<Kms>),
}

impl Backend {
    pub fn fds(&self) -> Vec<BorrowedFd<'_>> {
        match self {
            Backend::Nested(nested) => vec![nested.fd()],
            Backend::Kms(kms) => kms.fds(),
        }
    }

    pub fn dispatch(&mut self, readable: bool, events: &mut Vec<Event>) -> Result<(), Box<dyn Error>> {
        match self {
            Backend::Nested(nested) => nested.dispatch(readable, events),
            Backend::Kms(kms) => kms.dispatch(readable, events),
        }
    }

    pub fn ready(&self, output: OutputId) -> bool {
        match self {
            Backend::Nested(nested) => nested.ready(output),
            Backend::Kms(kms) => kms.ready(output),
        }
    }

    pub fn configure(
        &mut self,
        output: OutputId,
        mode: Option<Mode>,
        vrr: bool,
        enabled: bool,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            Backend::Nested(_) => Ok(()),
            Backend::Kms(kms) => kms.configure(output, mode, vrr, enabled),
        }
    }

    pub fn test(&self, output: OutputId, mode: Option<Mode>, vrr: bool) -> Result<(), Box<dyn Error>> {
        match self {
            Backend::Nested(_) => Ok(()),
            Backend::Kms(kms) => kms.test(output, mode, vrr),
        }
    }

    pub fn constrain(&mut self, lock: Option<bool>) {
        if let Backend::Nested(nested) = self {
            nested.constrain(lock);
        }
    }

    pub fn absolute_pointer(&self) -> bool {
        matches!(self, Backend::Nested(_))
    }

    pub fn gamma_size(&self, id: OutputId) -> Option<u32> {
        match self {
            Backend::Kms(kms) => kms.gamma_size(id),
            Backend::Nested(_) => None,
        }
    }

    pub fn set_gamma(&mut self, id: OutputId, ramps: Option<Vec<u16>>) {
        if let Backend::Kms(kms) = self {
            kms.set_gamma(id, ramps);
        }
    }

    pub fn needs_commit(&self, id: OutputId) -> bool {
        matches!(self, Backend::Kms(kms) if kms.needs_commit(id))
    }

    pub fn scanned_out(&self) -> Vec<u64> {
        match self {
            Backend::Kms(kms) => kms.scanned_out(),
            Backend::Nested(_) => vec![],
        }
    }

    pub fn computes_modifiers(&self) -> bool {
        matches!(self, Backend::Kms(_))
    }

    pub fn switch_vt(&mut self, vt: i32) {
        if let Backend::Kms(kms) = self {
            kms.switch_vt(vt);
        }
    }

    pub fn step_vt(&mut self, delta: i32) {
        if let Backend::Kms(kms) = self {
            kms.step_vt(delta);
        }
    }
}
