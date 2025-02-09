#![allow(unused)]
// too many warnings lol

// make rust error if I forget a .await
#![deny(unused_must_use)]

include!(concat!(env!("OUT_DIR"), "/capnproto.rs"));



use capnp::any_pointer::Owned as any_pointer;
use capnp::capability::FromClientHook;
use keystone::sqlite_capnp::add_d_b;
use keystone::sqlite_capnp::d_b_any;
use keystone::sqlite_capnp::database;
use keystone::sqlite_capnp::expr::table_column::TableColumn;
use keystone::sqlite_capnp::expr::Expr;
use keystone::sqlite_capnp::insert;
use keystone::sqlite_capnp::insert::source;
use keystone::sqlite_capnp::join_clause;
use keystone::sqlite_capnp::r_a_table_ref;
use keystone::sqlite_capnp::r_o_database;
use keystone::sqlite_capnp::r_o_table_ref;
use keystone::sqlite_capnp::select;
use keystone::sqlite_capnp::select::ordering_term::OrderingTerm;
use keystone::sqlite_capnp::select_core;
use keystone::sqlite_capnp::table_field;
use keystone::sqlite_capnp::table_or_subquery;
use keystone::storage_capnp::cell;
use diligence_capnp::root as dil_root;
use tracing::Level;

use std::env;
use std::time::Duration;

// Discord
mod commands;
use serenity::async_trait;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::prelude::*;
use serenity::model::id::{ChannelId, RoleId}; //, GuildId, MessageId};
use serenity::model::channel::Message;
use serenity::utils::MessageBuilder;


use std::sync::Mutex;
use std::sync::Arc;
use once_cell::sync::Lazy;
// laydown message list needs to be a global because it is accessed across threads
static LAYDOWN_MESSAGES: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(vec![]));

static KILL_SENDER: Lazy<Mutex<Option<oneshot::Sender<()>>>> = Lazy::new(|| Mutex::new(None));

use tokio::task::JoinHandle;
static SPIN_HANDLE: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));

fn set_spin_handle(handle: JoinHandle<()>) {
    let mut lock = SPIN_HANDLE.lock().unwrap();
    *lock = Some(handle);
}

fn take_spin_handle() -> Option<JoinHandle<()>> {
    let mut lock = SPIN_HANDLE.lock().unwrap();
    lock.take()
}

fn set_kill_sender(sender: oneshot::Sender<()>) {
    let mut lock = KILL_SENDER.lock().unwrap();
    *lock = Some(sender);
}

fn take_kill_sender() -> Option<oneshot::Sender<()>> {
    let mut lock = KILL_SENDER.lock().unwrap();
    lock.take() // Take the value, leaving None in its place
}



pub mod shared {
    //pub type SqliteMessage = (i32, String, f64, i32);
    #[derive(Debug)]
    pub enum SqliteMessage {
        LogTask(u64, String, f64, u64),
        CountHours(u64, i64, i64),
        CapnpResultEmpty(capnp::Result<()>),
        CapnpResultString(capnp::Result<(String)>),
        Fail(),
    }
    pub use tokio::sync::mpsc::{Sender, Receiver};
    use tokio::sync::Mutex as TokioMutex;
    use std::sync::Mutex;
    use std::sync::Arc;
    use once_cell::sync::Lazy;
    pub static TX: Lazy<Arc<TokioMutex<Option<Sender<SqliteMessage>>>>> = Lazy::new(|| {
        Arc::new(TokioMutex::new(None))
    });

    pub static RX2: Lazy<Arc<TokioMutex<Option<Receiver<SqliteMessage>>>>> = Lazy::new(|| {
        Arc::new(TokioMutex::new(None))
    });

    // Technically these don't need to be this since they're just used in a loop but here they are
    pub static RX: Lazy<Arc<TokioMutex<Option<Receiver<SqliteMessage>>>>> = Lazy::new(|| {
        Arc::new(TokioMutex::new(None))
    });
    pub static TX2: Lazy<Arc<TokioMutex<Option<Sender<SqliteMessage>>>>> = Lazy::new(|| {
        Arc::new(TokioMutex::new(None))
    });
}


use shared::*;

// Function to initialize the global channels
pub async fn init_global_channels(tx: Sender<SqliteMessage>, tx2: Sender<SqliteMessage>, rx: Receiver<SqliteMessage>, rx2: Receiver<SqliteMessage>) {
    let mut tx_lock = shared::TX.lock().await;
    *tx_lock = Some(tx);

    let mut tx2_lock = shared::TX2.lock().await;
    *tx2_lock = Some(tx2);

    let mut rx_lock = shared::RX.lock().await;
    *rx_lock = Some(rx);

    let mut rx2_lock = shared::RX2.lock().await;
    *rx2_lock = Some(rx2);
}

//static diligence_mutex: Lazy<Mutex<DiligenceImpl>> = Lazy::new(|| Mutex::new(DiligenceImpl::new()));
//static diligence_mutex: Lazy<Arc<Mutex<DiligenceImpl>>> = Lazy::new(|| Arc::new(Mutex::new(DiligenceImpl::new())));
                                                                    //
