use std::sync::{Mutex, OnceLock};

use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{self},
    error,
};

#[derive(Debug)]
pub struct DiscordActivity {
    pub client_id: String,
    pub name: String,
    pub state: String,
    pub details: String,
    pub large_text: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub details_url: String,
    pub art_url: String,
    pub status_line: i32,
    pub is_playing: bool,
    pub button_text: String,
    pub button_url: String,
}

static CLIENT: OnceLock<Mutex<DiscordIpcClient>> = OnceLock::new();

// pass None to clear activity
pub fn update(da: DiscordActivity) -> Result<(), error::Error> {
    let mut client = CLIENT
        .get_or_init(|| Mutex::new(DiscordIpcClient::new(&da.client_id)))
        .lock()
        .unwrap();

    let mut assets = activity::Assets::new();

    if da.art_url.is_empty() {
        assets = assets.large_image("graphic_eq");
    } else {
        assets = assets.large_image(&da.art_url);
    }

    if !da.large_text.is_empty() {
        assets = assets.large_text(&da.large_text);
    }

    if !da.is_playing {
        assets = assets.small_image("pause_circle").small_text("Paused")
    };

    let status_display_type = match da.status_line {
        2 => activity::StatusDisplayType::State,
        1 => activity::StatusDisplayType::Details,
        _ => activity::StatusDisplayType::Name,
    };

    let mut activity = activity::Activity::new()
        .activity_type(activity::ActivityType::Listening)
        .state(&da.state)
        .details(&da.details)
        .status_display_type(status_display_type)
        .assets(assets);

    if !da.name.is_empty() {
        activity = activity.name(&da.name);
    }

    if !da.details_url.is_empty() {
        activity = activity.details_url(&da.details_url);
    }

    if !da.button_text.is_empty() && !da.button_url.is_empty() {
        let buttons = vec![activity::Button::new(&da.button_text, &da.button_url)];
        activity = activity.buttons(buttons);
    }

    let mut ts = activity::Timestamps::new().start(da.start_time);
    if let Some(end_time) = da.end_time {
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
            log::error!("Failed to set Discord activity: {e}");
        }
    }

    Ok(())
}

pub fn clear(shutdown: bool) -> Result<(), error::Error> {
    let client = CLIENT.get();

    if client.is_none() {
        return Err(error::Error::NotConnected);
    }

    let mut client = client.unwrap().lock().unwrap();
    let _ = client.clear_activity();

    if shutdown {
        client.close()?;
    }

    Ok(())
}
