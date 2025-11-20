use std::sync::{Mutex, OnceLock};

use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{self},
    error,
};

#[derive(Debug)]
pub enum DiscordActivity {
    Playing {
        client_id: String,
        state: String,
        details: String,
        large_text: String,
        start_time: i64,
        end_time: Option<i64>,
        art_url: Option<String>,
        status_line: i32,
        is_playing: bool,
        buttons_texts_and_urls: Vec<(String, String)>,
    },
    Clear,
    Stop,
}

static CLIENT: OnceLock<Mutex<DiscordIpcClient>> = OnceLock::new();

pub fn discord_rpc(activity: DiscordActivity) -> Result<(), error::Error> {
    match activity {
        DiscordActivity::Playing {
            client_id,
            state,
            details,
            large_text,
            start_time,
            end_time,
            art_url,
            status_line,
            is_playing,
            buttons_texts_and_urls,
        } => {
            let mut client = CLIENT
                .get_or_init(|| Mutex::new(DiscordIpcClient::new(&client_id)))
                .lock()
                .unwrap();

            let mut assets = activity::Assets::new();

            assets = assets.large_image(art_url.as_deref().unwrap_or("graphic_eq"));

            if !large_text.is_empty() {
                assets = assets.large_text(large_text.as_str());
            }

            if !is_playing {
                assets = assets.small_image("pause_circle").small_text("Paused")
            };

            let status_display_type = match status_line {
                2 => activity::StatusDisplayType::State,
                1 => activity::StatusDisplayType::Details,
                _ => activity::StatusDisplayType::Name,
            };

            let mut activity = activity::Activity::new()
                .activity_type(activity::ActivityType::Listening)
                .state(state.as_str())
                .details(details.as_str())
                .status_display_type(status_display_type)
                .assets(assets);

            if !buttons_texts_and_urls.is_empty() {
                let buttons = buttons_texts_and_urls
                    .iter()
                    .take(2)
                    .map(|(text, url)| activity::Button::new(text, url))
                    .collect();
                activity = activity.buttons(buttons);
                activity = activity.details_url(buttons_texts_and_urls[0].1.as_str());
            }

            let mut ts = activity::Timestamps::new().start(start_time);
            if let Some(end_time) = end_time {
                ts = ts.end(end_time);
            }
            activity = activity.timestamps(ts);

            let send_result = client.set_activity(activity.clone());

            match send_result {
                Ok(_) => {}
                Err(error::Error::NotConnected) => {
                    client.connect()?;
                    client.set_activity(activity)?;
                }
                Err(error::Error::IPCConnectionFailed) => {
                    client.reconnect()?;
                    client.set_activity(activity)?;
                }
                Err(error::Error::WriteError(_)) => {
                    client.reconnect()?;
                    client.set_activity(activity)?;
                }

                Err(e) => {
                    eprintln!("Failed to set Discord activity: {e}");
                }
            }
        }
        DiscordActivity::Clear => {
            let client = CLIENT.get();

            if client.is_none() {
                return Err(error::Error::NotConnected);
            }

            let mut client = client.unwrap().lock().unwrap();

            client
                .clear_activity()
                .or_else(|_| client.set_activity(activity::Activity::new()))?;
        }

        DiscordActivity::Stop => {
            let client = CLIENT.get();

            if client.is_none() {
                return Err(error::Error::NotConnected);
            }

            let mut client = client.unwrap().lock().unwrap();

            client.close()?;
        }
    }

    Ok(())
}
