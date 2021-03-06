use secrets::get_secrets;
use tokio_core;
use hyper;
use serde_json;
use errors;
use errors::ResultExt;
use futures::Future;
use futures::Stream;
use model;
use time;
use serde;

#[derive(Serialize, Deserialize)]
struct Game {
    appid: u64,
    name: String,
    img_icon_url: String,
    img_logo_url: String,
    has_community_visible_stats: Option<bool>,
    playtime_forever: u64,
    playtime_2weeks: Option<u64>,
}

#[derive(Serialize, Deserialize)]
struct OwnedGames {
    game_count: u64,
    games: Vec<Game>,
}

#[derive(Serialize, Deserialize)]
struct OwnedGamesResponse {
    response: OwnedGames
}

fn request<T>(url: &str) -> Result<T, errors::Error> where for<'a> T: serde::Deserialize<'a> {
    let mut core = tokio_core::reactor::Core::new().chain_err(|| "unable to intialize tokio core")?;
    let client = hyper::Client::new(&core.handle());
    let response_future = client.get(url.parse().chain_err(|| "unable to parse url")?).and_then(|res| res.body().concat2());
    let chunk = core.run(response_future).chain_err(|| "unable to reap future")?;

    serde_json::from_reader(chunk.as_ref()).chain_err(|| "unable to parse json")
}

pub fn sync() -> Result<(), errors::Error> {
    let users = model::get_all_users().chain_err(|| "unable to load all users")?;
    for user in users {
        if let Some(steam_id) = user.steam_id {
            sync_user(user.id, &steam_id)?;
        }
    }
    Ok(())
}

fn sync_user(user_id: i64, steam_id: &String) -> Result<(), errors::Error> {
    let secrets = get_secrets()?;
    let owned_games_response: OwnedGamesResponse = request(
        &format!(
            "http://api.steampowered.com/IPlayerService/GetOwnedGames/v0001/?key={}&steamid={}&include_appinfo=1&format=json",
            secrets.steam_api_key,
            steam_id,
        )
    )?;

    for game in owned_games_response.response.games {
        // TODO upsert by steam id instead of name
        let game_id = upsert_game(&game)?;
        let has_played = game.playtime_forever > 0;
        let play_state = if has_played { "unfinished" } else { "unplayed" }.to_string();
        let start_date = if has_played { Some(time::get_time().sec) } else { None };
        model::upsert_user_game(
            model::NewUserGame{
                user_id: user_id,
                game_id: game_id,
                play_state: play_state,
                platform: "win".to_string(),
                acquisition_date: time::get_time().sec,
                start_date: start_date,
                beat_date: None,
            }
        ).chain_err(|| "unable to upsert game")?;
    }
    Ok(())
}

fn upsert_game(game: &Game) -> Result<i64, errors::Error> {
    match model::get_game_by_steam_id(game.appid) {
        Ok(game) => Ok(game.id),
        Err(_) => {
            model::insert_game(
                model::NewGame{
                    name: game.name.clone(),
                    steam_id: Some(game.appid as i64),
                },
            ).chain_err(|| "unable to insert game")
        }
    }
}
