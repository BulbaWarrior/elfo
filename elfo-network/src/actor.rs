use std::{
    fmt::{self, Display},
    hash::Hash,
    sync::Arc,
};

use elfo_core::{
    node::NodeNo,
    routers::{MapRouter, Outcome},
    ActorGroup, Context, GroupNo, Schema, Topology,
};

use crate::{config::Config, discovery::Discovery, listener::Listener, node_map::NodeMap};

#[derive(PartialEq, Eq, Hash, Clone)]
pub(crate) enum Key {
    Listener,
    Discovery,
    Tx(NodeNo, GroupNo),
    Rx(NodeNo, GroupNo),
}

impl Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: resolve `group_no` to name.
        match self {
            Key::Listener => f.write_str("listener"),
            Key::Discovery => f.write_str("discovery"),
            Key::Tx(node_no, group_no) => write!(f, "tx:{}:{}", node_no, group_no),
            Key::Rx(node_no, group_no) => write!(f, "rx:{}:{}", node_no, group_no),
        }
    }
}

pub fn new(topology: &Topology) -> Schema {
    let node_map = Arc::new(NodeMap::new(topology));

    ActorGroup::new()
        .config::<Config>()
        .router(MapRouter::new(|_| {
            Outcome::Multicast(vec![Key::Listener, Key::Discovery])
        }))
        .exec(move |ctx: Context<Config, Key>| {
            let node_map = node_map.clone();

            async move {
                match ctx.key() {
                    Key::Listener => Listener::new(node_map).main(ctx).await,
                    Key::Discovery => Discovery::new(node_map).main(ctx).await,
                    _ => todo!(),
                }
            }
        })
}
