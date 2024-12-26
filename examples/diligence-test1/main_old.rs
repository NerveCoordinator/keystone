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
//use sqlite_usage_capnp::root;
use diligence_capnp::root as dil_root;
use tracing::Level;


/*
pub struct DiligenceImpl {}

impl dil_root::Server for DiligenceImpl {
    async fn run_server(
        &self,
        params: dil_root::RunServerParams,
        mut results: dil_root::RunServerResults,
    ) -> Result<(), ::capnp::Error> {
        tracing::debug!("run_server was called!");
        Ok(())
    }
}

impl keystone::Module<diligence_capnp::config::Owned> for DiligenceImpl {
    async fn new(
        config: <diligence_capnp::config::Owned as capnp::traits::Owned>::Reader<'_>,
        _: keystone::keystone_capnp::host::Client<any_pointer>,
    ) -> capnp::Result<Self> {
        tracing::trace!("lol");
        println!("aaaaaa");

        Ok(DiligenceImpl {})
    }
}
*/

pub struct DiligenceImpl {
    pub outer: cell::Client<keystone::sqlite_capnp::table_ref::Owned>,
    pub inner: keystone::sqlite_capnp::table::Client,
    pub sqlite: keystone::sqlite_capnp::root::Client,
}
impl dil_root::Server for DiligenceImpl {
    async fn echo_alphabetical(
        &self,
        params: dil_root::EchoAlphabeticalParams,
        mut results: dil_root::EchoAlphabeticalResults,
    ) -> Result<(), ::capnp::Error> {
        tracing::debug!("echo_alphabetical was called!");

        let res = self
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
                    _results: vec![Expr::_Literal(d_b_any::DBAny::_Text("last".to_string()))],
                    _sql_where: None,
                })),
                _mergeoperations: vec![],
                _orderby: vec![OrderingTerm {
                    _expr: Some(Expr::_Column(TableColumn {
                        _col_name: "last".to_string(),
                        _reference: 0,
                    })),
                    _direction: select::ordering_term::AscDesc::Asc,
                }],
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
        let last = if !list.is_empty() {
            match list.get(0)?.get(0).which()? {
                d_b_any::Which::Text(s) => s?.to_str()?,
                _ => "<FAILURE>",
            }
        } else {
            "<No Previous Message>"
        };

        let message = format!("usage {last}");
        results.get().init_reply().set_message(message[..].into());

        let request = params.get()?.get_request()?;
        let current = request.get_name()?.to_str()?;

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
                _source: source::Source::_Values(vec![vec![d_b_any::DBAny::_Text(
                    current.to_string(),
                )]]),
                _cols: vec!["last".to_string()],
                _returning: Vec::new(),
            }))
            .send()
            .promise
            .await?;

        ins.get()?;
        Ok(())
    }
}

impl keystone::Module<diligence_capnp::config::Owned> for DiligenceImpl {
    async fn new(
        config: <diligence_capnp::config::Owned as capnp::traits::Owned>::Reader<'_>,
        _: keystone::keystone_capnp::host::Client<any_pointer>,
    ) -> capnp::Result<Self> {
        let sqlite = config.get_sqlite()?;
        let inner = config.get_inner()?;

        let table = inner.get_request().send().promise.await?;
        let result = table.get()?;
        let table_cap = if !result.has_data() {
            let create_table_request = sqlite
                .clone()
                .client
                .cast_to::<add_d_b::Client>()
                .build_create_table_request(vec![table_field::TableField {
                    _name: "last".to_string(),
                    _base_type: table_field::Type::Text,
                    _nullable: false,
                }]);

            create_table_request
                .send()
                .promise
                .await?
                .get()?
                .get_res()?
        } else {
            result.get_data()?
        };

        let mut set_request = inner.set_request();
        set_request.get().set_data(table_cap.clone())?;
        set_request.send().promise.await?;

        Ok(DiligenceImpl {
            inner: table_cap,
            outer: config.get_outer()?,
            sqlite,
        })
    }
}


use futures::stream::{self, StreamExt}; // Import the necessary traits
use futures::executor::block_on;

