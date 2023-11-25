use anyhow::Result;
use std::thread;
use std::time;
use std::time::Duration;
use async_trait::async_trait;
use events::EventHandler;
use subd_types::Event;
use subd_types::ElevenLabsRequest;

use twitch_chat::send_message;

use std::time::SystemTime;
use tokio::fs;
use tokio::io::AsyncReadExt;

use tokio::sync::broadcast;
use twitch_irc::{TwitchIRCClient, SecureTCPTransport, login::StaticLoginCredentials};

pub struct ChatGPTResponse {
    pub twitch_client: TwitchIRCClient<SecureTCPTransport, StaticLoginCredentials>,
}

#[async_trait]
impl EventHandler for ChatGPTResponse {
    async fn handle(
        self: Box<Self>,
        tx: broadcast::Sender<Event>,
        _rx: broadcast::Receiver<Event>,
    ) -> Result<()> {
        let response_file = "/home/begin/code/BeginGPT/tmp/current/chatgpt_response.txt";
        let metadata = fs::metadata(response_file).await.unwrap();
        let mut last_modified = metadata.modified().unwrap();
        
        loop {
            let metadata = fs::metadata(response_file).await.unwrap();
            let current_modified = metadata.modified().unwrap();
            println!("Current Modified: {:?}", current_modified);
            
            if current_modified > last_modified {
                let mut file = fs::File::open(response_file).await.unwrap();
                
                let mut contents = String::new();
                let _ = file.read_to_string(&mut contents).await;

                let voice_text = contents.clone();
                let _ = tx.send(Event::ElevenLabsRequest(subd_types::ElevenLabsRequest{
                    source: Some("begin".to_string()),
                    voice_text,
                    message: contents,
                    username: "beginbot".to_string(),

                    // This isn't being respected
                    voice: "ethan".to_string(),
                }));

                println!("New Current Modified: {:?}", current_modified);
                last_modified = current_modified;
                
            }
            let sleep_time = time::Duration::from_millis(1000);
            thread::sleep(sleep_time);
        }
    }
}
