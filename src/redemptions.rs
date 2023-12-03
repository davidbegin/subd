use anyhow::Result;
use sqlx::PgPool;
use sqlx::types::Uuid;
use subd_macros::database_model;

#[database_model]
pub mod redemptions {
    use super::*;

    pub struct Model {
        pub title: String,
        pub cost: i32,
        pub user_name: String,
        pub reward_id: Uuid,
        // This might need to be text
        // optional might FUCKING US 
        pub user_input: Option<String>,
    }
}

impl redemptions::Model {
    #[allow(dead_code)]

    pub async fn save(self, pool: &PgPool) -> Result<Self> {
        
        Ok(sqlx::query_as!(
            Self,
            r#"
            INSERT INTO redemptions 
            (title, cost, user_name, reward_id, user_input)
            VALUES ( $1, $2, $3, $4, $5)
            RETURNING title, cost, user_name, reward_id, user_input
        "#,
            self.title,
            self.cost,
            self.user_name,
            self.reward_id,
            self.user_input
        )
        .fetch_one(pool)
        .await?)
    }
}
//
// pub async fn turn_off_global_voice(pool: &PgPool) -> Result<()> {
//     let _res =
//         sqlx::query!("UPDATE twitch_stream_state SET global_voice = $1", false)
//             .execute(pool)
//             .await?;
//
//     Ok(())
// }
//
