use sqlx::{Pool, Postgres, query};

use crate::db::schema::{Node, NodeAddr, Peer};

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