async fn async_task(num: u32) {
    println!("Processing {}", num);
    // Simulate some async work
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    println!("Done with {}", num);
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    keystone::main::<crate::diligence_capnp::config::Owned, DiligenceImpl, dil_root::Owned>(
    //keystone::main::<crate::sqlite_usage_capnp::config::Owned, SqliteUsageImpl, root::Owned>(
        async move {
            //let _: Vec<String> = ::std::env::args().collect();

            #[cfg(feature = "tracing")]
            tracing_subscriber::fmt()
                .with_max_level(Level::DEBUG)
                .with_writer(std::io::stderr)
                .with_ansi(true)
                .init();
        },
    )
    .await
}

#[cfg(test)]
use tempfile::NamedTempFile;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::EnvFilter;

use tracing_subscriber::layer::Context;
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

struct TraceOnlyFilter;
impl<S> Filter<S> for TraceOnlyFilter {
    fn enabled(&self, meta: &Metadata<'_>, _: &Context<'_, S>) -> bool {
        meta.level() == &Level::TRACE
    }
}
/*
struct TraceOnlyHigherFilter;
impl<S> Filter<S> for TraceOnlyFilter {
    fn enabled(&self, meta: &Metadata<'_>, _: &Context<'_, S>) -> bool {
        true//meta.level() <= &Level::TRACE
    }
}*/

use once_cell::sync::Lazy;
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
/*
#[test]
fn test_diligence() -> eyre::Result<()> {

    // Ensure that the subscriber is initialized before this test
    Lazy::force(&INIT);
    tracing::trace!("tracing debug1 trace called!");


    let temp_db = NamedTempFile::new().unwrap().into_temp_path();
    let temp_log = NamedTempFile::new().unwrap().into_temp_path();
    let temp_prefix = NamedTempFile::new().unwrap().into_temp_path();
    let mut source = keystone::build_temp_config(&temp_db, &temp_log, &temp_prefix);

    source.push_str(&keystone::build_module_config(
        "Diligence",
        "diligence-module",
        //r#"{}"#
        //r#"{ sqlite = [ "@sqlite" ], outer = [ "@keystone", "initCell", {id = "OuterTableRef", default = ["@sqlite", "createTable", { def = [{ name="state", baseType="text", nullable=false }] }, "res"]}, "result" ], inner = [ "@keystone", "initCell", {id = "InnerTableRef"}, "result" ] }"#
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

        /*
        {
            let mut echo = usage_client.echo_alphabetical_request();
            echo.get().init_request().set_name("3 Keystone".into());
            let echo_response = echo.send().promise.await?;

            let msg = echo_response.get()?.get_reply()?.get_message()?;

            assert_eq!(msg, "usage <No Previous Message>");
        }

        {
            let mut echo = usage_client.echo_alphabetical_request();
            echo.get().init_request().set_name("2 Replace".into());
            let echo_response = echo.send().promise.await?;

            let msg = echo_response.get()?.get_reply()?.get_message()?;

            assert_eq!(msg, "usage 3 Keystone");
        }

        {
            let mut echo = usage_client.echo_alphabetical_request();
            echo.get().init_request().set_name("1 Reload".into());
            let echo_response = echo.send().promise.await?;

            let msg = echo_response.get()?.get_reply()?.get_message()?;

            assert_eq!(msg, "usage 2 Replace");
        }
        */

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
                eprintln!("Ctrl-C detected, aborting!");
                Ok(Ok(r.expect("failed to capture ctrl-c")))
            },
        }
    });

    runtime.shutdown_timeout(std::time::Duration::from_millis(1));
    result.unwrap().unwrap();

    Ok(())
}
*/



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
    tracing::trace!("tracing debug1 trace called!");


    let temp_db = NamedTempFile::new().unwrap().into_temp_path();
    let temp_log = NamedTempFile::new().unwrap().into_temp_path();
    let temp_prefix = NamedTempFile::new().unwrap().into_temp_path();
    let mut source = keystone::build_temp_config(&temp_db, &temp_log, &temp_prefix);

    source.push_str(&keystone::build_module_config(
        "Diligence",
        "diligence-module",
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

        {
            let mut echo = usage_client.echo_alphabetical_request();
            echo.get().init_request().set_name("3 Keystone".into());
            let echo_response = echo.send().promise.await?;

            let msg = echo_response.get()?.get_reply()?.get_message()?;

            assert_eq!(msg, "usage <No Previous Message>");
        }

        {
            let mut echo = usage_client.echo_alphabetical_request();
            echo.get().init_request().set_name("2 Replace".into());
            let echo_response = echo.send().promise.await?;

            let msg = echo_response.get()?.get_reply()?.get_message()?;

            assert_eq!(msg, "usage 3 Keystone");
        }

        {
            let mut echo = usage_client.echo_alphabetical_request();
            echo.get().init_request().set_name("1 Reload".into());
            let echo_response = echo.send().promise.await?;

            let msg = echo_response.get()?.get_reply()?.get_message()?;

            assert_eq!(msg, "usage 2 Replace");
        }
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
                eprintln!("Ctrl-C detected, aborting!");
                Ok(Ok(r.expect("failed to capture ctrl-c")))
            },
        }
    });

    runtime.shutdown_timeout(std::time::Duration::from_millis(1));
    result.unwrap().unwrap();

    Ok(())
}

