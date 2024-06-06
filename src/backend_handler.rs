use std::{future::Future, process, time::Duration};

use iced::Command;
use reqwest::{Client, Url};
use serde::Serialize;

use crate::ytmrs::YtmrsMsg;

const DEFAULT_PORT: u16 = 55001;

/// How the app connects to the server
#[derive(Debug)]
pub enum ConnectionMode {
    /// The app is a direct parent to the server.
    Child(process::Child, Url),

    /// The app is a separate process that connects to the server.
    External(Url),
}

#[derive(Debug, Serialize)]
struct RequestInfoDict {
    url: String,
    process: bool,
}

#[derive(Debug, Serialize)]
struct DownloadSongDict {
    url: String,
    convert_to: String, // CHOICES: vorbis, aac, flac, mp3
}

#[derive(Debug, Clone)]
pub enum BackendReqErr {
    RequestError,
    JsonParseError,
}
pub type RequestResult = Result<String, BackendReqErr>;

#[derive(Debug, Default)]
pub enum BackendLaunchStatus {
    #[default]
    Unknown,
    PythonMissing,
    Launched(ConnectionMode),
    Failed(std::io::Error),
    Exited(usize), // exit code
}
impl BackendLaunchStatus {
    pub fn as_string(&self) -> &'static str {
        match self {
            BackendLaunchStatus::Unknown => "?",
            BackendLaunchStatus::Launched(_) => ":)",
            BackendLaunchStatus::Failed(_) => ":(",
            BackendLaunchStatus::PythonMissing => "missing",
            BackendLaunchStatus::Exited(_) => "D:",
        }
    }
}

#[derive(Debug, Default)]
pub struct BackendHandler {
    pub status: BackendLaunchStatus,
}
impl BackendHandler {
    pub fn load(port: Option<u16>) -> Self {
        let port = port.unwrap_or(DEFAULT_PORT);

        let url = Url::parse(&format!("http://127.0.0.1:{port}/")).unwrap();

        let blocking_client = reqwest::blocking::Client::new();

        let status = {
            // Check the port for an existing server
            let resp = blocking_client
                .get(url.clone())
                .timeout(Duration::from_millis(500))
                .send();
            let (exists, is_backend) = match resp {
                Ok(re) => {
                    if let Ok(text) = re.text() {
                        println!["Port {port} is being used"];
                        (true, text == "YTM_RS_BACKEND")
                    } else {
                        (true, false)
                    }
                }
                Err(err) => {
                    println![
                        "No server running on port {port}. \n{err:?}\nLaunching server as a child"
                    ];
                    (false, false)
                }
            };

            if exists && !is_backend {
                println!["Port {port} is being used by something else"];
                BackendLaunchStatus::Unknown
            } else if exists {
                // Assumes the existing server is a backend.
                println!["Successfully polled to YTM_RS_BACKEND"];
                BackendLaunchStatus::Launched(ConnectionMode::External(url))
            } else {
                // Try to create the server as a child process
                let python_exe = which::which("python");
                match python_exe {
                    Ok(exe) => {
                        println!["Python found at {exe:?}"];
                        let child = process::Command::new(exe)
                            .args(["-m", "ytm_rs_backend", &format!["{}", port]])
                            .stdout(process::Stdio::piped())
                            // .kill_on_drop(true)
                            .spawn();
                        match child {
                            Ok(c) => BackendLaunchStatus::Launched(ConnectionMode::Child(c, url)),
                            Err(e) => BackendLaunchStatus::Failed(e),
                        }
                    }
                    Err(_) => BackendLaunchStatus::PythonMissing,
                }
            }
        };
        Self { status }
    }

    pub async fn poll_external_server(url: Url) -> Result<(), reqwest::Error> {
        let _ = reqwest::get(url).await?;

        Ok(())
    }

