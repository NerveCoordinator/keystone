use crate::shared;
use shared::*;

use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;

use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::CommandDataOptionValue;

// environment access
use std::env;

// time and date
use chrono::prelude::*;

// csv and file system access
use std::fs;
use std::path::Path;
use std::fs::OpenOptions;
use std::io::Write;
use csv;

use std::process;
//use rusqlite::{Connection, Result};
use std::time::{SystemTime, UNIX_EPOCH};
#[derive(Debug)]
struct Task {
    user_id: u64,
    task_description: String,
    duration: f64,
    timestamp: u64,
}

/*
fn add_task(conn: &Connection, task:Task) -> Result<usize> {
    println!("{:?}", task);
    conn.execute(
        "INSERT INTO tasks (user_id, task_description, duration, timestamp) values (?1, ?2, ?3, ?4)",
        (task.user_id, task.task_description, task.duration, task.timestamp),
    )
}
*/

// returns tuple (discord message to user, productivity thread data from user supplied input)
pub async fn run(interaction: &mut ApplicationCommandInteraction, test:String) -> (String, String) {
    // get the slash command arguments
    let option = interaction.data.options
        .get(0)
        .expect("Expected user option")
        .resolved
        .as_ref()
        .expect("Expected user object");
    let option2 = interaction.data.options
        .get(1)
        .expect("Expected user option")
        .resolved
        .as_ref()
        .expect("Expected user object");


    tracing::debug!("inside log task");

    // extract arguments and then use them to construct a message to the user and append to database
    if let CommandDataOptionValue::String(user_task_description) = option{
        let mut message_to_user = std::string::String::from(""); // success message to send over discord
        let mut productivity_thread_msg = std::string::String::from(""); // content to put in productivity thread
        if let CommandDataOptionValue::Number(task_hours) = option2{
            // get the user's ID and name from the slash command data
            let (user_id, user_name) = match &interaction.member {
                Some(member) => (member.user.id.0, member.user.name.to_string()),
                _ => return (format!("Could not find member. \ntask description: {}\nduration: {}", user_task_description, task_hours), "".to_string()),
            };
            // message formatting
            let user_input = format!("\nuser name: {} \n user id: {} \ntask description: {}\n duration: {}", user_name, user_id, user_task_description, task_hours);
            message_to_user = format!("successfully logged {}", user_input);
            productivity_thread_msg = user_input.clone();

            // displayed to the user whenever there is a problem, it would be nice if this was saved to a log as well.
            let report_error_str = format!("\nuser input: {}\nyour input has not been saved.", user_input);

            tracing::debug!("inside log task");
            let message: SqliteMessage = SqliteMessage::LogTask(
                user_id,
                user_task_description.clone(),
                *task_hours,
                SystemTime::now().duration_since(UNIX_EPOCH).expect("Error: system time before 1970").as_secs()
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

            let user_input = format!("\nuser name: {} \ntask description: {}\n duration: {}", user_name, user_task_description, task_hours);
            let mut message_to_user = "default message to user".to_string();
            let mut productivity_thread_msg = "default productivity thread message".to_string();
            // Receive message using RX2
            let mut rx2_option = shared::RX2.lock().await; // Lock and hold the result
            if let Some(ref mut rx2) = *rx2_option { // Pattern match to get the Receiver
                if let Some(message) = rx2.recv().await {
                    tracing::debug!("Received in count hours: {:?}", message);
                    match message {
                        SqliteMessage::CapnpResultEmpty(capnp_result) => {
                            tracing::debug!("Received CapnpResult");
                            match capnp_result {
                                Ok(_) => {
                                    message_to_user = "successfully logged".to_string();
                                    productivity_thread_msg = user_input.clone();
                                },
                                Err(e) => {
                                    message_to_user = format!("log failed, error was: {}", e);
                                    productivity_thread_msg = user_input.clone();

                                }
                            }
                        }
                        SqliteMessage::Fail() => {
                            tracing::debug!("Received Fail()");
                            message_to_user = format!("SqliteMessage::Fail() error, input probably not saved. Sorry!");
                            productivity_thread_msg = "".to_string();

                        }
                        _ => {
                            tracing::debug!("Received ?: {:?}", &message);
                            message_to_user = format!("Unknown error {:?}, input probably not saved. Sorry!", message);
                            productivity_thread_msg = "".to_string();
                        }
                    }
                } else {
                    tracing::debug!("Channel closed before receiving a message");
                }
            } else {
                tracing::error!("RX2 is not initialized");
            }
            /*
            match add_task(&conn, new_task){
                Ok(_) => 1+1,
                Err(e) => return(format!("Error adding task: {} {}", e, report_error_str), "".to_string()),
            };
            */

            message_to_user = format!("{} input was: {}", message_to_user, user_input);
        }
        return (message_to_user, productivity_thread_msg)
    }
    else {
        return (format!("Unknown error, input not saved. Sorry!"), "".to_string());
    }
    return (format!("Unknown success, input only maybe saved. Sorry!"), "".to_string());
}

// declare slash command arguments and constraints such as type or range
pub fn register(command: &mut CreateApplicationCommand,) -> &mut CreateApplicationCommand {
    command
        .name("log_task")
        .description("laydown prototype")
        .create_option(|option| {
            option
                .name("description")
                .description("The task that was done")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("duration")
                .description("The hours the task took today")
                .kind(CommandOptionType::Number)
                .min_number_value(0.0)
                .max_number_value(24.0)
                .required(true)
        })
}
