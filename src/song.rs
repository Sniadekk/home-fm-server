use super::system::AppState;
use actix_web::{Json, Result, State};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{ChildStderr, ChildStdout, Command, Stdio};

#[derive(Serialize, Deserialize)]
pub struct Song {
	name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DownloadStatus {
	#[serde(rename = "Requested song wasn't available in any supported format")]
	NoFormatAvailable,
	#[serde(rename = "Network problems occured while downloading song")]
	NetworkError,
	#[serde(rename = "Successfully downloaded song")]
	Success,
}

pub struct DownloadResponse {
	status: DownloadStatus,
}

impl DownloadResponse {
	pub fn is_success(&self) -> bool {
		match self.status {
			DownloadStatus::NetworkError => false,
			DownloadStatus::NoFormatAvailable => false,
			DownloadStatus::Success => true,
			_ => false,
		}
	}

	pub fn get_status(self) -> DownloadStatus {
		self.status
	}
}

pub fn set_current_song(song_name: &str, state: &AppState) -> DownloadResponse {
	*(state.current_song.lock().unwrap()) = song_name.to_string();
	println!("current song: {}", state.current_song.lock().unwrap());
	let request_status = download_song(song_name);
	DownloadResponse {
		status: request_status,
	}
}

/// Formats the sum of two numbers as string
fn download_song(song_name: &str) -> DownloadStatus {
	println!("Downloading song");
	let static_path = PathBuf::from("static/songs");
	let search_query: &str = &format!("ytsearch1:{}", song_name);
	let output = Command::new("youtube-dl")
		// download one song from youtube
		.arg(search_query)
		// extract audio from the video and format it to mp3
		.arg("-x")
		.arg("--audio-format")
		.arg("mp3")
		// save file in /static/songs directory
		.arg(format!(
			"-o{path}/%(title)s.%(ext)s",
			path = static_path.display()
		))
		// create stdout,stderr pipeline to listen for changes
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.unwrap();
	// result of the function is built by stuff that happens in std
	read_std(output.stdout.unwrap(), output.stderr.unwrap())
}

// TODO: parse errors from stderr and match them with DownloadErr enum
fn read_std(stdout: ChildStdout, stderr: ChildStderr) -> DownloadStatus {
	// buffer to read from stdout
	let stdout_reader = BufReader::new(stdout);
	let stdout_lines = stdout_reader.lines();
	// buffer to read from stderr
	let stderr_reader = BufReader::new(stderr);
	let stderr_lines = stderr_reader.lines();

	let mut error_message = String::new();

	// simply print the std value, it's gonna be streamed through websockets to the client
	for line in stdout_lines {
		println!("Read stdout: {:?}", line);
	}

	for line in stderr_lines {
		// for now just build string containing error message
		println!("Read stdout: {:?}", line);
		match line {
			Ok(val) => error_message.push_str(&val),
			_ => (),
		}
	}

	if error_message.len() > 0 {
		DownloadStatus::NetworkError
	} else {
		DownloadStatus::Success
	}
}
