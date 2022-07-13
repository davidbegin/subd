use serde::{Deserialize, Serialize};
pub use twitch_api2::pubsub::channel_subscriptions::ChannelSubscribeEventsV1Reply;
use twitch_irc::message::PrivmsgMessage;

pub type UserID = i64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    // Info
    TwitchChatMessage(PrivmsgMessage),
    TwitchSubscriptionCount(usize),
    TwitchSubscription(TwitchSubscriptionEvent),
    GithubSponsorshipEvent,

    // UserEvents
    ThemesongDownload(ThemesongDownload),
    ThemesongPlay(ThemesongPlay),

    // Requests
    RequestTwitchSubCount,

    // Control
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThemesongDownload {
    Request { msg: PrivmsgMessage },
    Start { display_name: String },
    Finish { display_name: String, success: bool },
    Format { sender: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemesongPlay {
    pub user_id: UserID,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubUser {
    pub id: String,
    pub login: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserRoles {
    pub is_github_sponsor: bool,
    pub is_twitch_mod: bool,
    pub is_twitch_vip: bool,
    pub is_twitch_founder: bool,
    pub is_twitch_sub: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TwitchSubscriptionEvent {
    subscription: ChannelSubscribeEventsV1Reply,
}

impl From<ChannelSubscribeEventsV1Reply> for TwitchSubscriptionEvent {
    fn from(subscription: ChannelSubscribeEventsV1Reply) -> Self {
        Self { subscription }
    }
}

impl TwitchSubscriptionEvent {
    pub fn display_name(&self) -> String {
        match &self.subscription {
            ChannelSubscribeEventsV1Reply::Sub(sub) => &sub.display_name,
            ChannelSubscribeEventsV1Reply::ReSub(sub) => &sub.display_name,
            ChannelSubscribeEventsV1Reply::SubGift(sub) => &sub.display_name,
            ChannelSubscribeEventsV1Reply::ResubGift(sub) => &sub.display_name,
            ChannelSubscribeEventsV1Reply::ExtendSub(sub) => &sub.display_name,
            _ => unimplemented!(),
        }
        .to_string()
    }
}

pub fn get_nyx_sub_for_begin() -> TwitchSubscriptionEvent {
    let message = r##"
{
    "benefit_end_month": 11,
    "user_name": "nyxkrage",
    "display_name": "NyxKrage",
    "channel_name": "beginbot",
    "user_id": "1234",
    "channel_id": "424038378",
    "time": "2020-10-20T22:17:43.242793831Z",
    "sub_message": {
        "message": "You are my favorite streamer",
        "emotes": null
    },
    "sub_plan": "1000",
    "sub_plan_name": "Channel Subscription (emilgardis)",
    "months": 0,
    "cumulative_months": 1,
    "context": "sub",
    "is_gift": false,
    "multi_month_duration": 0
}
"##;

    let subscription = serde_json::from_str(message).unwrap();
    TwitchSubscriptionEvent { subscription }
}

// const MY_CHANNEL: UserId = 114257969;

pub fn get_nyx_sub() -> TwitchSubscriptionEvent {
    let message = r##"
{
    "benefit_end_month": 11,
    "user_name": "nyxkrage",
    "display_name": "NyxKrage",
    "channel_name": "teej_dv",
    "user_id": "1234",
    "channel_id": "27620241",
    "time": "2020-10-20T22:17:43.242793831Z",
    "sub_message": {
        "message": "You are my favorite streamer",
        "emotes": null
    },
    "sub_plan": "1000",
    "sub_plan_name": "Channel Subscription (emilgardis)",
    "months": 0,
    "cumulative_months": 1,
    "context": "sub",
    "is_gift": false,
    "multi_month_duration": 0
}
"##;

    let subscription = serde_json::from_str(message).unwrap();
    TwitchSubscriptionEvent { subscription }
}

pub fn get_prime_sub() -> TwitchSubscriptionEvent {
    let message = r##"
{
    "benefit_end_month": 11,
    "user_name": "theprimeagen",
    "display_name": "ThePrimeagen",
    "channel_name": "teej_dv",
    "user_id": "1234",
    "channel_id": "27620241",
    "time": "2020-10-20T22:17:43.242793831Z",
    "sub_message": {
        "message": "You are my favorite streamer",
        "emotes": null
    },
    "sub_plan": "1000",
    "sub_plan_name": "Channel Subscription (emilgardis)",
    "months": 0,
    "cumulative_months": 1,
    "context": "sub",
    "is_gift": false,
    "multi_month_duration": 0
}
"##;

    let subscription = serde_json::from_str(message).unwrap();
    TwitchSubscriptionEvent { subscription }
}
