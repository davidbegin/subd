use anyhow::Result;
use elevenlabs_api::{Auth, Elevenlabs};
use serde::{Deserialize, Serialize};
use server::ai_scenes;
use server::audio;
use server::handlers;
use server::uberduck;
use std::collections::HashMap;
use std::env;
use subd_db::get_db_pool;
use subd_twitch::rewards::RewardManager;
use twitch_api::helix::{self, points::update_custom_reward};
use twitch_api::HelixClient;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;
use twitch_irc::SecureTCPTransport;
use twitch_irc::TwitchIRCClient;
use twitch_oauth2::{AccessToken, UserToken};

fn get_chat_config() -> ClientConfig<StaticLoginCredentials> {
    let twitch_username = subd_types::consts::get_twitch_bot_username();
    ClientConfig::new_simple(StaticLoginCredentials::new(
        twitch_username,
        Some(subd_types::consts::get_twitch_bot_oauth()),
    ))
}

#[derive(Serialize, Deserialize, Debug)]
struct EventSubRoot {
    subscription: Subscription,
    event: Option<EventSub>,
    challenge: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Subscription {
    id: String,
    status: String,
    #[serde(rename = "type")]
    type_field: String,
    version: String,
    condition: HashMap<String, String>,
    transport: Transport,
    created_at: String,
    cost: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Transport {
    method: String,
    callback: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct EventSub {
    user_id: String,
    user_login: String,
    user_name: String,
    broadcaster_user_id: String,
    broadcaster_user_login: String,
    broadcaster_user_name: String,
    tier: Option<String>,
    is_gift: Option<bool>,
}

#[tokio::main]
async fn main() -> Result<()> {
    {
        use rustrict::{add_word, Type};

        // You must take care not to call these when the crate is being
        // used in any other way (to avoid concurrent mutation).
        unsafe {
            add_word(format!("vs{}", "code").as_str(), Type::PROFANE);
            add_word("vsc*de", Type::SAFE);
        }
    }

    // Advice!
    // codyphobe:
    //           For the OBSClient cloning,
    //           could you pass the OBSClient in the constructor when making event_loop,
    //           then pass self.obsclient into each handler's handle method inside
    //           EventLoop#run

    // Create 1 Event Loop
    // Push handles onto the loop
    // those handlers are things like twitch-chat, twitch-sub, github-sponsor etc.
    let mut event_loop = events::EventLoop::new();

    // You can clone this
    // because it's just adding one more connection per clone()???
    //
    // This is useful because you need no lifetimes
    let pool = subd_db::get_db_pool().await;

    // Turns twitch IRC things into our message events
    event_loop.push(twitch_chat::TwitchChat::new(
        pool.clone(),
        "beginbot".to_string(),
    )?);

    // TODO: Update this description to be more exact
    // Saves the message and extracts out some information
    // for easier routing
    event_loop.push(twitch_chat::TwitchMessageHandler::new(
        pool.clone(),
        twitch_service::Service::new(
            pool.clone(),
            user_service::Service::new(pool.clone()).await,
        )
        .await,
    ));

    let twitch_config = get_chat_config();
    let (_, twitch_client) = TwitchIRCClient::<
        SecureTCPTransport,
        StaticLoginCredentials,
    >::new(twitch_config);

    // This really is named wrong
    // this handles more than OBS
    // and it's also earlier in the program
    // but it takes an obs_client and pool none-the-less
    let obs_client = server::obs::create_obs_client().await?;
    event_loop.push(handlers::obs_messages::OBSMessageHandler {
        obs_client,
        twitch_client,
        pool: pool.clone(),
    });

    // TODO: This should be abstracted
    // Works for Arch Linux
    let (_stream, stream_handle) =
        audio::get_output_stream("pulse").expect("stream handle");

    // Works for Mac
    // let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
    let sink = rodio::Sink::try_new(&stream_handle).unwrap();

    event_loop.push(handlers::sound_handler::SoundHandler {
        sink,
        pool: pool.clone(),
    });

    let sink = rodio::Sink::try_new(&stream_handle).unwrap();
    event_loop.push(handlers::sound_handler::ExplicitSoundHandler {
        sink,
        pool: pool.clone(),
    });

    let pool = get_db_pool().await;
    let twitch_user_access_token =
        env::var("TWITCH_CHANNEL_REWARD_USER_ACCESS_TOKEN").unwrap();

    // Setup the http client to use with the library.
    let reqwest = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let twitch_reward_client: HelixClient<reqwest::Client> = HelixClient::new();

    // let token =
    //     twitch_oauth2::UserToken::from_existing(&reqwest, twitch_user_access_token.into(), None, None)
    //         .await?;
    // let reward_manager = RewardManager::new(
    //     &twitch_reward_client,
    //     &token,
    // );

    // let broadcaster_id = "424038378";
    // let reward_id = "";
    // let request = update_custom_reward::UpdateCustomRewardRequest::new(broadcaster_id, reward_id);
    // let mut body = update_custom_reward::UpdateCustomRewardBody::default();
    // body.cost = Some(420);
    // // // body.title = Some("hydrate but differently now!".into());
    // let response: update_custom_reward::UpdateCustomReward = twitch_reward_client.req_patch(request, body, &token).await?.data;

    // // Elevenlabs/Uberduck handles voice messages
    // let elevenlabs_auth = Auth::from_env().unwrap();
    // let elevenlabs =
    //     Elevenlabs::new(elevenlabs_auth, "https://api.elevenlabs.io/v1/");
    // let sink = rodio::Sink::try_new(&stream_handle).unwrap();
    // let obs_client = server::obs::create_obs_client().await?;
    // let twitch_config = get_chat_config();
    // let (_, twitch_client) = TwitchIRCClient::<
    //     SecureTCPTransport,
    //     StaticLoginCredentials,
    // >::new(twitch_config);
    // event_loop.push(uberduck::ElevenLabsHandler {
    //     pool: pool.clone(),
    //     twitch_client,
    //     sink,
    //     obs_client,
    //     elevenlabs,
    // });

    // AI Scenes
    let elevenlabs_auth = Auth::from_env().unwrap();
    let elevenlabs =
        Elevenlabs::new(elevenlabs_auth, "https://api.elevenlabs.io/v1/");
    let sink = rodio::Sink::try_new(&stream_handle).unwrap();
    let obs_client = server::obs::create_obs_client().await?;
    let twitch_config = get_chat_config();
    let (_, twitch_client) = TwitchIRCClient::<
        SecureTCPTransport,
        StaticLoginCredentials,
    >::new(twitch_config);
    event_loop.push(ai_scenes::AiScenesHandler {
        pool: pool.clone(),
        twitch_client,
        sink,
        obs_client,
        elevenlabs,
    });

    // OBS Hotkeys are controlled here
    let obs_client = server::obs::create_obs_client().await?;
    event_loop.push(handlers::trigger_obs_hotkey::TriggerHotkeyHandler {
        obs_client,
    });

    // OBS Text is controlled here
    let obs_client = server::obs::create_obs_client().await?;
    event_loop.push(handlers::transform_obs_test::TransformOBSTextHandler {
        obs_client,
    });

    // OBS Sources are controlled here
    let obs_client = server::obs::create_obs_client().await?;
    event_loop.push(handlers::source_visibility::SourceVisibilityHandler {
        obs_client,
    });

    // Skyboxes
    let obs_client = server::obs::create_obs_client().await?;
    event_loop.push(handlers::skybox::SkyboxHandler { obs_client });

    // // OBS Stream Characters are controlled here
    let obs_client = server::obs::create_obs_client().await?;
    event_loop.push(
        handlers::stream_character_handler::StreamCharacterHandler {
            obs_client,
        },
    );

    let twitch_config = get_chat_config();
    let (_, twitch_client) = TwitchIRCClient::<
        SecureTCPTransport,
        StaticLoginCredentials,
    >::new(twitch_config);
    event_loop.push(handlers::chatgpt_response_handler::ChatGPTResponse {
        twitch_client,
    });

    // Twitch EventSub Events
    let twitch_config = get_chat_config();
    let (_, twitch_client) = TwitchIRCClient::<
        SecureTCPTransport,
        StaticLoginCredentials,
    >::new(twitch_config);
    let obs_client = server::obs::create_obs_client().await?;
    event_loop.push(handlers::twitch_eventsub_handler::TwitchEventSubHandler {
        pool: pool.clone(),
        obs_client,
        twitch_client,
    });

    let obs_client = server::obs::create_obs_client().await?;
    event_loop.push(handlers::stream_background::StreamBackgroundHandler {
        obs_client,
    });

    let obs_client = server::obs::create_obs_client().await?;
    event_loop.push(handlers::voices_handler::VoicesHandler {
        pool: pool.clone(),
        obs_client,
    });

    let obs_client = server::obs::create_obs_client().await?;
    event_loop.push(handlers::music_scenes_handler::MusicScenesHandler {
        pool: pool.clone(),
        obs_client,
    });
    // =======================================================================

    event_loop.run().await?;
    Ok(())
}
