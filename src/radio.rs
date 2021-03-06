use crate::song::Song;
use crate::song_queue::SongQueue;
use actix::fut::wrap_future;
use actix::SpawnHandle;
use actix::*;
use futures::Future;
use std::path::Path;
use std::process::Command;
use tokio_process::CommandExt;

#[derive(Default)]
/// Struct responsible for playing songs via PiFmAdv.
pub struct Radio {
    script_path: String,
    // handle to process playing song
    command_handle: Option<SpawnHandle>,
    frequency: f32,
    // is song played
    playing: bool,
}

impl Radio {
    pub fn new() -> Self {
        // panic if script isn't avialable
        let script_path = get_script_path().unwrap();
        Radio {
            script_path,
            command_handle: None,
            frequency: 104.1,
            playing: false,
        }
    }
}

pub struct PlaySong {
    pub song: Song,
    pub queue_addr: Addr<SongQueue>,
}

pub struct NextSong;
impl Message for NextSong {
    type Result = ();
}

pub struct SkipSong {
    pub queue_addr: Addr<SongQueue>,
}

impl Message for SkipSong {
    type Result = ();
}

impl Message for PlaySong {
    type Result = ();
}

impl Actor for Radio {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {}
}

impl Handler<PlaySong> for Radio {
    type Result = ();
    fn handle(&mut self, msg: PlaySong, ctx: &mut Self::Context) -> Self::Result {
        self.playing = true;
        // spawn command playing song on the radio
        // it is async, so if I will cancel the future
        // tokio will drop the command's process
        // therefore it is useful for skipping logic
        let handle = Command::new("timeout")
            .arg(&msg.song.duration.to_string())
            .arg("sudo")
            .arg(self.script_path.clone())
            .arg("--freq")
            // replace . with , because that's what library
            .arg(self.frequency.to_string().replace(".", ","))
            .arg("--audio")
            .arg(&msg.song.path)
            .spawn_async();

        let future = handle
            .expect("failed to spawn")
            .map(move |_| {
                msg.queue_addr.do_send(NextSong {});
            })
            .map_err(|e| panic!("failed to wait for exit: {}", e));
        self.command_handle = Some(ctx.spawn(wrap_future::<_, Self>(future)));
    }
}

impl Handler<SkipSong> for Radio {
    type Result = ();
    fn handle(&mut self, msg: SkipSong, ctx: &mut Self::Context) -> Self::Result {
        if let Some(command_handle) = self.command_handle {
            // cancel future and drop the command proccess
            ctx.cancel_future(command_handle);
            msg.queue_addr.do_send(NextSong {});
        }
    }
}

pub struct SetFrequency {
    pub frequency: f32,
}

impl Message for SetFrequency {
    type Result = ();
}

impl Handler<SetFrequency> for Radio {
    type Result = ();

    fn handle(&mut self, msg: SetFrequency, ctx: &mut Self::Context) -> Self::Result {
        self.frequency = msg.frequency;
    }
}

pub struct GetFrequency;

impl Message for GetFrequency {
    type Result = f32;
}

impl Handler<GetFrequency> for Radio {
    type Result = f32;

    fn handle(&mut self, msg: GetFrequency, ctx: &mut Self::Context) -> Self::Result {
        self.frequency
    }
}

/// Check if PiFmAdv script exists if not then panic.
pub fn get_script_path() -> Result<String, ()> {
    let path = Path::new("../PiFmAdv/src/pi_fm_adv");
    let script_exists = path.exists();
    if script_exists {
        Ok(std::fs::canonicalize(&path)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned())
    } else {
        Err(())
    }
}
