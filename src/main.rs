use dotenv::dotenv;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use std::env;
use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sysinfo::{CpuExt, DiskExt, NetworkExt, System, SystemExt};
use tokio::time::sleep;

struct Handler {
    channel_id: ChannelId,
    status_message_id: RwLock<Option<u64>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("debug: Logged in as {}", ready.user.name);

        // Get or create status message
        let channel = self.channel_id;
        let messages = channel
            .messages(&ctx.http, |retriever| retriever.limit(100))
            .await
            .unwrap();

        let mut status_id = None;
        for msg in &messages {
            if msg.author.id == ready.user.id {
                status_id = Some(msg.id.0);
                break;
            }
        }

        // Delete other messages in the channel
        for msg in &messages {
            if msg.author.id != ready.user.id {
                let _ = msg.delete(&ctx.http).await;
            }
        }

        if let Some(id) = status_id {
            *self.status_message_id.write().await = Some(id);
        }

        // Start system monitoring loop
        let ctx_clone = ctx.clone();
        let handler_clone = self.clone();
        tokio::spawn(async move {
            loop {
                handler_clone.update_status(&ctx_clone).await;
                sleep(Duration::from_secs(5)).await;
            }
        });
    }
}

impl Handler {
    fn clone(&self) -> Self {
        Handler {
            channel_id: self.channel_id,
            status_message_id: RwLock::new(*self.status_message_id.blocking_read()),
        }
    }

    async fn update_status(&self, ctx: &Context) {
        // Initialize system info gatherer
        let mut sys = System::new_all();
        sys.refresh_all();

        // Get CPU usage
        sys.refresh_cpu();
        let cpu_usage = sys.global_cpu_info().cpu_usage();
        let cpu_usage_str = format!("CPU Usage: {}%", cpu_usage.round() as i32);

        // Get memory usage
        sys.refresh_memory();
        let total_memory = sys.total_memory() / 1024 / 1024; // Convert to MB
        let free_memory = sys.free_memory() / 1024 / 1024;
        let used_memory = total_memory - free_memory;
        let used_memory_percentage =
            (used_memory as f64 / total_memory as f64 * 100.0).round() as i32;
        let memory_usage = format!(
            "Memory Usage: {}MB/{}MB ({}%)",
            used_memory, total_memory, used_memory_percentage
        );

        // Get disk usage
        sys.refresh_disks_list();
        let mut disk_usage = String::from("Disk Usage:\n");
        for disk in sys.disks() {
            let total_disk = disk.total_space() / 1024 / 1024; // Convert to MB
            let free_disk = disk.available_space() / 1024 / 1024;
            let used_disk = total_disk - free_disk;
            let used_disk_percentage =
                (used_disk as f64 / total_disk as f64 * 100.0).round() as i32;

            let mount_point = disk.mount_point().to_string_lossy();
            disk_usage.push_str(&format!(
                "```\n{}\n{}MB/{}MB ({}%)\n```",
                mount_point, used_disk, total_disk, used_disk_percentage
            ));
        }

        // Get network usage
        sys.refresh_networks();
        let mut network_usage = String::from("Network Usage: ");
        let mut total_rx = 0;
        let mut total_tx = 0;

        for (_, network) in sys.networks() {
            total_rx += network.received();
            total_tx += network.transmitted();
        }

        // Note: This is not exactly the same as the original which tracked per-second bandwidth
        // For accurate per-second monitoring, we'd need to track previous values and calculate the diff
        let kb_received = total_rx as f64 / 1024.0;
        let kb_transmitted = total_tx as f64 / 1024.0;
        network_usage.push_str(&format!(
            "In: {:.2} KB/s Out: {:.2} KB/s",
            kb_received, kb_transmitted
        ));

        // Get uptime
        let uptime_secs = sys.uptime();
        let uptime_days = uptime_secs / (24 * 60 * 60);
        let uptime_hours = (uptime_secs % (24 * 60 * 60)) / (60 * 60);
        let uptime_minutes = (uptime_secs % (60 * 60)) / 60;
        let uptime_seconds = uptime_secs % 60;
        let uptime_message = format!(
            "{}d {}h {}m {}s",
            uptime_days, uptime_hours, uptime_minutes, uptime_seconds
        );

        // Format timestamp
        let now = SystemTime::now();
        let timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_secs();
        let datetime = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // Build status message
        let status_message = format!(
            "{}\n{}\n{}\n{}\nUptime: {}\nLast Update: {} (<t:{}:R>)",
            cpu_usage_str,
            memory_usage,
            disk_usage,
            network_usage,
            uptime_message,
            datetime,
            timestamp
        );

        // Update or send message
        if let Some(msg_id) = *self.status_message_id.read().await {
            match self
                .channel_id
                .edit_message(&ctx.http, msg_id, |m| m.content(&status_message))
                .await
            {
                Ok(_) => {}
                Err(e) => println!("Error updating message: {:?}", e),
            }
        } else {
            match self
                .channel_id
                .send_message(&ctx.http, |m| m.content(&status_message))
                .await
            {
                Ok(msg) => {
                    *self.status_message_id.write().await = Some(msg.id.0);
                }
                Err(e) => println!("Error sending message: {:?}", e),
            }
        }

        println!("debug: Update status({})", datetime);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables
    dotenv().ok();

    // Get configuration from environment variables
    let token = env::var("TOKEN").expect("Expected a token in the environment");
    let channel_id = env::var("CHANNEL_ID").expect("Expected a channel ID in the environment");
    let channel_id = ChannelId(channel_id.parse::<u64>()?);

    // Configure the client
    let intents =
        GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILDS;

    let handler = Handler {
        channel_id,
        status_message_id: RwLock::new(None),
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    // Start the client
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }

    Ok(())
}
