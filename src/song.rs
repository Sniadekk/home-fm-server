use super::db::{DeleteSong, GetAllSongs, GetRandomSong, ToggleSongNsfw};
use super::schema::songs;
use super::system::AppState;
use actix_web::{AsyncResponder, Error as AWError, FutureResponse, HttpResponse, Path, State};
use chrono::prelude::*;
use diesel::{Insertable, Queryable};
use futures::future::Future;
use serde::{self, Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SongRequest {
    pub artists: String,
    pub name: String,
    #[serde(skip_deserializing, default = "now")]
    pub requested_at: DateTime<Utc>,
    thumbnail_url: String,
    pub nsfw: bool,
}

fn now() -> DateTime<Utc> {
    Utc::now()
}

impl SongRequest {
    /// {song's name} - {song's artists separated by ", " }
    pub fn get_formatted_name(&self) -> String {
        format!("{} - {}", self.name, self.artists)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Queryable)]
pub struct Song {
    id: i32,
    pub name: String,
    #[serde(skip_serializing)]
    pub path: String,
    pub duration: i32,
    thumbnail_url: String,
    artists: String,
    nsfw: bool,
}

#[derive(Insertable, Clone, Debug)]
#[table_name = "songs"]
pub struct NewSong {
    pub name: String,
    path: String,
    duration: i32,
    thumbnail_url: String,
    // , separated array
    pub artists: String,
    nsfw: bool,
}

/// Get song's path inside /static/songs.
pub fn get_song_path(song_name: &str) -> String {
    let canonicalized_path = std::fs::canonicalize(PathBuf::from("static/songs")).unwrap();
    format!("{}/{}", canonicalized_path.display(), song_name)
}

/// Get path of the json saved by youtube-dl with informations about downloaded song.
fn get_json_path(song_path: &str) -> String {
    format!("{}.info.json", song_path)
}

/// Downloads song from youtube via youtube-dl and returns boolean  whether song was downloaded or not.
pub fn download_song(requested_song: &SongRequest) -> Result<NewSong, ()> {
    let song_path = get_song_path(&requested_song.get_formatted_name());
    let search_query: &str = &format!("ytsearch1:{}", &requested_song.get_formatted_name());
    println!("{}, {}", song_path, &requested_song.get_formatted_name());
    let output = Command::new("youtube-dl")
        // download one song from youtube
        .current_dir("./static/songs")
        .arg(search_query)
        // extract audio from the video and format it to mp3
        .arg("-x")
        .arg("--audio-format")
        .arg("wav")
        .arg("--output")
        // why not just use song_path? without %(ext)s weird things happen inside youtube-dl and it outputs not working on rpi working file
        .arg(format!(
            "{}.%(ext)s",
            &requested_song.get_formatted_name().clone()
        ))
        .arg("--write-info-json")
        .output();
    if output.is_ok() {
        let info = get_song_info(&song_path, &requested_song.name).unwrap();
        // if there is no thumbnail specified use the one provided by youtube-dl
        let thumbnail_url = if requested_song.thumbnail_url == "none" {
            info.thumbnail
        } else {
            requested_song.thumbnail_url.clone()
        };
        Ok(NewSong {
            duration: info.duration,
            name: requested_song.name.clone(),
            artists: requested_song.artists.clone(),
            thumbnail_url,
            path: format!("{}.wav", song_path),
            nsfw: requested_song.nsfw,
        })
    } else {
        println!(
            "Error during downloading a song - {:#?}",
            String::from_utf8(output.unwrap().stderr)
        );
        Err(())
    }
    // decode duration from .info.json that youtube-dl downloads
}

#[derive(Serialize, Deserialize)]
struct Info {
    duration: i32,
    thumbnail: String,
}

/// Extracts informations from song.info.json saved by youtube-dl with informations about downloaded song.
fn get_song_info(song_path: &str, song_name: &str) -> Result<Info, ()> {
    let json_path = get_json_path(song_path);
    let file = fs::File::open(&json_path);
    match file {
        Ok(file) => {
            let reader = BufReader::new(file);
            let json_content: Info = serde_json::from_reader(reader).unwrap();
            fs::remove_file(json_path);
            Ok(json_content)
        }
        e => {
            eprintln!("error during opening a file - {:#?}", e);
            Err(())
        }
    }
}

// API functions
/// GET /songs
pub fn get_all_songs(state: State<AppState>) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(GetAllSongs {})
        .and_then(|res| Ok(HttpResponse::Ok().json(res.unwrap())))
        .from_err()
        .responder()
}

/// PUT /songs/toggle_nsfw/{song_id}/{is_nsfw}
pub fn toggle_song_nsfw(
    path: Path<(i32, bool)>,
    state: State<AppState>,
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(ToggleSongNsfw {
            id: path.0,
            is_nsfw: path.1,
        })
        .and_then(|song| Ok(HttpResponse::Ok().json(song.unwrap())))
        .from_err()
        .responder()
}

#[derive(Deserialize)]
pub struct SongId {
    id: i32,
}

/// DELETE /songs/{song_id}
pub fn delete_song(path: Path<SongId>, state: State<AppState>) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(DeleteSong { song_id: path.id })
        .and_then(|song| Ok(HttpResponse::Ok().json(song.unwrap())))
        .from_err()
        .responder()
}
