use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::interaction::application_command::CommandDataOption;
use serenity::http::Http;
use std::env;

pub async fn run(_options: &[CommandDataOption], guild_id: u64) -> String {
    dotenv::dotenv().expect("Failed to load .env file");
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let http = Http::new(&token);
    let roles = Http::get_guild_roles(&http, guild_id).await.map(|i| i).unwrap();
    let mut role_names_vec: Vec<String> = roles.into_iter()
    .map(|role| role.name)
    .collect();
    role_names_vec.drain(0..1);
    role_names_vec.join("\n")
}


pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("roles").description("List roles")
}