/*
#[test]
fn test_sqlite_usage() -> eyre::Result<()> {

    // Ensure that the subscriber is initialized before this test
    Lazy::force(&INIT);
    tracing::trace!("tracing debug1 trace called!");


    let temp_db = NamedTempFile::new().unwrap().into_temp_path();
    let temp_log = NamedTempFile::new().unwrap().into_temp_path();
    let temp_prefix = NamedTempFile::new().unwrap().into_temp_path();
    let mut source = keystone::build_temp_config(&temp_db, &temp_log, &temp_prefix);

    source.push_str(&keystone::build_module_config(
        "Sqlite Usage",
        "sqlite-usage-module",
        r#"{ sqlite = [ "@sqlite" ], outer = [ "@keystone", "initCell", {id = "OuterTableRef", default = ["@sqlite", "createTable", { def = [{ name="state", baseType="text", nullable=false }] }, "res"]}, "result" ], inner = [ "@keystone", "initCell", {id = "InnerTableRef"}, "result" ] }"#
    ));

    let (client_writer, server_reader) = async_byte_channel::channel();
    let (server_writer, client_reader) = async_byte_channel::channel();

    let pool = tokio::task::LocalSet::new();
    let a = pool.run_until(pool.spawn_local(keystone::start::<
        crate::sqlite_usage_capnp::config::Owned,
        SqliteUsageImpl,
        crate::sqlite_usage_capnp::root::Owned,
        async_byte_channel::Receiver,
        async_byte_channel::Sender,
    >(client_reader, client_writer)));

    let b = pool.run_until(pool.spawn_local(async move {
        let (mut instance, rpc, _disconnect, api) = keystone::Keystone::init_single_module(
            &source,
            "Sqlite Usage",
            server_reader,
            server_writer,
        )
        .await
        .unwrap();

        let handle = tokio::task::spawn_local(rpc);
        let usage_client: crate::sqlite_usage_capnp::root::Client = api;

        {
            let mut echo = usage_client.echo_alphabetical_request();
            echo.get().init_request().set_name("3 Keystone".into());
            let echo_response = echo.send().promise.await?;

            let msg = echo_response.get()?.get_reply()?.get_message()?;

            assert_eq!(msg, "usage <No Previous Message>");
        }

        {
            let mut echo = usage_client.echo_alphabetical_request();
            echo.get().init_request().set_name("2 Replace".into());
            let echo_response = echo.send().promise.await?;

            let msg = echo_response.get()?.get_reply()?.get_message()?;

            assert_eq!(msg, "usage 3 Keystone");
        }

        {
            let mut echo = usage_client.echo_alphabetical_request();
            echo.get().init_request().set_name("1 Reload".into());
            let echo_response = echo.send().promise.await?;

            let msg = echo_response.get()?.get_reply()?.get_message()?;

            assert_eq!(msg, "usage 2 Replace");
        }

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
                eprintln!("Ctrl-C detected, aborting!");
                Ok(Ok(r.expect("failed to capture ctrl-c")))
            },
        }
    });

    runtime.shutdown_timeout(std::time::Duration::from_millis(1));
    result.unwrap().unwrap();

    Ok(())
}

#[test]
fn test_sqlite_usage2() -> eyre::Result<()> {
    // Ensure that the subscriber is initialized before this test
    Lazy::force(&INIT);

    tracing::trace!("inside test_sqlite_usage2!");

    Ok(())
}
*/