// create discord productivity hour thread
// msg contains the task descriptions and durations
async fn make_thread(ctx : &Context, msg : &String){
        // The thread has two parts:
        // * A declaration where we we ping people who signed up for it
        // * A reply which contains task descriptions and durations

        tracing::debug!("inside make_thread");
        let role_id_num = match env::var("ROLE_ID").expect("Expected a role ID in the environment").parse::<u64>(){
            Ok(id) => id,
            Err(why) => {
                tracing::debug!("Error getting role ID from environment: {:?}", why);
                tracing::debug!("Cannot make thread, returning.");
                return;
            }
        };
        let channel_id_num = match env::var("CHANNEL_ID").expect("Expected a channel ID in the environment").parse::<u64>() {
            Ok(id) => id,
            Err(why) => {
                tracing::debug!("Error getting channel ID from environment: {:?}", why);
                tracing::debug!("Cannot make thread, returning.");
                return;
            }
        };

        // construct thread declaration
        let pingable = RoleId(role_id_num);
        let response = MessageBuilder::new()
            .push("productivity thread! ")
            .mention(&pingable)
            .build();

        // Construct reply
        let user_tasks;
        if msg == "" {
            user_tasks = "There were no logged tasks.".to_string();
        }
        else {
            user_tasks = msg.to_string();
        }
        let response2 = MessageBuilder::new()
            .push(&user_tasks)
            .build();

        tracing::debug!("make_thread before posting");
        // Post both the thread and its reply
        match ChannelId(channel_id_num).say(&ctx, &response).await {
            Err(why) => tracing::debug!("Error posting productivity thread declaration: {:?}", why),
            Ok(Message{id, .. }) =>
                match ChannelId(channel_id_num).create_public_thread(&ctx, id, |b| b.name("productivity hour")).await {
                    Err(why) => tracing::debug!("Error making productivity thread: {:?}", why),
                    Ok(thread_id) => {
                        if let Err(why2) = thread_id.say(&ctx, &response2).await {
                            tracing::debug!("Error posting productivity thread reply: {:?}", why2);
                        };
                    },
                },
        }
        tracing::debug!("make_thread after posting");
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // slash commands
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        tracing::debug!("after 2");

        if let Interaction::ApplicationCommand(mut command) = interaction {
            // slash commands are called here
            let content = match command.data.name.as_str() {
                "log_task" => {
                    let log_result = commands::log_task::run(&mut command, "test".to_string()).await;
                    // .0 is success or fail message for user, .1 is the user's input formatted for productivity thread
                    // record user's input for later posting to productivity thread
                    LAYDOWN_MESSAGES.lock().unwrap().push(log_result.1);
                    log_result.0
                },
                "count_hours" => commands::count_hours::run(&mut command).await,
                _ => "slash command not implemented".to_string(),
            };
            // report success/fail to user
            if let Err(e) = command.create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                }).await {
                    tracing::debug!("Cannot respond to slash command: {}", e);
            }
        }
    }

    // fires on boot and sets up slash commands and the repeating thread
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::debug!("{} is connected!", ready.user.name);
        let guild_id = GuildId(
            env::var("GUILD_ID")
                .expect("Expected GUILD_ID in environment")
                .parse()
                .expect("GUILD_ID must be an integer"),
        );

        // register commands
        match GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands
                .create_application_command(|command| commands::log_task::register(command))
                .create_application_command(|command| commands::count_hours::register(command))
        })
        .await {
            Ok(_) => (),
            Err(e) => tracing::debug!("Error registering commands: {}", e),
        }
        let _ = tokio::time::sleep(Duration::from_secs(3)).await;
        tracing::debug!("just before loop starts");
        // spawn tokio thread to post productivity thread regularly
        let ctx2 = Context::clone(&ctx);
        tokio::spawn(async move {
            tracing::debug!("just after loop starts");
            loop {
                // combine all messages since last thread into a single string
                let mut msg: String = "".to_owned();
                  for item in LAYDOWN_MESSAGES.lock().unwrap().iter() {
                    msg.push_str(item);
                    msg.push_str(&"\n".to_string());
                }

                // empty temporary message storage
                while LAYDOWN_MESSAGES.lock().unwrap().len() > 0 {
                    LAYDOWN_MESSAGES.lock().unwrap().remove(0);
                }

                tracing::debug!("before making thread");
                // create the thread and post the messages
                make_thread(&ctx2, &msg).await;
                tracing::debug!("after making thread?");

                // wait for 24 hours before posting again
                tokio::time::sleep(Duration::from_secs(24 * 60 * 60)).await;
            }
        });
    }
}


async fn run_discord_server() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    //let data_path = env::var("DATA_PATH").expect("Expected a log path in the environment");
    /*
    let conn = match Connection::open(format!("{}/tasks.db", data_path)) {
        Ok(x) => x,
        Err(e) => {tracing::debug!("failed to open database: {}", e); process::exit(0x0100)},
    };
    match conn.execute(
        "create table if not exists tasks (
            id integer primary key,
            user_id integer not null,
            task_description text not null,
            duration float not null,
            timestamp integer not null
        )",
        (),
    ){
        Ok(_) => tracing::debug!("created table if needed"),
        Err(_) => tracing::debug!("failed to create table?"),
    };
    let _ = conn.close();
    */

    tracing::debug!("create");

    let mut client = Client::builder(token, GatewayIntents::empty())
        .event_handler(Handler)// {tx: tx.clone()})
        .await
        .expect("Error creating client");

    tracing::debug!("post-running");
    if let Err(why) = client.start().await {
        tracing::debug!("Client error: {:?}", why);
    }
}

use std::process;
//use rusqlite::{Connection, Result};
use std::collections::HashMap;

/*
struct DiligenceImpl {
    pub inner: Option<Arc<keystone::sqlite_capnp::table::Client>>,
    pub outer: Option<Arc<keystone::storage_capnp::cell::Client<keystone::sqlite_capnp::table_ref::Owned>>>,
    pub sqlite: Option<Arc<keystone::sqlite_capnp::root::Client>>,
}
 */


use chrono::Utc;
use chrono::TimeZone;

fn get_timestamps_from_year_month(desired_year: i32, desired_month: u32) -> (i64, i64){
    let start_dt = Utc.ymd(desired_year, desired_month, 1).and_hms_milli(0, 0, 0, 0);

    let mut end_month = desired_month + 1;
    let mut end_year = desired_year;
    if end_month == 13 {
        end_month = 1;
        end_year += 1;
    }
    let end_dt = Utc.ymd(end_year, end_month, 1).and_hms_milli(0, 0, 0, 0);
    return (start_dt.timestamp() as i64, end_dt.timestamp() as i64);
}
// cannot be in DiligenceImpl because we need to call it from new so we don't actually have access to that yet.
async fn diligence_main() -> () {
    _ = 1+1;
}




///////////////////////////////////////////////////////////////////////////////////

