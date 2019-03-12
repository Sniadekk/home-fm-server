use super::web_socket::ws_index;
use actix_web::{middleware, server, App};
use listenfd::ListenFd;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub current_song: Arc<Mutex<String>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            current_song: Arc::new(Mutex::new(String::new())),
        }
    }
}

pub struct System {}

impl System {
    pub fn new() -> Self {
        ::std::env::set_var("RUST_LOG", "actix_web=info");
        env_logger::init();
        let sys = actix::System::new("home-fm-server");
        // listenfd object that brings hotreloading
        let mut listenfd = ListenFd::from_env();

        // Initial state of the app filled with Arc<Mutex>> to make it shareable between states
        let state = AppState::new();

        let mut server = server::new(move || {
            App::with_state(state.clone()) // <- create app with shared state
                // add our resources (routes)
                .resource("/ws/", |r| r.route().f(ws_index))
                // add middleware to log stuff
                .middleware(middleware::Logger::default())
        });

        server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
            server.listen(l)
        } else {
            server.bind("127.0.0.1:8080").unwrap()
        };
        println!("Started http server: 127.0.0.1:8080");
        server.run();
        System {}
    }
}
