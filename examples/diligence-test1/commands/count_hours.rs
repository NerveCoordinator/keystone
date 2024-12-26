use crate::shared;
use shared::*;

use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;

use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::CommandDataOptionValue;

// environment access
use std::env;

// csv access
use csv;

// file system access
use std::fs;

use std::process;
//use rusqlite::{Connection, Result};
use chrono::Utc;
use chrono::TimeZone;
#[derive(Debug)]
struct Task {
    user_id: u64,
    task_description: String,
    duration: f64,
    timestamp: u64,
}

struct Date {
    year: i32,
    month: u32,
    day: u32,
}

/*
fn find_tasks(conn: &Connection, desired_month: Date) -> Vec<Task> {

    // Goal: acquire unix epoch for first of the specified and the first of the month after at 00:00
    let start_dt = Utc.ymd(desired_month.year, desired_month.month, 1).and_hms_milli(0, 0, 0, 0);

    let mut end_month = desired_month.month + 1;
    let mut end_year = desired_month.year;
    if end_month == 13 {
        end_month = 1;
        end_year += 1;
    }
    let end_dt = Utc.ymd(end_year, end_month, 1).and_hms_milli(0, 0, 0, 0);

    let mut tasks = Vec::<Task>::new();
    let mut stmt = match conn.prepare(
        // where did we filter by user again...? Did I even bother lol
        format!("SELECT * FROM tasks WHERE timestamp > {}
         INTERSECT
         SELECT * FROM tasks WHERE timestamp < {}", start_dt.timestamp(), end_dt.timestamp()).as_str(),

    ) {
        Ok(x) => x,
        Err(_) => return tasks,
    };

    let rows = match stmt.query_map((), |row| {
        Ok(Task {
            user_id: row.get(1)?,
            task_description: row.get(2)?,
            duration: row.get(3)?,
            timestamp: row.get(4)?,
        })
    }) {
        Ok(x) => x,
        Err(_) => return tasks,
    };

    for thing in rows {
        tasks.push(thing.expect("task from database in find_tasks failed to push"));
    }

    tasks
}
*/
// returns the discord message to user, which is either the summed hours for specified year and month, or an error.
pub async fn run(interaction: &mut ApplicationCommandInteraction) -> String {
    // get the slash command arguments
    let option = interaction.data.options
        .get(0)
        .expect("Expected user option")
        .resolved
        .as_ref()
        .expect("Expected user object");
    let option2 = interaction.data.options
        .get(1)
        .expect("Expected year option")
        .resolved
        .as_ref()
        .expect("Expected year object");
    let option3 = interaction.data.options
        .get(2)
        .expect("Expected month option")
        .resolved
        .as_ref()
        .expect("Expected month object");

    if let CommandDataOptionValue::User(user, _) = option{
        if let CommandDataOptionValue::Integer(year) = option2{
            if let CommandDataOptionValue::Integer(month) = option3{

                tracing::debug!("count hours");

                let userid = user.id.0;
                let message: SqliteMessage = SqliteMessage::CountHours(
                    userid,
                    *year,
                    *month
                );
                tracing::debug!("message {:?}", &message);
                // Send message using TX
                let mut tx_option = shared::TX.lock().await; // Lock and hold the result
                if let Some(ref tx) = *tx_option { // Pattern match to get the Sender
                    if let Err(e) = tx.send(message).await {
                        tracing::error!("Failed to send message: {:?}", e);
                    } else {
                        tracing::debug!("Message sent successfully");
                    }
                } else {
                    tracing::error!("TX is not initialized");
                }

                let mut message_to_user = "default message to user".to_string();
                // Receive message using RX2
                let mut rx2_option = shared::RX2.lock().await; // Lock and hold the result
                if let Some(ref mut rx2) = *rx2_option { // Pattern match to get the Receiver
                    if let Some(message) = rx2.recv().await {
                        tracing::debug!("Received in count hours: {:?}", message);
                        match message {
                            SqliteMessage::CapnpResultString(capnp_result) => {
                                tracing::debug!("Received CapnpResult: unformatable");
                                match capnp_result {
                                    Ok(sum) => {
                                        /*
                                        let tasks = find_tasks(&conn,Date{year:*year as i32,month:*month as u32,day:0});
                                        let mut sum = 0.0;
                                        for task in tasks {
                                            if task.user_id == userid {
                                                sum += task.duration;
                                            }
                                        }*/
                                        //return sum.to_string();
                                        message_to_user = format!("sum for user X at time range Y was: Z");
                                        return (sum);
                                    },
                                    Err(e) => {
                                        message_to_user = format!("log failed, error was: {}", e);
                                        return ("1".to_string());

                                    }
                                }
                            }
                        SqliteMessage::Fail() => {
                            tracing::debug!("Received Fail()");
                            message_to_user = format!("SqliteMessage::Fail() error, cannot compute sum. Sorry!");

                        }
                        _ => {
                            tracing::debug!("Received ?: {:?}", &message);
                            message_to_user = format!("Unknown error {:?}, cannot compute sum. Sorry!", message);
                        }
                        }
                    } else {
                        tracing::debug!("Channel closed before receiving a message");
                    }
                } else {
                    tracing::error!("RX2 is not initialized");
                }

                /*
                let conn = match Connection::open(&log_path) {
                    Ok(x) => x,
                    Err(e) => return format!("log doesn't exist: {}, {}", &log_path, e),
                };
                */

            }
        }
    }
    return "unknown error".to_string();
}

// declare slash command arguments and constraints such as type or range
pub fn register(command: &mut CreateApplicationCommand,) -> &mut CreateApplicationCommand {
    command
        .name("count_hours")
        .description("sum hours worked for user for a given month")
        .create_option(|option| {
            option
                .name("user")
                .description("User to sum hours for")
                .kind(CommandOptionType::User)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("year")
                .description("Year to sum hours for")
                .kind(CommandOptionType::Integer)
                .min_number_value(2022.0)
                .max_number_value(2030.0)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("month")
                .description("Month to sum hours for")
                .kind(CommandOptionType::Integer)
                .min_number_value(0.0)
                .max_number_value(12.0)
                .required(true)
        })
}