pub struct DiligenceImpl {
    //pub outer: cell::Client<keystone::sqlite_capnp::table_ref::Owned>,
    pub inner: keystone::sqlite_capnp::table::Client,
    pub sqlite: keystone::sqlite_capnp::root::Client,
    kill_tx: RefCell<Option<oneshot::Sender<()>>>,
    spin_handle: RefCell<Option<JoinHandle<()>>>,
}
impl DiligenceImpl {
    async fn sqlite_insert_task(&self, user_id:String, description:String, duration:f64, timestamp:i64) -> capnp::Result<(())>{
        let ins = self
            .sqlite
            .clone()
            .cast_to::<database::Client>()
            .build_insert_request(Some(insert::Insert {
                _fallback: insert::ConflictStrategy::Fail,
                _target: self
                    .inner
                    .clone()
                    .cast_to::<r_a_table_ref::Client>()
                    .clone(),
                _source: source::Source::_Values(vec![
                    vec![
                        d_b_any::DBAny::_Text(user_id),
                        d_b_any::DBAny::_Text(description),
                        d_b_any::DBAny::_Real(duration),
                        d_b_any::DBAny::_Integer(timestamp),
                    ],

                ]),
                _cols: vec![
                    "user_id".to_string(),
                    "task_description".to_string(),
                    "duration".to_string(),
                    "timestamp".to_string(),
                ],
                _returning: Vec::new(),
            }))
            .send()
            .promise
            .await?;

        ins.get()?;
        Ok(())
    }
    async fn sqlite_select_tasks(&self, user_id:String, year:i64, month: i64) -> capnp::Result<(String)>{
        tracing::debug!("sqlite_select_tasks inputs: {} {} {}", &user_id, &year, &month);
        tracing::debug!("before ratable");
        let ra_table_ref_cap = self.inner
            .adminless_request()
            .send()
            .promise
            .await?
            .get()?
            .get_res()?
            .appendonly_request()
            .send()
            .promise
            .await?
            .get()?
            .get_res()?;

        tracing::debug!("before rotable");
        let ro_tableref_cap = ra_table_ref_cap
            .readonly_request()
            .send()
            .promise
            .await?
            .get()?
            .get_res()?;

        tracing::debug!("before build_select");
        let (start_dt, end_dt) = get_timestamps_from_year_month(year as i32, month as u32);
        let res_stream = match self
            .sqlite
            .clone()
            .cast_to::<r_o_database::Client>()
            .send_request_from_sql(
                "SELECT * FROM ?0 WHERE ?1 BETWEEN ?2 AND ?3",
                vec![
                    Bindings::ROTableref(ro_tableref_cap.clone()),
                    Bindings::Column(Col::new(0, "timestamp".to_string())),
                    Bindings::DBAny(DBAnyBindings::_Integer(start_dt)),
                    Bindings::DBAny(DBAnyBindings::_Integer(end_dt)),
                ],
            ) {
            Ok(request) => request.promise.await?.get()?.get_res()?,
            Err(e) => {
                tracing::debug!("Error sending request: {:?}", e);
                return Err(capnp::Error::failed(format!("Error occurred: {:?}", e)));
            }
        };

        tracing::debug!("after build_select");


        let mut next_request = res_stream.next_request();
        next_request.get().set_size(8);
        let res = next_request.send().promise.await?;
        let rows = res.get()?.get_res()?.get_results()?;
        let mut sum = 0.0;
        let mut total = 0;
        for row in rows.iter() {
            tracing::debug!("{:?}", row);

            let value_at_1 = row.clone()?.get(1); // Get the third element (index 2)
            let value_at_3 = row.clone()?.get(3); // Get the fourth element (index 3)

            if let d_b_any::Which::Text(text) = value_at_1.which()? {
                let text_str = text?.to_str()?; // Extract text as string
                tracing::debug!("Text value: {:?}, user_id:{:?}", &text_str, &user_id);
                if text_str == user_id {
                    tracing::debug!("equal!");
                    if let d_b_any::Which::Real(real) = value_at_3.which()? {
                        tracing::debug!("Real value: {:?}", &real);
                        sum += real;
                    } else {
                        tracing::error!("Expected a real value at index 3");
                    }
                }
                // Get the fourth column (index 3) - known to be a real/float value

            } else {
                tracing::error!("Expected a text value at index 2");
            }


            /*
            for value in row?.iter() {
                match value.which()? {
                    d_b_any::Which::Null(()) => tracing::debug!("None "),
                    d_b_any::Which::Integer(int) => tracing::debug!("{int} "),
                    d_b_any::Which::Real(real) => tracing::debug!("{real} "),
                    d_b_any::Which::Text(text) => tracing::debug!("{} ", text?.to_str()?),
                    d_b_any::Which::Blob(blob) => tracing::debug!("{} ", std::str::from_utf8(blob?)?),
                    d_b_any::Which::Pointer(_) => tracing::debug!("anypointer "),
                }
            }*/
            tracing::debug!("\n");
        }
        tracing::debug!("after res_stream");
        Ok((sum.to_string()))
    }
}
use tokio::sync::mpsc;
use tokio::task;


use keystone::sqlite::Bindings;
use keystone::sqlite::Col;
use keystone::sqlite::DBAnyBindings;


