mod application;
mod handler;
mod service;

use crate::application::HomenisState;
use crate::handler::connect_socket;

fn main() {
    let state = HomenisState::new();

    connect_socket(state.token(), false);

    loop {}
}
