pub mod capture;
pub mod foreign_toplevel;
pub mod gamma;
pub mod idle_notify;
pub mod layer_shell;
pub mod output_management;
pub mod output_power;
pub mod session_lock;
pub mod virtual_input;
pub mod workspaces;

pub fn output_removed(st: &mut crate::State, id: backend::OutputId) {
    output_power::output_removed(st, id);
    gamma::failed(st, id);
}