//#[capnproto_rpc(root)]
//impl root::Server for DiligenceImpl {
//    async fn say_hello(self: Rc<Self>, request: Reader) -> capnp::Result<Self> {
/*
#[capnproto_rpc(root)]
impl dil_root::Server for DiligenceImpl {
    //async fn say_hello(self: Rc<Self>, request: Reader) -> capnp::Result<Self> {
    async fn capture_sqlite_requests(self: Rc<Self>, request: Reader) -> capnp::Result<Self> {
    //async fn capture_sqlite_requests(
    //    self: Rc<Self>,
    //    params: dil_root::CaptureSqliteRequestsParams,
    //    mut results: dil_root::CaptureSqliteRequestsResults,
    //) -> Result<(), ::capnp::Error> {
    async fn stop(
        self: Rc<DiligenceImpl>,
        params: dil_root::StopParams,
        mut results: dil_root::StopResults,
    ) -> Result<(), ::capnp::Error> {
        // Your implementation here
        tracing::debug!("stop was called!");
        /*
        while let Some(message) = rx.blocking_recv() {
            println!("Received in non-Send thread: {:?}", message);
        }
        */

        let _ = self.sqlite_insert_task("1".to_string(),"blah1".to_string(),1.2,0).await;
        let _ = self.sqlite_insert_task("2".to_string(),"blah2".to_string(),2.3,1).await;
        let _ = self.sqlite_insert_task("3".to_string(),"blah3".to_string(),3.4,2).await;
        let _ = self.sqlite_select_tasks("2".to_string(),2024,10).await;





        /*
        let next = res
            .get()?
            .get_res()?
            .build_next_request(1)
            .send()
            .promise
            .await?;

        let reader = next.get()?.get_res()?;
        let list = reader.get_results()?;
        tracing::debug!("list contents: {:?}", list);
        let last = if !list.is_empty() {
            match list.get(0)?.get(0).which()? {
                d_b_any::Which::Text(s) => s?.to_str()?,
                _ => "<FAILURE>",
            }
        } else {
            "<No Previous Message>"
        };
        */
        /*

        let second_SelectCore = Some(select_core::SelectCore {
                _from: Some(join_clause::JoinClause {
                    _tableorsubquery: Some(table_or_subquery::TableOrSubquery::_Tableref(
                        self.inner.clone().cast_to::<r_o_table_ref::Client>(),
                    )),
                    _joinoperations: vec![],
                }),
                _results: vec![
                    Expr::_Literal(d_b_any::DBAny::_Text("user_id".to_string())),
                    Expr::_Literal(d_b_any::DBAny::_Text("task_description".to_string())),
                    Expr::_Literal(d_b_any::DBAny::_Text("duration".to_string())),
                    Expr::_Literal(d_b_any::DBAny::_Text("timestamp".to_string()))
                ],
                _sql_where: Some(where_expr::WhereExpr {
                    _cols: vec!["user_id".to_string()],
                    _operator_and_expr: vec![where_expr::op_and_expr::OpAndExpr {
                        _operator: where_expr::Operator::Is,
                        _expr: Some(Expr::_Literal(d_b_any::DBAny::_Integer(2))),
                    }],
                }),
            });


        tracing::debug!("skipping select test since that is on the other branch");

        tracing::debug!("before");
        //let res = self
        let build_select = self
            .sqlite
            .clone()
            .cast_to::<r_o_database::Client>()
            .build_select_request(Some(select::Select {
                _selectcore: Some(Box::new(select_core::SelectCore {
                    _from: Some(join_clause::JoinClause {
                        _tableorsubquery: Some(table_or_subquery::TableOrSubquery::_Tableref(
                            self.inner.clone().cast_to::<r_o_table_ref::Client>(),
                        )),
                        _joinoperations: vec![],
                    }),
                    _results: vec![
                        Expr::_Literal(d_b_any::DBAny::_Text("user_id".to_string())),
                        Expr::_Literal(d_b_any::DBAny::_Text("task_description".to_string())),
                        Expr::_Literal(d_b_any::DBAny::_Text("duration".to_string())),
                        Expr::_Literal(d_b_any::DBAny::_Text("timestamp".to_string()))
                    ],
                    _sql_where: Some(Column {
                        _cols: vec!["user_id".to_string()],
                        _operator_and_expr: vec![where_expr::op_and_expr::OpAndExpr {
                            _operator: where_expr::Operator::Is,
                            _expr: Some(Expr::_Literal(d_b_any::DBAny::_Integer(2))),
                        }],
                    }),
                })),
                _mergeoperations: vec![select::merge_operation::MergeOperation {
                    _operator: select::merge_operation::MergeOperator::Intersect,
                    _selectcore: second_SelectCore
                }],

                _orderby: vec![OrderingTerm {
                    _expr: Some(Expr::_Column(TableColumn {
                        _col_name: "user_id".to_string(),
                        _reference: 0,
                    },)),
                    _direction: select::ordering_term::AscDesc::Asc,
                    },
                    OrderingTerm {
                    _expr: Some(Expr::_Column(TableColumn {
                        _col_name: "task_description".to_string(),
                        _reference: 0,
                    },)),
                    _direction: select::ordering_term::AscDesc::Asc,
                    },
                        OrderingTerm {
                    _expr: Some(Expr::_Column(TableColumn {
                        _col_name: "duration".to_string(),
                        _reference: 0,
                    },)),
                    _direction: select::ordering_term::AscDesc::Asc,
                    },
                    OrderingTerm {
                    _expr: Some(Expr::_Column(TableColumn {
                        _col_name: "timestamp".to_string(),
                        _reference: 0,
                    },)),
                    _direction: select::ordering_term::AscDesc::Asc,
                    },
                ],
                _limit: None,
                _names: vec![],
            }));
            //.send()
            //.promise
            //.await?;

        //tracing::debug!("skipping intersect for now due to crash - waiting on Niv");

        tracing::debug!("middle");


        let res = match build_select.send().promise.await {
            Ok(res) => res,
            Err(e) => {
                tracing::debug!("build select error: {}", e);
                return Err(e);
            },
        };

        let next = res
            .get()?
            .get_res()?
            .build_next_request(1)
            .send()
            .promise
            .await?;
        tracing::debug!("middle2");

        let reader = next.get()?.get_res()?;
        let list = reader.get_results()?;
        tracing::debug!("middle3");
        tracing::debug!("about to print list");
        tracing::debug!("list contents: {:?}", list);
        let last = if !list.is_empty() {
            match list.get(0)?.get(0).which()? {
                d_b_any::Which::Text(s) => s?.to_str()?,
                _ => "<FAILURE>",
            }
        } else {
            "<No Previous Message>"
        };
        tracing::debug!("select result: {:?}", last);
        */
        Ok(())
    }

}
*/
/*

#[cfg(test)]
use tempfile::NamedTempFile;

use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::EnvFilter;

use tracing_subscriber::layer::Context as tracing_Context;
use tracing::Metadata;
use tracing_subscriber::layer::Filter;

use std::fs::OpenOptions;
use tracing::{info, error, debug};
use tracing_subscriber::{
    prelude::*,
    fmt,
    layer::Layer,
    Registry, filter
};

 */
/*
struct TraceOnlyFilter;
impl<S> Filter<S> for TraceOnlyFilter {
    fn enabled(&self, meta: &Metadata<'_>, _: &tracing_Context<'_, S>) -> bool {
        meta.level() == &Level::TRACE
    }
}

 */
/*
// Ensure the subscriber is initialized only once globally.
static INIT: Lazy<()> = Lazy::new(|| {
    let subscriber = Registry::default()
        .with(
            fmt::layer()
                .compact()
                .with_ansi(true)
                .with_filter(TraceOnlyFilter)
        );
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
});

 */
/*
use tokio::time;



#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    keystone::main::<crate::diligence_capnp::config::Owned, DiligenceImpl, dil_root::Owned>(
        async move {
            #[cfg(feature = "tracing")]
            tracing_subscriber::fmt()
                .with_max_level(Level::DEBUG)
                .with_writer(std::io::stderr)
                .with_ansi(true)
                .init();
            diligence_main().await;
        },
    )
    .await
}

 */
