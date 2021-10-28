use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use async_channel::{bounded, Receiver, RecvError, Sender, TryRecvError, TrySendError};
use chrono::Utc;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri};
use ipfs_api_backend_hyper::response::IdResponse;
use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::db::schema::{Node, NodeAddr, NodeObjectPin, NodeUpdate, Object, Peer};

mod db;

static MATCH_IP: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^/ip(\d)/(.*)/(?:tcp|udp)/.*$"#).unwrap());

#[derive(Clone)]
struct Data {
    db: Arc<PgPool>,
    seen: Arc<Mutex<HashSet<String>>>,
    to_scan_tx: Arc<Sender<NodeData>>,
    to_scan_rx: Arc<Receiver<NodeData>>,
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

// #[async_recursion(? Send)]
// async fn scan_node(data: &Data, node: &NodeData) {
//     // let prefix = " ".repeat((depth * 4) as usize);
//
//     match node.client.pin_ls(None, None).await {
//         Ok(v) => {
//             for (id, _) in v.keys {
//                 match node.client.object_stat(&format!("/ipfs/{}", id)).await {
//                     Ok(v) => {
//                         // println!("{} --> {}: {}", id, v.data_size);
//
//                         db::model::add_object(&data.db, &Object {
//                             id: id.clone(),
//                             size: v.data_size as i64,
//                         }).await.unwrap();
//
//                         db::model::add_node_object_pin(&data.db, &NodeObjectPin {
//                             id_node: node.info.id.clone(),
//                             id_object: id.clone(),
//                         }).await.unwrap();
//                     }
//                     Err(e) => {
//                         println!("x-> {}: {:?}", id, e);
//                     }
//                 };
//             }
//         }
//         Err(e) => {
//             println!("x-> {:?}", e);
//         }
//     };
//
//     let peers = match node.client.swarm_peers().await {
//         Ok(v) => v,
//         Err(_) => {
//             println!("- {}", node.addr);
//             return;
//         }
//     };
//
//     db::model::deactivate_node_peers(&data.db, &node.info.id).await.unwrap();
//     let mut peers_shuf = peers.peers;
//     peers_shuf.shuffle(&mut thread_rng());
//
//     for peer in peers_shuf {
//         let (p, a) = match MATCH_IP.captures(&peer.addr) {
//             None => continue,
//             Some(v) => {
//                 if v.len() != 3 {
//                     println!("? {}", node.addr);
//                     continue;
//                 }
//
//                 let p = v.get(1).unwrap().as_str().parse::<u8>().unwrap();
//                 let a = v.get(2).unwrap().as_str().to_owned();
//
//                 (p, a)
//             }
//         };
//
//         let addr = match p {
//             4 => format!("/ip4/{}/tcp/5001/http", a),
//             6 => format!("/ip6/{}/tcp/5001/http", a),
//             _ => {
//                 println!("* {}", node.addr);
//                 continue;
//             }
//         };
//
//         db::model::add_node_addr(&data.db, &NodeAddr {
//             id_node: node.info.id.clone(),
//             addr: addr.clone(),
//         }).await.unwrap();
//
//         {
//             let mut seen = SEEN.lock().await;
//             if seen.contains(&addr) {
//                 continue;
//             }
//             seen.insert(addr.clone());
//         }
//
//         match get_node(&addr).await {
//             None => {
//                 println!("  {}", addr);
//
//                 save_closed_node(&data, &peer.peer).await;
//                 save_peer(&data, &node.info.id, &peer.peer).await;
//             }
//             Some(v) => {
//                 println!("+ {}", addr);
//
//                 // Use resolved node ID here instead of the ID from `peer.peer`
//                 save_public_node(&data, &v.info.id, &v.addr).await;
//                 save_peer(&data, &node.info.id, &v.info.id).await;
//
//                 data.to_scan.push(node.addr.clone());
//
//                 // if depth < 32 {
//                 //     scan_node(data, &v, depth + 1).await;
//                 // }
//             }
//         }
//     }
// }

async fn scan_node_2(data: Arc<Mutex<Data>>, id: &str, addr: &str) -> anyhow::Result<()> {
    let (p, a) = match MATCH_IP.captures(addr) {
        None => return Ok(()),
        Some(v) => {
            if v.len() != 3 {
                println!("? {}", addr);
                return Ok(());
            }

            let p = v.get(1).unwrap().as_str().parse::<u8>()?;
            let a = v.get(2).unwrap().as_str().to_owned();

            (p, a)
        }
    };

    let public_addr = match p {
        4 => format!("/ip4/{}/tcp/5001/http", a),
        6 => format!("/ip6/{}/tcp/5001/http", a),
        _ => {
            println!("* {}", addr);
            return Ok(());
        }
    };

    let existing = {
        let data_l = data.lock().await;

        db::model::get_node(&data_l.db, id).await?
    };

    // {
    //     let data_l = data.lock().await;
    //
    //     if data_l.seen.contains(&public_addr) {
    //         return;
    //     }
    //     data_l.seen.insert(public_addr.clone());
    //
    //     // db::model::add_node_addr(&data_l.db, &NodeAddr {
    //     //     id_node: node_ref.id.clone(),
    //     //     addr: node_ref.addr.clone(),
    //     // }).await.unwrap();
    // }

    match get_node(&public_addr).await {
        None => {
            println!("  {}", public_addr);

            let data_l = data.lock().await;

            match existing {
                None => {
                    // This node has never been seen before. Add it.
                    db::model::add_node(&data_l.db, &Node {
                        id: id.to_owned(),
                        seen_first: Utc::now(),
                        seen_last: Utc::now(),
                        scan_last: None,
                        public_addr: None,
                    }).await?
                }
                Some(v) => match v.public_addr {
                    None => {
                        // This node has been seen before, but it didn't have a public address. Only update seen_last.
                        db::model::update_node(&data_l.db, &NodeUpdate {
                            id: id.to_owned(),
                            seen_last: v.seen_last,
                            scan_last: v.scan_last,
                            public_addr: None,
                        }).await?;
                    }
                    Some(_) => {
                        // This node has been seen before with a public address, but it is not inaccessible. Delete the public address.
                        db::model::update_node(&data_l.db, &NodeUpdate {
                            id: id.to_owned(),
                            seen_last: v.seen_last,
                            scan_last: v.scan_last,
                            public_addr: None,
                        }).await?;
                    }
                }
            }
        }
        Some(v) => {
            println!("+ {}", public_addr);

            let data_l = data.lock().await;

            match existing {
                None => {
                    // This node has never been seen before. Add it.
                    db::model::add_node(&data_l.db, &Node {
                        id: "".to_string(),
                        seen_first: Utc::now(),
                        seen_last: Utc::now(),
                        scan_last: None,
                        public_addr: Some(public_addr.clone()),
                    }).await?;
                }
                Some(v) => {
                    // This node had been seen before. Update seen_last.
                    db::model::update_node(&data_l.db, &NodeUpdate {
                        id: id.to_owned(),
                        seen_last: v.seen_last,
                        scan_last: v.scan_last,
                        public_addr: Some(public_addr.clone()),
                    }).await?;
                }
            }

            data_l.to_scan_tx.send(v).await?;

            drop(data_l);
        }
    }

    Ok(())
}

async fn read_node_objects(data: Arc<Mutex<Data>>, node: &NodeData) {
    match node.client.pin_ls(None, None).await {
        Ok(v) => {
            for (id, _) in v.keys {
                match node.client.object_stat(&format!("/ipfs/{}", id)).await {
                    Ok(v) => {
                        // println!("{} --> {}: {}", id, v.data_size);
                        let data_l = data.lock().await;

                        db::model::add_object(&data_l.db, &Object {
                            id: id.clone(),
                            size: v.data_size as i64,
                        }).await.unwrap();
                        db::model::add_node_object_pin(&data_l.db, &NodeObjectPin {
                            id_node: node.info.id.clone(),
                            id_object: id.clone(),
                        }).await.unwrap();
                    }
                    Err(e) => {
                        println!("x-> {}: {:?}", id, e);
                    }
                };
            }
        }
        Err(e) => {
            println!("x-> {:?}", e);
        }
    };
}

async fn read_node_peers(data: Arc<Mutex<Data>>, node: &NodeData) {
    let peers = match node.client.swarm_peers().await {
        Ok(v) => v,
        Err(_) => {
            println!("- {}", node.addr);
            return;
        }
    };

    let mut peers_new = Vec::new();
    {
        let mut data_l = data.lock().await;

        db::model::deactivate_node_peers(&data_l.db, &node.info.id).await.unwrap();

        for peer in &peers.peers {
            db::model::add_peer(&data_l.db, &Peer {
                id_left: node.info.id.clone(),
                id_right: peer.peer.clone(),
            }).await.unwrap();

            db::model::add_node_addr(&data_l.db, &NodeAddr {
                id_node: peer.peer.clone(),
                addr: peer.addr.clone(),
            }).await.unwrap();

            let mut seen_l = data_l.seen.lock().await;

            if !seen_l.contains(&peer.addr) {
                seen_l.insert(peer.addr.clone());
                peers_new.push(peer.clone())
            }
        }
    }

    for peer in &peers_new {
        match scan_node_2(data.clone(), &peer.peer, &peer.addr).await {
            Ok(()) => {}
            Err(e) => {
                println!("! {}", e);
            }
        }
    }
}

async fn node_scan_worker(data: Arc<Mutex<Data>>) {
    let rx = data.lock().await.to_scan_rx.clone();

    loop {
        match rx.try_recv() {
            Ok(v) => {
                read_node_objects(data.clone(), &v).await;
                read_node_peers(data.clone(), &v).await;

                //
            }
            Err(e) => match e {
                TryRecvError::Empty => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
                TryRecvError::Closed => {
                    return;
                }
            }
        }
    }
}

// async fn save_public_node(data: &Data, id: &str, public_multiaddr: &str) {
//     db::model::add_node(&data.db, &Node {
//         id: id.to_owned(),
//         seen_first: Utc::now(),
//         seen_last: Utc::now(),
//         scan_last: None,
//         public_addr: Some(public_multiaddr.to_owned()),
//     }).await.unwrap();
// }
//
// async fn save_closed_node(data: &Data, id: &str) {
//     db::model::add_node(&data.db, &Node {
//         id: id.to_owned(),
//         seen_first: Utc::now(),
//         seen_last: Utc::now(),
//         scan_last: None,
//         public_addr: None,
//     }).await.unwrap();
// }
//
// async fn save_peer(data: &Data, id: &str, peer: &str) {
//     db::model::add_peer(&data.db, &Peer {
//         id_left: id.to_owned(),
//         id_right: peer.to_owned(),
//     }).await.unwrap();
// }

#[tokio::main]
async fn main() {
    let node = get_node("/ip4/127.0.0.1/tcp/5001/http").await.unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://ipfsi_usr:34b04a8a3e834b10@10.2.0.16/db0")
        .await.unwrap();

    db::model::add_node(&pool, &Node {
        id: node.info.id.clone(),
        seen_first: Utc::now(),
        seen_last: Utc::now(),
        scan_last: None,
        public_addr: None,
    }).await.unwrap();

    let (to_scan_tx, to_scan_rx) = async_channel::bounded(128);

    let to_scan_tx = Arc::new(to_scan_tx);
    to_scan_tx.send(node).await.unwrap();

    let to_scan_rx = Arc::new(to_scan_rx);

    let data = Data {
        db: Arc::new(pool),
        seen: Arc::new(Mutex::new(HashSet::new())),
        to_scan_tx,
        to_scan_rx,
    };
    let data = Arc::new(Mutex::new(data));

    let mut handles = Vec::new();

    for _ in 0..64 {
        let d = data.clone();
        let handle = tokio::spawn(async move {
            node_scan_worker(d).await;
        });
        handles.push(handle);
    }

    futures::future::join_all(handles).await;

    // save_closed_node(&data, &node.info.id).await;
    // scan_node_2(data, &node).await;

    //

    // let (tx, rx) = tokio::sync::oneshot::channel::<u8>();
    // rx.await;
}
