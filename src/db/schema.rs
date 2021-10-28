use chrono::{DateTime, Utc};

pub struct Node {
    pub id: String,
    pub seen_first: DateTime<Utc>,
    pub seen_last: DateTime<Utc>,
    pub scan_last: Option<DateTime<Utc>>,
    pub public_addr: Option<String>,
}

pub struct NodeUpdate {
    pub id: String,
    pub seen_last: DateTime<Utc>,
    pub scan_last: Option<DateTime<Utc>>,
    pub public_addr: Option<String>,
}

pub struct NodeAddr {
    pub id_node: String,
    pub addr: String,
}

pub struct Peer {
    pub id_left: String,
    pub id_right: String,
}

pub struct Object {
    pub id: String,
    pub size: i64,
}

pub struct NodeObjectPin {
    pub id_node: String,
    pub id_object: String,
}
