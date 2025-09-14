use std::sync::{Mutex, OnceLock};

use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity, error};

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
        status_is_state: bool,
        is_playing: bool,
    },
    Clear,
    Stop,
}

static CLIENT: OnceLock<Mutex<DiscordIpcClient>> = OnceLock::new();

pub fn discord_rpc(activity: DiscordActivity) -> Result<(), error::Error> {
    println!("Sending Discord RPC: {:?}", activity);

    match activity {
        DiscordActivity::Playing {
            client_id,
            state,
            details,
            large_text,
            start_time,
            end_time,
            art_url,
            status_is_state,
            is_playing,
        } => {
            let mut client = CLIENT
                .get_or_init(|| Mutex::new(DiscordIpcClient::new(&client_id)))
                .lock()
                .unwrap();

            let mut assets = activity::Assets::new();

            assets = assets
                .large_image(art_url.as_deref().unwrap_or("cover-placeholder"))
                .large_text(large_text.as_str());

            assets = if is_playing {
                assets.small_image("playing").small_text("Playing")
            } else {
                assets.small_image("paused").small_text("Paused")
            };

            let status_display_type = if status_is_state {
                activity::StatusDisplayType::State
            } else {
                activity::StatusDisplayType::Details
            };

            let mut activity = activity::Activity::new()
                .activity_type(activity::ActivityType::Listening)
                .state(state.as_str())
                .details(details.as_str())
                .status_display_type(status_display_type)
                .assets(assets);

            // Don't show timestamp if the song is paused, since Discord will continue counting up otherwise
            activity = if is_playing {
                let mut ts = activity::Timestamps::new().start(start_time);
                if let Some(end_time) = end_time {
                    ts = ts.end(end_time);
                }
                activity.timestamps(ts)
            } else {
                activity
            };

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
