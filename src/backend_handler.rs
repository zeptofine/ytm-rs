use std::time::Duration;

use iced::{
    widget::{button, hover, text},
    Command, Element,
};
use reqwest::{Client, Url};

use crate::ytmrs::YtmrsMsg;

const DEFAULT_PORT: u16 = 55001;

/// How the app connects to the server
#[derive(Debug)]
pub enum ConnectionMode {
    /// The app is a direct parent to the server.
    Child(async_process::Child),

    /// The app is a separate process that connects to the server.
    External(Box<Client>, Url),
}

#[derive(Debug, Default)]
pub enum BackendLaunchStatus {
    #[default]
    Unknown,
    PythonMissing,
    Launched(ConnectionMode),
    Failed(std::io::Error),
    Exited(usize), // exit code
}

#[derive(Debug, Default)]
pub struct BackendHandler {
    pub status: BackendLaunchStatus,
}
impl BackendHandler {
    pub fn load(port: Option<u16>) -> Self {
        let port = port.unwrap_or(DEFAULT_PORT);

        let url = Url::parse(&format!("http://localhost:{port}/")).unwrap();

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
                BackendLaunchStatus::Launched(ConnectionMode::External(
                    Box::new(Client::new()),
                    url,
                ))
            } else {
                // Try to create the server as a child process
                let python_exe = which::which("python");
                match python_exe {
                    Ok(exe) => {
                        println!["Python found at {exe:?}"];
                        let child = async_process::Command::new(exe)
                            .args(["-m", "ytm_rs_backend", &format!["{}", port]])
                            .stdout(async_process::Stdio::piped())
                            .kill_on_drop(true)
                            .spawn();
                        match child {
                            Ok(c) => BackendLaunchStatus::Launched(ConnectionMode::Child(c)),
                            Err(e) => BackendLaunchStatus::Failed(e),
                        }
                    }
                    Err(_) => BackendLaunchStatus::PythonMissing,
                }
            }
        };
        Self { status }
    }

    pub async fn poll_external_server(client: Client, url: Url) -> Result<(), reqwest::Error> {
        let _ = client.get(url).send().await?;

        Ok(())
    }

    pub fn poll(&mut self) -> Option<Command<YtmrsMsg>> {
        match &mut self.status {
            BackendLaunchStatus::Launched(ConnectionMode::Child(ref mut c)) => {
                if let Ok(Some(status)) = c.try_status() {
                    self.status = BackendLaunchStatus::Exited(status.code().unwrap() as usize);
                }
            }
            BackendLaunchStatus::Launched(ConnectionMode::External(client, ref mut url)) => {
                return Some(Command::perform(
                    Self::poll_external_server(*client.clone(), url.clone()),
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

    pub fn view(&self) -> Element<YtmrsMsg> {
        match &self.status {
            BackendLaunchStatus::Unknown => text("Backend status: ?").into(),
            BackendLaunchStatus::Launched(_) => hover(
                button("Backend status: :)"),
                text("Backend is running normally"),
            ),
            BackendLaunchStatus::Failed(e) => {
                hover(button("Backend status: :("), text(format!("{:?}", e)))
            }
            BackendLaunchStatus::Exited(code) => hover(
                button("Backend status: D:"),
                text(format!("Exit code: {:?}", code)),
            ),
            BackendLaunchStatus::PythonMissing => text("Backend status: missing").into(),
        }
    }
}
