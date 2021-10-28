SET SEARCH_PATH = ipfsi;

DROP TABLE IF EXISTS node CASCADE;
DROP TABLE IF EXISTS node_addr CASCADE;
DROP TABLE IF EXISTS peer CASCADE;
DROP TABLE IF EXISTS object CASCADE;
DROP TABLE IF EXISTS node_object_pin CASCADE;

CREATE TABLE node
(
    id          VARCHAR(64) NOT NULL,
    seen_first  timestamptz NOT NULL,
    seen_last   timestamptz NOT NULL,
    scan_last   timestamptz,
    public_addr VARCHAR(128),

    CONSTRAINT node_pk PRIMARY KEY (id)
);

CREATE TABLE node_addr
(
    id_node VARCHAR(64)  NOT NULL,
    addr    VARCHAR(128) NOT NULL,
    active  BOOLEAN      NOT NULL,

    CONSTRAINT node_addr_pk PRIMARY KEY (id_node, addr),
    CONSTRAINT node_addr_id_node_fk FOREIGN KEY (id_node) REFERENCES node (id)
);

CREATE TABLE peer
(
    id_left  VARCHAR(64) NOT NULL,
    id_right VARCHAR(64) NOT NULL,
    active   BOOLEAN     NOT NULL,

    CONSTRAINT peer_pk PRIMARY KEY (id_left, id_right),
    CONSTRAINT peer_id_left_fk FOREIGN KEY (id_left) REFERENCES node (id),
    CONSTRAINT peer_id_right_fk FOREIGN KEY (id_right) REFERENCES node (id)
);

CREATE TABLE object
(
    id   VARCHAR(64) NOT NULL,
    size BIGINT      NOT NULL,

    CONSTRAINT object_pk PRIMARY KEY (id)
);

CREATE TABLE node_object_pin
(
    id_node   VARCHAR(64) NOT NULL,
    id_object VARCHAR(64) NOT NULL,

    CONSTRAINT node_object_pin_pk PRIMARY KEY (id_node, id_object),
    CONSTRAINT node_object_pin_node_id_fk FOREIGN KEY (id_node) REFERENCES node (id),
    CONSTRAINT node_object_pin_object_id_fk FOREIGN KEY (id_object) REFERENCES object (id)
);
