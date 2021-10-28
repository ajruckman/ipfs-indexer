use std::convert::TryFrom;
use std::time::Duration;

use futures::TryStreamExt;
use sqlx::{Pool, Postgres, query};
use sqlx::postgres::types::PgInterval;

use crate::db::schema::{Node, NodeAddr, NodeObjectPin, NodeUpdate, Object, Peer};

pub async fn get_node(
    conn: &Pool<Postgres>,
    id: &str,
) -> anyhow::Result<Option<Node>> {
    let r = query!("SELECT id, seen_first, seen_last, scan_last, public_addr FROM node WHERE id=$1",
        id)
        .fetch_optional(conn)
        .await?;

    Ok(match r {
        Some(r) => Some(Node {
            id: r.id,
            seen_first: r.seen_first,
            seen_last: r.seen_last,
            scan_last: r.scan_last,
            public_addr: r.public_addr,
        }),
        _ => None,
    })
}

pub async fn add_node(
    conn: &Pool<Postgres>,
    node: &Node,
) -> anyhow::Result<()> {
    query!("INSERT INTO node (id, seen_first, seen_last, public_addr)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT ON CONSTRAINT node_pk DO UPDATE SET seen_last=$3, public_addr=$4",
        node.id, node.seen_first, node.seen_last, node.public_addr)
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn update_node(
    conn: &Pool<Postgres>,
    update: &NodeUpdate,
) -> anyhow::Result<()> {
    query!("UPDATE node
            SET seen_last=$1, scan_last=$2, public_addr=$3
            WHERE id=$4",
        update.seen_last, update.scan_last, update.public_addr, update.id)
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn deactivate_node_addrs(
    conn: &Pool<Postgres>,
    id_node: &str,
) -> anyhow::Result<()> {
    query!("UPDATE node_addr SET active=FALSE WHERE id_node=$1",
        id_node)
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn add_node_addr(
    conn: &Pool<Postgres>,
    node_addr: &NodeAddr,
) -> anyhow::Result<()> {
    query!("INSERT INTO node_addr (id_node, addr, active)
            VALUES ($1, $2, TRUE)
            ON CONFLICT ON CONSTRAINT node_addr_pk DO UPDATE SET active=TRUE",
        node_addr.id_node, node_addr.addr)
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn deactivate_node_peers(
    conn: &Pool<Postgres>,
    id_node: &str,
) -> anyhow::Result<()> {
    query!("UPDATE peer SET active=FALSE WHERE id_left=$1",
        id_node)
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn add_peer(
    conn: &Pool<Postgres>,
    peer: &Peer,
) -> anyhow::Result<()> {
    query!("INSERT INTO peer (id_left, id_right, active)
            VALUES ($1, $2, TRUE)
            ON CONFLICT ON CONSTRAINT peer_pk DO UPDATE SET active=TRUE",
        peer.id_left, peer.id_right)
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn add_object(
    conn: &Pool<Postgres>,
    object: &Object,
) -> anyhow::Result<()> {
    query!("INSERT INTO object (id, size)
            VALUES ($1, $2)
            ON CONFLICT ON CONSTRAINT object_pk DO NOTHING",
        object.id, object.size)
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn add_node_object_pin(
    conn: &Pool<Postgres>,
    node_object_pin: &NodeObjectPin,
) -> anyhow::Result<()> {
    query!("INSERT INTO node_object_pin (id_node, id_object)
            VALUES ($1, $2)
            ON CONFLICT ON CONSTRAINT node_object_pin_pk DO NOTHING",
        node_object_pin.id_node, node_object_pin.id_object)
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn get_unscanned_nodes(
    conn: &Pool<Postgres>,
    min_time_since_last: Duration,
) -> anyhow::Result<Vec<Node>> {
    let interval = PgInterval::try_from(min_time_since_last).unwrap();

    let mut stream = query!("SELECT * FROM node
            WHERE NOW() - scan_last > $1",
        interval)
        .map(|row| {
            Node {
                id: row.id,
                seen_first: row.seen_first,
                seen_last: row.seen_last,
                scan_last: row.scan_last,
                public_addr: row.public_addr,
            }
        })
        .fetch(conn);

    let mut result = Vec::new();
    while let Some(row) = stream.try_next().await? {
        result.push(row);
    }

    Ok(result)
}