    pub fn poll(&mut self) -> Option<Command<YtmrsMsg>> {
        match &mut self.status {
            BackendLaunchStatus::Launched(ConnectionMode::Child(ref mut c, _)) => {
                if let Ok(Some(status)) = c.try_wait() {
                    self.status = BackendLaunchStatus::Exited(status.code().unwrap() as usize);
                }
            }
            BackendLaunchStatus::Launched(ConnectionMode::External(ref mut url)) => {
                return Some(Command::perform(
                    Self::poll_external_server(url.clone()),
                    |r| match r {
                        Ok(()) => YtmrsMsg::BackendStatusPollSuccess,
                        Err(e) => YtmrsMsg::BackendStatusPollFailure(e.to_string()),
                    },
                ));
            }
            BackendLaunchStatus::Unknown => {}
            BackendLaunchStatus::Failed(_) => todo!(),
            BackendLaunchStatus::Exited(_) => todo!(),
            BackendLaunchStatus::PythonMissing => todo!(),
        }
        None
    }

    pub fn request_info(&self, url: String) -> Option<impl Future<Output = RequestResult>> {
        if let BackendLaunchStatus::Launched(mode) = &self.status {
            println!["Requesting info for {}", url];
            Some(match mode {
                ConnectionMode::Child(_, host) | ConnectionMode::External(host) => {
                    let mut host = host.clone();
                    host.set_path("request_info");
                    let info_dict = RequestInfoDict {
                        url,
                        process: false,
                    };
                    Self::__post(host, info_dict)
                }
            })
        } else {
            None
        }
    }

    pub fn request_search(&self, query: String) -> Option<impl Future<Output = RequestResult>> {
        if let BackendLaunchStatus::Launched(mode) = &self.status {
            println!["Requesting search of {:#?}", query];
            Some(match mode {
                ConnectionMode::Child(_, host) | ConnectionMode::External(host) => {
                    let mut host = host.clone();
                    host.set_path("search");
                    host.query_pairs_mut().append_pair("q", &query);
                    Self::__get(host)
                }
            })
        } else {
            None
        }
    }

    pub fn request_download_song(
        &self,
        url: String,
    ) -> Option<impl Future<Output = RequestResult>> {
        if let BackendLaunchStatus::Launched(mode) = &self.status {
            println!["Requesting download of {}", url];
            match mode {
                ConnectionMode::Child(_, host) | ConnectionMode::External(host) => {
                    let mut host = host.clone();
                    host.set_path("download");
                    let dct = DownloadSongDict {
                        url,
                        convert_to: "wav".to_string(),
                    };
                    Some(Self::__post(host, dct))
                }
            }
        } else {
            None
        }
    }

    async fn __post<T: Serialize>(host: Url, dct: T) -> RequestResult {
        match Client::new().post(host.clone()).json(&dct).send().await {
            Err(e) => {
                println!["{e:?}"];
                Err(BackendReqErr::RequestError)
            }
            Ok(r) => r.text().await.map_err(|_| BackendReqErr::JsonParseError),
        }
    }

    async fn __get(host: Url) -> RequestResult {
        match Client::new().get(host).send().await {
            Err(e) => {
                println!["{e:?}"];
                Err(BackendReqErr::RequestError)
            }
            Ok(r) => r.text().await.map_err(|_| BackendReqErr::JsonParseError),
        }
    }
}

// #[tokio::main]
// pub async fn main() {
//     let mut response = Client::new()
//         .post("http:/localhost:55001/download")
//         .json(&DownloadSongDict {
//             url: "https://music.youtube.com/watch?v=UOJNTdIR-Mc&si=njw7WCsqu3OLj0SM".to_string(),
//         })
//         .send()
//         .await
//         .unwrap();

//     let mut buffer: Vec<u8> = Vec::new();
//     while let Ok(Some(item)) = response.chunk().await {
//         buffer.extend_from_slice(&item[..]);
//         if item.ends_with(b"\n") {
//             println!("Chunk size: {:?}", buffer.len());
//             let download_progress: Result<DownloadProgress, serde_json::Error> =
//                 serde_json::from_slice(&buffer);
//             println!["Progress: {:?}", download_progress];
//             buffer.clear()
//         }
//     }
// }
