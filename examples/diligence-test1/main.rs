
include!(concat!(env!("OUT_DIR"), "/capnproto.rs"));

use crate::diligence_capnp::root;
use capnp::any_pointer::Owned as any_pointer;
use capnp_macros::capnproto_rpc;
use std::rc::Rc;

pub struct DiligenceImpl {
    pub greeting: String,
}

#[capnproto_rpc(root)]
impl root::Server for DiligenceImpl {
    async fn say_hello(self: Rc<Self>, request: Reader) -> capnp::Result<Self> {
        tracing::debug!("say_hello was called!");
        let name = request.get_name()?.to_str()?;
        let greet = self.greeting.as_str();
        let message = format!("{greet}, {name}!");

        results.get().init_reply().set_message(message[..].into());
        Ok(())
    }
}

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
use tokio::time::{sleep, Duration};
use tracing::Level;

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
        },
    )
    .await
}
