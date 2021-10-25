-- Create users for new schema
CREATE USER ipfsi_mgr WITH ENCRYPTED PASSWORD 'a3ac050850d81064' INHERIT;
CREATE USER ipfsi_usr WITH ENCRYPTED PASSWORD '34b04a8a3e834b10' INHERIT;
CREATE USER ipfsi_ro  WITH ENCRYPTED PASSWORD '50cb0d68d38b677f' INHERIT;

GRANT ipfsi_usr TO ipfsi_mgr;
GRANT ipfsi_ro  TO ipfsi_usr;

GRANT CONNECT ON DATABASE db0 TO ipfsi_ro; -- others inherit

-- Create new schema
\connect db0

CREATE SCHEMA ipfsi AUTHORIZATION ipfsi_mgr;

SET search_path = ipfsi;

-- These are not inheritable
ALTER ROLE ipfsi_mgr IN DATABASE db0 SET search_path = ipfsi;
ALTER ROLE ipfsi_usr IN DATABASE db0 SET search_path = ipfsi;
ALTER ROLE ipfsi_ro  IN DATABASE db0 SET search_path = ipfsi;

GRANT CREATE ON SCHEMA ipfsi TO ipfsi_mgr;
GRANT USAGE  ON SCHEMA ipfsi TO ipfsi_ro ; -- ipfsi_usr inherits

-- Set default privileges
-- -> Read only
ALTER DEFAULT PRIVILEGES FOR ROLE ipfsi_mgr GRANT SELECT ON TABLES TO ipfsi_ro;

-- -> Read/write
ALTER DEFAULT PRIVILEGES FOR ROLE ipfsi_mgr GRANT INSERT, UPDATE, DELETE, TRUNCATE ON TABLES TO ipfsi_usr;

-- -> Read/write for sequences
ALTER DEFAULT PRIVILEGES FOR ROLE ipfsi_mgr GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO ipfsi_usr;
