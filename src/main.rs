use std::collections::{HashMap, HashSet};
use std::env;
use std::sync::Arc;

use serenity::async_trait;
use serenity::client::bridge::gateway::ShardManager;
use serenity::framework::standard::macros::{command, group, help, hook};
use serenity::framework::standard::{
    help_commands,
    Args,
    CommandGroup,
    CommandResult,
    HelpOptions,
    StandardFramework, DispatchError,
};
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::gateway::{GatewayIntents, Ready};
use serenity::model::id::UserId;
use serenity::model::guild::Guild;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use tokio::sync::Mutex;


struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct CommandCounter;

impl TypeMapKey for CommandCounter {
    type Value = HashMap<String, u64>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(roles, some_long_command, assign)]
struct General;

#[group]
#[prefixes("emoji", "em")]
#[description = "Emoji commands!"]
#[summary = "Do emoji shit!"]
#[default_command(dog)]
#[commands(dog)]
struct Emoji;

/* 
#[group]
#[owners_only]
#[only_in(guilds)]
#[summary = "Commands for server owners"]
#[commands(slow_mode)]
struct Owner; */

#[help]
#[command_not_found_text = "Could not find: `{}`."]
#[max_levenshtein_distance(3)]
#[lacking_permissions = "Hide"]
#[lacking_role = "Nothing"]
#[wrong_channel = "Strike"]

async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[hook]
async fn before(ctx: &Context, msg: &Message, command_name: &str) -> bool {
    println!("Got command '{}' byuser '{}'", command_name, msg.author.name);

    let mut data = ctx.data.write().await;
    let counter = data.get_mut::<CommandCounter>().expect("Expected CommandCounter in TypeMap.");
    let entry = counter.entry(command_name.to_string()).or_insert(0);
    *entry += 1;

    true
}

#[hook]
async fn after(_ctx: &Context, _msg: &Message, command_name: &str, command_result: CommandResult) {
    match command_result {
        Ok(()) => println!("Processed command '{}'", command_name),
        Err(why) => println!("Command '{}' returned error {:?}", command_name, why),
    }
}

#[hook]
async fn unknown_command(_ctx: &Context, _msg: &Message, unknown_command_name: &str) {
    println!("Could not find command named '{}'", unknown_command_name);
}

#[hook]
async fn normal_message(_ctx: &Context, msg: &Message) {
    println!("Message is not a command '{}'", msg.content);
}

#[hook]
async fn delay_action(ctx: &Context, msg: &Message) {
    let _ = msg.react(ctx, '⏱').await;
}

#[hook]
async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError, _command_name: &str) {
    if let DispatchError::Ratelimited(info) = error {
        if info.is_first_try {
            let _ = msg
            .channel_id
            .say(&ctx.http, &format!("Try this again in {} seconds.", info.as_secs()))
            .await;
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new(&token);

    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            if let Some(team) = info.team {
                owners.insert(team.owner_user_id);
            } else {
                owners.insert(info.owner.id);
            }
            match http.get_current_user().await {
                Ok(bot_id) => (owners, bot_id.id),
                Err(why) => panic!("Could not access the bot id: {:?}", why),
            }
        },
        Err(why) => panic!("Could not access application info {:?}", why),
    };

    let framework = StandardFramework::new()
    .configure(|c| c
        .with_whitespace(true)
        .on_mention(Some(bot_id))
        .prefix("~")
        .delimiters(vec![", ", ","])
        .owners(owners))

    .before(before)
    .after(after)
    .unrecognised_command(unknown_command)
    .normal_message(normal_message)
    .on_dispatch_error(dispatch_error)
    .bucket("emoji", |b| b.delay(5)).await
    .help(&MY_HELP)
    .group(&EMOJI_GROUP)
    .group(&GENERAL_GROUP);
    //.group(&OWNER_GROUP);

let intents = GatewayIntents::all();
let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<CommandCounter>(HashMap::default())
        .await 
        .expect("Err creating client");

{
    let mut data = client.data.write().await;
    data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
}

if let Err(why) = client.start().await {
    println!("Client error: {:?}", why);
}
}

#[command]
#[description = "Sends an emoji with a dog."]
#[bucket = "emoji"]
async fn dog(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, ":dog:").await?;

    Ok(())
}

#[command]
async fn roles(ctx: &Context, msg: &Message) -> CommandResult {
    dotenv::dotenv().expect("Failed to load .env file");
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let guild_id = 
                env::var("GUILD_ID")
                    .expect("Expected GUILD_ID in environment")
                    .parse()
                    .expect("GUILD_ID must be an integer");
    let http = Http::new(&token);
    let roles = Http::get_guild_roles(&http, guild_id).await.map(|i| i).unwrap();
    let mut role_names_vec: Vec<String> = roles.into_iter()
    .map(|role| role.name)
    .collect();
    role_names_vec.drain(0..1);
    let response = MessageBuilder::new()
            .quote_rest()
            .push(role_names_vec.join("\n"))
            .build();
    msg.reply(ctx, response).await?;
    Ok(())
}

#[command]
async fn some_long_command(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    msg.channel_id.say(&ctx.http, &format!("Arguments: {:?}", args.rest())).await?;

    Ok(())
}

#[command]
async fn assign(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    dotenv::dotenv().expect("Failed to load .env file");
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    if let Some(member) = &msg.member {
        let tmp = member.user.clone();
        let id = tmp.unwrap().id;
        msg.reply(ctx, &format!("{}", id)).await?;
        //let guild = Guild::
        //Guild::member(token, id);
        //
        // let user = Member::add_role(id, token, role_id);
    }
    Ok(())
    
}