/*
#[test]
fn test_diligence_usage() -> eyre::Result<()> {

    // Ensure that the subscriber is initialized before this test
    //Lazy::force(&INIT);

    #[cfg(feature = "tracing")]
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .init();
    tracing::debug!("tracing debug1 trace called!");


    let temp_db = NamedTempFile::new().unwrap().into_temp_path();
    let temp_log = NamedTempFile::new().unwrap().into_temp_path();
    let temp_prefix = NamedTempFile::new().unwrap().into_temp_path();
    let mut source = keystone::build_temp_config(&temp_db, &temp_log, &temp_prefix);

    source.push_str(&keystone::build_module_config(
        "Diligence",
        "diligence-test1-module",
        r#"{ sqlite = [ "@sqlite" ], outer = [ "@keystone", "initCell", {id = "OuterTableRef", default = ["@sqlite", "createTable", { def = [{ name="state", baseType="text", nullable=false }] }, "res"]}, "result" ], inner = [ "@keystone", "initCell", {id = "InnerTableRef"}, "result" ] }"#
    ));

    let (client_writer, server_reader) = async_byte_channel::channel();
    let (server_writer, client_reader) = async_byte_channel::channel();

    let pool = tokio::task::LocalSet::new();
    let a = pool.run_until(pool.spawn_local(keystone::start::<
        crate::diligence_capnp::config::Owned,
        DiligenceImpl,
        crate::diligence_capnp::root::Owned,
        async_byte_channel::Receiver,
        async_byte_channel::Sender,
    >(client_reader, client_writer)));

    let b = pool.run_until(pool.spawn_local(async move {
        let (mut instance, rpc, _disconnect, api) = keystone::Keystone::init_single_module(
            &source,
            "Diligence",
            server_reader,
            server_writer,
        )
        .await
        .unwrap();

        let handle = tokio::task::spawn_local(rpc);
        let usage_client: crate::diligence_capnp::root::Client = api;

        let nonsend_data = Rc::new("world");
        let local = task::LocalSet::new();


        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let (tx2, mut rx2) = tokio::sync::mpsc::channel(32);

        init_global_channels(tx, tx2, rx, rx2).await;
        //let tx = Arc::new(Mutex::new(tx));
        //let rx2 = Arc::new(Mutex::new(rx2));

        let nonsend_data2 = nonsend_data.clone();
        let usage_client2 = usage_client.clone();
        let usage_client3 = usage_client.clone();
        local.spawn_local(async move {
            // ...
            tracing::debug!("hello {}", nonsend_data2);

            let mut echo = usage_client.run_server_request();
            let echo_response = echo.send().promise.await;

            //let msg = echo_response.get()?.get_reply()?.get_message()?;
            tracing::debug!("run response?");
        });

        local.spawn_local(async move {
            time::sleep(time::Duration::from_millis(2000)).await;
            tracing::debug!("goodbye {}", nonsend_data);

            let mut echo = usage_client2.stop_request();
            let echo_response = echo.send().promise.await;
            tracing::debug!("stop echo response acquired");

            match echo_response {
                Ok(x) => tracing::debug!("stop was okay"),
                Err(e) => tracing::debug!("stop err {}", e),
            };

            //let msg = echo_response.get()?.get_reply()?.get_message()?;
            tracing::debug!("stop response?");
        });

        local.spawn_local(async move {

            tracing::debug!("run response?");
            let mut capture = usage_client3.capture_sqlite_requests_request();
            let capture_response = capture.send().promise.await;

            //let msg = echo_response.get()?.get_reply()?.get_message()?;
            tracing::debug!("capture response");
            });

        // ...

        local.await;

        tokio::select! {
            r = handle => r,
            _ = instance.shutdown() => Ok(Ok(())),
        }
        .unwrap()
        .unwrap();
        Ok::<(), capnp::Error>(())
    }));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(async move {
        tokio::select! {
            r = a => r,
            r = b => r,
            r = tokio::signal::ctrl_c() => {
                tracing::debug!("Ctrl-C detected, aborting!");
                Ok(Ok(r.expect("failed to capture ctrl-c")))
            },
        }
    });

    runtime.shutdown_timeout(std::time::Duration::from_millis(1));
    result.unwrap().unwrap();

    Ok(())
}

 */
/*
#[test]
fn test_usage() -> eyre::Result<()> {

    println!("test??");
    // Ensure that the subscriber is initialized before this test
    //Lazy::force(&INIT);

    #[cfg(feature = "tracing")]
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .init();
    tracing::debug!("tracing debug1 trace called!");

    Ok(())
}
*/
use crate::diligence_capnp::root;
use capnp_macros::capnproto_rpc;
use std::rc::Rc;

//pub struct DiligenceImpl {
//    pub greeting: String,
//}
use std::cell::RefCell;
#[capnproto_rpc(root)]
impl root::Server for DiligenceImpl {
    async fn echo_alphabetical(self: Rc<Self>, request: Reader) -> capnp::Result<Self> {
        tracing::debug!("say_hello was called!");
        //let name = request.get_name()?.to_str()?;
        //let greet = "hi".to_string();//self.greeting.as_str();
        //let message = format!("{greet}, {name}!");

        //results.get().init_reply().set_message(message[..].into());
        Ok(())
    }
    async fn run_server(self: Rc<Self>) -> capnp::Result<Self> {
        tracing::debug!("run_server was called!");
        run_discord_server().await;
        tracing::debug!("done");
        //let name = request.get_name()?.to_str()?;
        //let greet = "hi".to_string();//self.greeting.as_str();
        //let message = format!("{greet}, {name}!");

        //results.get().init_reply().set_message(message[..].into());
        Ok(())
    }
    async fn capture_sqlite_requests(self: Rc<Self>) -> capnp::Result<Self> {
        tracing::debug!("capture_sqlite_requests was called!");
        //let name = request.get_name()?.to_str()?;
        //let greet = "hi".to_string();//self.greeting.as_str();
        //let message = format!("{greet}, {name}!");

        //results.get().init_reply().set_message(message[..].into());
        //
        loop {
            tracing::debug!("!send before");
            let mut response_message: SqliteMessage = SqliteMessage::Fail();
            // Receive message using RX2
            let mut rx_option = shared::RX.lock().await;
            if let Some(ref mut rx) = *rx_option { // get the Receiver
                if let Some(message) = rx.recv().await {
                    tracing::debug!("Received in !Send thread: {:?}", message);
                    match message {
                        SqliteMessage::LogTask(id, description, duration, timestamp) => {
                            tracing::debug!("Received LogTask: id={}, msg={}, value={}, num={}", &id, &description, &duration, &timestamp);
                            let insert_result = self.sqlite_insert_task(id.to_string(),description, duration, timestamp.try_into().unwrap()).await; // timestamp is a u64 but sqlite integers are i64. This shouldn't be a problem for a few decades at least.
                            response_message = SqliteMessage::CapnpResultEmpty(insert_result);
                        }
                        SqliteMessage::CountHours(id, year, month) => {
                            tracing::debug!("Received CountHours: id={}, year={}, month={}", &id, &year, &month);
                            let select_result = self.sqlite_select_tasks(id.to_string(),year, month).await;
                            response_message = SqliteMessage::CapnpResultString(select_result);
                        }
                        _ => {
                            tracing::debug!("Received ?: {:?}", message);
                            response_message = SqliteMessage::Fail();
                        }
                    }
                } else {
                    tracing::debug!("Channel closed before receiving a message");
                }
            } else {
                tracing::error!("RX is not initialized");
            }
            tracing::debug!("!send middle");


            // Send message using TX
            let mut tx2_option = shared::TX2.lock().await;
            if let Some(ref tx2) = *tx2_option { // get the Sender
                if let Err(e) = tx2.send(response_message).await {
                    tracing::error!("Failed to send message: {:?}", e);
                } else {
                    tracing::debug!("Message sent successfully");
                }
            } else {
                tracing::error!("TX2 is not initialized");
            }
            tracing::debug!("!send after");
        }
        Ok(())
    }
}

