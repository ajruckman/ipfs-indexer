use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use async_recursion::async_recursion;
use chrono::Utc;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri};
use ipfs_api_backend_hyper::response::IdResponse;
use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::db::schema::{Node, NodeAddr, Peer};

mod db;

static MATCH_IP: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^/ip(\d)/(.*)/(?:tcp|udp)/.*$"#).unwrap());
static SEEN: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

#[derive(Clone)]
struct Data {
    db: Arc<PgPool>,
}

struct NodeData {
    client: IpfsClient,
    addr: String,
    info: IdResponse,
}

async fn get_node(addr: &str) -> Option<NodeData> {
    let client = match IpfsClient::from_multiaddr_str(addr) {
        Ok(v) => v,
        Err(_) => {
            return None;
        }
    };

    match timeout(Duration::from_secs(3), client.id(None)).await {
        Ok(v) => match v {
            Ok(v) => Some(NodeData {
                client,
                addr: addr.to_owned(),
                info: v,
            }),
            Err(_) => None,
        }
        Err(_) => None,
    }
}

#[async_recursion(? Send)]
async fn scan_node(data: &Data, node: &NodeData, depth: u8) {
    let prefix = " ".repeat((depth * 4) as usize);

    // match client.pin_ls(None, None).await {
    //     Ok(v) => {
    //         for (id, t) in v.keys {
    //             match client.object_stat(&format!("/ipfs/{}", id)).await {
    //                 Ok(v) => {
    //                     println!("{} --> {}: {}", prefix, id, v.data_size);
    //                 }
    //                 Err(e) => {
    //                     println!("{} x-> {}: {}", prefix, id, e);
    //                 }
    //             };
    //         }
    //     }
    //     Err(e) => {}
    // };

    let peers = match node.client.swarm_peers().await {
        Ok(v) => v,
        Err(_) => {
            println!("{} - {}", prefix, node.addr);
            return;
        }
    };

    db::model::deactivate_node_peers(&data.db, &node.info.id).await.unwrap();

    for peer in peers.peers {
        let (p, a) = match MATCH_IP.captures(&peer.addr) {
            None => continue,
            Some(v) => {
                if v.len() != 3 {
                    println!("{} ? {}", prefix, node.addr);
                    continue;
                }

                let p = v.get(1).unwrap().as_str().parse::<u8>().unwrap();
                let a = v.get(2).unwrap().as_str().to_owned();

                (p, a)
            }
        };

        let addr = match p {
            4 => format!("/ip4/{}/tcp/5001/http", a),
            6 => format!("/ip6/{}/tcp/5001/http", a),
            _ => {
                println!("{} * {}", prefix, node.addr);
                continue;
            }
        };

        db::model::add_node_addr(&data.db, &NodeAddr {
            id_node: node.info.id.clone(),
            addr: addr.clone(),
        }).await.unwrap();

        {
            let mut seen = SEEN.lock().await;
            if seen.contains(&addr) {
                continue;
            }
            seen.insert(addr.clone());
        }

        match get_node(&addr).await {
            None => {
                println!("{}   {}", prefix, addr);

                save_closed_node(&data, &peer.peer).await;
                save_peer(&data, &node.info.id, &peer.peer).await;
            }
            Some(v) => {
                println!("{} + {}", prefix, addr);

                // Use resolved node ID here instead of the ID from `peer.peer`
                save_public_node(&data, &v.info.id, &v.addr).await;
                save_peer(&data, &node.info.id, &v.info.id).await;
                scan_node(data, &v, depth + 1).await;
            }
        }
    }
}

async fn save_public_node(data: &Data, id: &str, public_multiaddr: &str) {
    db::model::add_node(&data.db, &Node {
        id: id.to_owned(),
        seen_first: Utc::now(),
        seen_last: Utc::now(),
        public_addr: Some(public_multiaddr.to_owned()),
    }).await.unwrap();
}

async fn save_closed_node(data: &Data, id: &str) {
    db::model::add_node(&data.db, &Node {
        id: id.to_owned(),
        seen_first: Utc::now(),
        seen_last: Utc::now(),
        public_addr: None,
    }).await.unwrap();
}

async fn save_peer(data: &Data, id: &str, peer: &str) {
    db::model::add_peer(&data.db, &Peer {
        id_left: id.to_owned(),
        id_right: peer.to_owned(),
    }).await.unwrap();
}

#[tokio::main]
async fn main() {
    let node = get_node("/ip4/127.0.0.1/tcp/5001/http").await.unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://ipfsi_usr:34b04a8a3e834b10@10.2.0.16/db0")
        .await.unwrap();

    let data = Data {
        db: Arc::new(pool),
    };

    save_closed_node(&data, &node.info.id).await;
    scan_node(&data, &node, 0).await;
}
