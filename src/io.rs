use super::song::{download_song, Song};
use crate::db::DBExecutor;
use crate::song::{NewSong, SongRequest};
use actix::*;

pub struct MyIO {
    pub db: Addr<DBExecutor>,
}

#[derive(Debug)]
pub enum IOJob {
    DownloadSong { requested_song: SongRequest },
}

impl Message for IOJob {
    type Result = Result<NewSong, ()>;
}

impl Actor for MyIO {
    type Context = SyncContext<Self>;
}

impl Handler<IOJob> for MyIO {
    type Result = Result<NewSong, ()>;

    fn handle(&mut self, msg: IOJob, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            IOJob::DownloadSong { requested_song } => {
                // Result containing NewSong with all informations of it we need or empty error for now
                download_song(&requested_song)
            }
        }
    }
}