/*
impl keystone::Module<diligence_capnp::config::Owned> for DiligenceImpl {
    async fn new(
        config: <diligence_capnp::config::Owned as capnp::traits::Owned>::Reader<'_>,
        _: keystone::keystone_capnp::host::Client<any_pointer>,
    ) -> capnp::Result<Self> {
        Ok(DiligenceImpl {
            //greeting: config.get_greeting()?.to_string()?,
        })
    }

}*/
use tokio::task::spawn_local;
use tokio::sync::oneshot;

async fn join_spin() {
        sleep(Duration::from_millis(100)).await;
        eprintln!("attempting to join spin handle ");
        // Wait for spin_handle to finish before exiting
        if let Some(handle) = take_spin_handle() {
            sleep(Duration::from_millis(100)).await;
            eprintln!("Waiting for spin handle to exit...");
            sleep(Duration::from_millis(100)).await;
            let _ = handle.await; // Ensures task isn't dropped too early
            sleep(Duration::from_millis(100)).await;
            eprintln!("Spin handle finished execution.");
            sleep(Duration::from_millis(100)).await;
        } else {

            sleep(Duration::from_millis(100)).await;
            eprintln!("No spin handle found!");
            sleep(Duration::from_millis(100)).await;
        }

        sleep(Duration::from_millis(100)).await;
        eprintln!("done with spin handle?");
        sleep(Duration::from_millis(100)).await;
}
async fn kill_spin() {
        sleep(Duration::from_millis(100)).await;
        eprintln!("before attempting to send kill signal");
        sleep(Duration::from_millis(100)).await;
        if let Some(killsender) = take_kill_sender() {
            eprintln!("attempting to send kill signal");
            sleep(Duration::from_millis(100)).await;
            if killsender.send(()).is_err() {
                eprintln!("Failed to send kill signal, receiver might be dropped!");
            }
            eprintln!("finished attempting to send kill signal");
            sleep(Duration::from_millis(100)).await;
        } else {
            sleep(Duration::from_millis(100)).await;
            eprintln!("No killsender found!");
        }
        eprintln!("done with kill signal sending");

        sleep(Duration::from_millis(100)).await;
}

