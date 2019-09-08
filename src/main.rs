mod rpc;
mod error;
mod types;

use futures::{
    compat::{Future01CompatExt, Stream01CompatExt},
    future::{FutureExt, self, TryFutureExt, lazy},
    stream::{TryStreamExt, StreamExt, Stream},
    channel::mpsc
};


fn main() {
    let  (mut rt, client) = rpc::client();
    let (sender, receiver) = mpsc::unbounded();
    rt.spawn(rpc::sub_blocks(client.clone(), sender.clone()).unit_error().boxed().compat());
    rt.spawn(rpc::sub_finalized(client.clone(), sender.clone()).unit_error().boxed().compat());
    let data = receiver.for_each(|x| {
        println!("got a message");
        println!("{:?}", x);
        future::ready(())
    });
    tokio::run(data.unit_error().boxed().compat());
}
