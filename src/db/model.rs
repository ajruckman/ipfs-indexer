use sqlx::{Pool, Postgres, query};

use crate::db::schema::{Node, NodeAddr, NodeObjectPin, Object, Peer};

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