impl keystone::Module<diligence_capnp::config::Owned> for DiligenceImpl {
    async fn stop(&self) -> capnp::Result<()> {
        eprintln!("module stop was called!!");
        // First, send kill signal if we still have a sender
        //
        if let Some(kill_tx) = self.kill_tx.borrow_mut().take() {
            eprintln!("Sending kill signal.");
            let _ = kill_tx.send(()); // ignoring error
            eprintln!("Kill signal sent.");
        }

        // Next, wait for the spin_handle to exit so it doesn't keep the runtime alive
        if let Some(handle) = self.spin_handle.borrow_mut().take() {
            eprintln!("Waiting for background task to finish...");
            let _ = handle.await;
            eprintln!("Background task joined.");
        }

        eprintln!("exiting module stop");
        sleep(Duration::from_millis(1000)).await;
        eprintln!("1000 ms have elapsed after sending kill signal");
        eprintln!("exiting module stop");
        sleep(Duration::from_millis(100)).await;
        Ok(())
    }
    async fn new(
        config: <diligence_capnp::config::Owned as capnp::traits::Owned>::Reader<'_>,
        _: keystone::keystone_capnp::host::Client<any_pointer>,
    ) -> capnp::Result<Self> {


        eprintln!("inside NEW start");
        eprintln!("0 ms have elapsed inside NEW start");

        let (kill_tx, mut kill_rx) = oneshot::channel::<()>();

        sleep(Duration::from_millis(100)).await;

        let spin_handle = spawn_local(async move {
            let mut n = 0;

            loop {
                tokio::select! {
                    Ok(()) = &mut kill_rx => {
                        sleep(Duration::from_millis(100)).await;
                        eprintln!("Received kill signal, exiting loop.");
                        sleep(Duration::from_millis(100)).await;
                        return;
                    }
                    _ = sleep(Duration::from_millis(1000)) => {
                        eprintln!("inside NEW spawn local {}", n);
                        n += 1;
                    }
                }
            }
            eprintln!("Loop unexpectedly exited without receiving the kill signal!");

        });

        //set_spin_handle(spin_handle);
        sleep(Duration::from_millis(100)).await;
        eprintln!("after NEW spawn_local");
        sleep(Duration::from_millis(2000)).await;
        eprintln!("2000 ms have elapsed inside NEW start");
        sleep(Duration::from_millis(10)).await;
        eprintln!("sqlite attempting get");
        let sqlite = config.get_sqlite()?;
        eprintln!("sqlite get");
        let inner = config.get_inner()?;
        eprintln!("inner get");

        /*
        let _ = killsender.send(());
        */

        /*
        set_kill_sender(killsender); // Store the sender
        //kill_spin().await;
        //
        //
        sleep(Duration::from_millis(100)).await;
        eprintln!("before attempting to send kill signal");
        sleep(Duration::from_millis(100)).await;
        if let Some(killsender2) = take_kill_sender() {
            eprintln!("attempting to send kill signal");
            sleep(Duration::from_millis(2000)).await;
            match killsender2.send(()) {
                Ok(_) => eprintln!("Kill signal sent successfully"),
                Err(_) => eprintln!("Failed to send kill signal: receiver may have been dropped"),
            }
            sleep(Duration::from_millis(100)).await;
            eprintln!("finished attempting to send kill signal");
            sleep(Duration::from_millis(100)).await;
        } else {
            sleep(Duration::from_millis(100)).await;
            eprintln!("No killsender found!");
        }
        eprintln!("done with kill signal sending");
        */

        sleep(Duration::from_millis(100)).await;

        let table = inner.get_request().send().promise.await?;
        eprintln!("table get");
        let result = table.get()?;
        eprintln!("result get");

        eprintln!("table fields start");
        let table_fields = vec![table_field::TableField {
                                                _name: "user_id".to_string(),
                                                _base_type: table_field::Type::Text,
                                                _nullable: false,
                                }, table_field::TableField {
                                                _name: "task_description".to_string(),
                                                _base_type: table_field::Type::Text,
                                                _nullable: false,
                                }, table_field::TableField {
                                                _name: "duration".to_string(),
                                                _base_type: table_field::Type::Real,
                                                _nullable: false,
                                },table_field::TableField {
                                                _name: "timestamp".to_string(),
                                                _base_type: table_field::Type::Integer,
                                                _nullable: false,
                                },
        ];

        eprintln!("cap table start");
        let table_cap = if !result.has_data() {
            tracing::debug!("table cap needs to be created");
            let create_table_request = sqlite
                .clone()
                .client
                .cast_to::<add_d_b::Client>()
                .build_create_table_request(table_fields);

            match create_table_request.send().promise.await {
                Ok(response) => {
                    match response.get() {
                        Ok(response_data) => {
                            match response_data.get_res() {
                                Ok(res) => {
                                    // Successfully got the result in `res`
                                    tracing::debug!("a0: ok?");
                                    res
                                }
                                Err(e) => {
                                    // Handle the error from `get_res()`
                                    tracing::debug!("a1: {:?}", e);
                                    return Err(e);
                                }
                            }
                        }
                        Err(e) => {
                            // Handle the error from `get()`
                            tracing::debug!("a2: {:?}", e);
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    // Handle the error from `send().promise.await`
                    tracing::debug!("a3: {:?}", e);
                    return Err(e);
                }
            }

        } else {
            tracing::debug!("table cap already exists");
            result.get_data()?
        };

        eprintln!("cap table end");
        let mut set_request = inner.set_request();

        // Handle the first `?` with a match statement
        match set_request.get().set_data(table_cap.clone()) {
            Ok(x) => {
                // Handle the second `?` inside this block
                match set_request.send().promise.await {
                    Ok(y) => {
                        // Successfully awaited the promise
                        tracing::debug!("b0: ok?");
                    }
                    Err(e) => {
                        // Handle the error from the await
                        tracing::debug!("b1:{:?}", e);
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                // Handle the error from set_data
                tracing::debug!("b2:{:?}", e);
                return Err(e);
            }
        }

        eprintln!("set request end");


        let ins = &sqlite
            .clone()
            .cast_to::<database::Client>()
            .build_insert_request(Some(insert::Insert {
                _fallback: insert::ConflictStrategy::Fail,
                _target: table_cap
                    .clone()
                    .cast_to::<r_a_table_ref::Client>()
                    .clone(),
                _source: source::Source::_Values(vec![
                    vec![
                        d_b_any::DBAny::_Integer(2),
                        d_b_any::DBAny::_Text("test".to_string()),
                        d_b_any::DBAny::_Real(3.14),
                        d_b_any::DBAny::_Integer(3),
                    ],

                ]),
                _cols: vec![
                    "user_id".to_string(),
                    "task_description".to_string(),
                    "duration".to_string(),
                    "timestamp".to_string(),
                ],
                _returning: Vec::new(),
            }))
            .send()
            .promise
            .await?;

        eprintln!("ins mid");
        ins.get()?;

        let ins = &sqlite
            .clone()
            .cast_to::<database::Client>()
            .build_insert_request(Some(insert::Insert {
                _fallback: insert::ConflictStrategy::Fail,
                _target: table_cap
                    .clone()
                    .cast_to::<r_a_table_ref::Client>()
                    .clone(),
                _source: source::Source::_Values(vec![
                    vec![
                        d_b_any::DBAny::_Integer(2),
                        d_b_any::DBAny::_Text("testb".to_string()),
                        d_b_any::DBAny::_Real(2.14),
                        d_b_any::DBAny::_Integer(5),
                    ],

                ]),
                _cols: vec![
                    "user_id".to_string(),
                    "task_description".to_string(),
                    "duration".to_string(),
                    "timestamp".to_string(),
                ],
                _returning: Vec::new(),
            }))
            .send()
            .promise
            .await?;

        eprintln!("ins mid2");
        ins.get()?;

        let ins = sqlite
            .clone()
            .cast_to::<database::Client>()
            .build_insert_request(Some(insert::Insert {
                _fallback: insert::ConflictStrategy::Fail,
                _target: table_cap
                    .clone()
                    .cast_to::<r_a_table_ref::Client>()
                    .clone(),
                _source: source::Source::_Values(vec![
                    vec![
                        d_b_any::DBAny::_Integer(4),
                        d_b_any::DBAny::_Text("test".to_string()),
                        d_b_any::DBAny::_Real(555.14),
                        d_b_any::DBAny::_Integer(2),
                    ],

                ]),
                _cols: vec![
                    "user_id".to_string(),
                    "task_description".to_string(),
                    "duration".to_string(),
                    "timestamp".to_string(),
                ],
                _returning: Vec::new(),
            }))
            .send()
            .promise
            .await?;

        ins.get()?;
        eprintln!("ins end");

        /*
        let insert_request =
            sqlite
            .clone()
            .cast_to::<database::Client>()
            .build_insert_request(Some(insert::Insert {
                _fallback: insert::ConflictStrategy::Fail,
                _target: table_cap
                    .clone()
                    .cast_to::<r_a_table_ref::Client>()
                    .clone(),
                _source: source::Source::_Values(vec![
                    vec![
                        d_b_any::DBAny::_Integer(2),
                        d_b_any::DBAny::_Text("test".to_string()),
                        d_b_any::DBAny::_Real(3.14),
                        d_b_any::DBAny::_Integer(3),
                    ],

                ]),
                _cols: vec![
                    "user_id".to_string(),
                    "task_description".to_string(),
                    "duration".to_string(),
                    "timestamp".to_string(),
                ],
                _returning: Vec::new(),
            }));

        match insert_request.send().promise.await {
            Ok(ins) => {
                match ins.get() {
                    Ok(response) => {
                        // Handle the successful response
                        tracing::debug!("Insert successful: {:?}", response);
                        response
                    }
                    Err(e) => {
                        // Handle the error from `get()`
                        tracing::debug!("Failed to get insert result: {:?}", e);
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                // Handle the error from `send().promise.await`
                tracing::debug!("Insert request failed: {:?}", e);
                return Err(e);
            }
        };
        let insert_request =
            sqlite
            .clone()
            .cast_to::<database::Client>()
            .build_insert_request(Some(insert::Insert {
                _fallback: insert::ConflictStrategy::Fail,
                _target: table_cap
                    .clone()
                    .cast_to::<r_a_table_ref::Client>()
                    .clone(),
                _source: source::Source::_Values(vec![
                    vec![
                        d_b_any::DBAny::_Integer(4),
                        d_b_any::DBAny::_Text("test2".to_string()),
                        d_b_any::DBAny::_Real(3.1422222222),
                        d_b_any::DBAny::_Integer(1),
                    ],

                ]),
                _cols: vec![
                    "user_id".to_string(),
                    "task_description".to_string(),
                    "duration".to_string(),
                    "timestamp".to_string(),
                ],
                _returning: Vec::new(),
            }));

        match insert_request.send().promise.await {
            Ok(ins) => {
                match ins.get() {
                    Ok(response) => {
                        // Handle the successful response
                        tracing::debug!("Insert successful: {:?}", response);
                        response
                    }
                    Err(e) => {
                        // Handle the error from `get()`
                        tracing::debug!("Failed to get insert result: {:?}", e);
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                // Handle the error from `send().promise.await`
                tracing::debug!("Insert request failed: {:?}", e);
                return Err(e);
            }
        };
        */
        /*
        let res = sqlite
            .clone()
            .cast_to::<r_o_database::Client>()
            .build_select_request(Some(select::Select {
                _selectcore: Some(Box::new(select_core::SelectCore {
                    _from: Some(join_clause::JoinClause {
                        _tableorsubquery: Some(table_or_subquery::TableOrSubquery::_Tableref(
                            table_cap.clone().cast_to::<r_o_table_ref::Client>(),
                        )),
                        _joinoperations: vec![],
                    }),
                    _results: vec![
                        Expr::_Literal(d_b_any::DBAny::_Text("user_id".to_string())),
                        Expr::_Literal(d_b_any::DBAny::_Text("task_description".to_string())),
                        Expr::_Literal(d_b_any::DBAny::_Text("duration".to_string())),
                        Expr::_Literal(d_b_any::DBAny::_Text("timestamp".to_string()))
                    ],
                    _sql_where: Some(where_expr::WhereExpr {
                        _cols: vec!["user_id".to_string()],
                        _operator_and_expr: vec![where_expr::op_and_expr::OpAndExpr {
                            _operator: where_expr::Operator::Is,
                            _expr: Some(Expr::_Literal(d_b_any::DBAny::_Integer(2))),
                        }],
                    }),
                })),
                _mergeoperations: vec![],
                _orderby: vec![OrderingTerm {
                    _expr: Some(Expr::_Column(TableColumn {
                        _col_name: "user_id".to_string(),
                        _reference: 0,
                    },)),
                    _direction: select::ordering_term::AscDesc::Asc,
                    },
                    OrderingTerm {
                    _expr: Some(Expr::_Column(TableColumn {
                        _col_name: "task_description".to_string(),
                        _reference: 0,
                    },)),
                    _direction: select::ordering_term::AscDesc::Asc,
                    },
                     OrderingTerm {
                    _expr: Some(Expr::_Column(TableColumn {
                        _col_name: "duration".to_string(),
                        _reference: 0,
                    },)),
                    _direction: select::ordering_term::AscDesc::Asc,
                    },
                    OrderingTerm {
                    _expr: Some(Expr::_Column(TableColumn {
                        _col_name: "timestamp".to_string(),
                        _reference: 0,
                    },)),
                    _direction: select::ordering_term::AscDesc::Asc,
                    },
                ],
                _limit: None,
                _names: vec![],
            }))
            .send()
            .promise
            .await?;

        let next = res
            .get()?
            .get_res()?
            .build_next_request(1)
            .send()
            .promise
            .await?;

        let reader = next.get()?.get_res()?;
        let list = reader.get_results()?;
        tracing::debug!("list contents: {:?}", list);
        let last = if !list.is_empty() {
            match list.get(0)?.get(0).which()? {
                d_b_any::Which::Text(s) => s?.to_str()?,
                _ => "<FAILURE>",
            }
        } else {
            "<No Previous Message>"
        };
        tracing::debug!("select result: {:?}", last);
        */
        /*
        let res_stream = select_request.send().promise.await?.get()?.get_res()?;
        let mut next_request = res_stream.next_request();
        next_request.get().set_size(8);
        let res = next_request.send().promise.await?;
        let rows = res.get()?.get_res()?.get_results()?;
        for row in rows.iter() {
            for value in row?.iter() {
                match value.which()? {
                    d_b_any::Which::Null(()) => tracing::debug!("None "),
                    d_b_any::Which::Integer(int) => tracing::debug!("{int} "),
                    d_b_any::Which::Real(real) => tracing::debug!("{real} "),
                    d_b_any::Which::Text(text) => tracing::debug!("{} ", text?.to_str()?),
                    d_b_any::Which::Blob(blob) => tracing::debug!("{} ", std::str::from_utf8(blob?)?),
                    d_b_any::Which::Pointer(_) => tracing::debug!("anypointer "),
                }
            }
            tracing::debug!();
        }*/



        //let message = format!("usage {last}");
        //results.get().init_reply().set_message(message[..].into());

        sleep(Duration::from_millis(100)).await;
        eprintln!("0 ms have elapsed inside NEW - end");
        sleep(Duration::from_millis(2000)).await;
        eprintln!("2000 ms have elapsed inside NEW - end");
            //sleep(Duration::from_millis(100000)).await;
            //eprintln!("100000 ms have elapsed inside NEW");
        sleep(Duration::from_millis(100)).await;

        Ok(DiligenceImpl {
            inner: table_cap,
            sqlite,
            kill_tx: RefCell::new(Some(kill_tx)),
            spin_handle: RefCell::new(Some(spin_handle)),
        })
    }
}
//#[capnproto_rpc(root)]
//impl root::Server for DiligenceImpl {
    /*
    async fn say_hello(self: Rc<Self>, request: Reader) -> capnp::Result<Self> {
        tracing::debug!("say_hello was called!");
        let name = request.get_name()?.to_str()?;
        let greet = "hi".to_string();//self.greeting.as_str();
        let message = format!("{greet}, {name}!");

        results.get().init_reply().set_message(message[..].into());
        Ok(())
    }
     */
//}
/*
impl keystone::Module<diligence_capnp::config::Owned> for DiligenceImpl {
    async fn new(
        config: <diligence_capnp::config::Owned as capnp::traits::Owned>::Reader<'_>,
        _: keystone::keystone_capnp::host::Client<any_pointer>,
    ) -> capnp::Result<Self> {
        Ok(DiligenceImpl {
            greeting: config.get_greeting()?.to_string()?,
        })
    }

    async fn stop(&self) -> capnp::Result<()> {

        eprintln!("stop was called!!");
        //sleep(Duration::from_millis(10000)).await;
        Ok(())
    }
}
*/
use tokio::time::sleep;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    keystone::main::<crate::diligence_capnp::config::Owned, DiligenceImpl, root::Owned>(
        async move {
            //let _: Vec<String> = ::std::env::args().collect();
            /*
            #[cfg(feature = "tracing")]
            tracing_subscriber::fmt()
                .with_max_level(Level::DEBUG)
                .with_writer(std::io::stderr)
                .with_ansi(true)
                .init();
            tracing::debug!("tracing debug1 trace called!");
            */
            //sleep(Duration::from_millis(100)).await;
            eprintln!("0 ms have elapsed");
            sleep(Duration::from_millis(100)).await;
            eprintln!("100 ms have elapsed");
            sleep(Duration::from_millis(1000)).await;
            eprintln!("1000 ms have elapsed");
            sleep(Duration::from_millis(1000)).await;
            eprintln!("2000 ms have elapsed");
        },
    )
    .await
}
