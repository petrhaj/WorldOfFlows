import json
import os
import sqlite3
import struct
import sys

src, dst = sys.argv[1], sys.argv[2]

if os.path.exists(dst):
    con = sqlite3.connect(dst)
    columns = {row[1] for row in con.execute("PRAGMA table_info(links)")}
    if "cost_eur_per_tonne" not in columns:
        if "cost_usd_per_tonne" not in columns:
            con.close()
            raise RuntimeError("Existing GeoPackage is missing links.cost_usd_per_tonne")
        con.execute(
            "ALTER TABLE links RENAME COLUMN cost_usd_per_tonne TO cost_eur_per_tonne"
        )
        columns.remove("cost_usd_per_tonne")
        columns.add("cost_eur_per_tonne")
    if "cost_eur_per_passenger" not in columns:
        if "cost_usd_per_passenger" in columns:
            con.execute(
                "ALTER TABLE links RENAME COLUMN cost_usd_per_passenger TO cost_eur_per_passenger"
            )
        else:
            con.execute(
                "ALTER TABLE links ADD COLUMN cost_eur_per_passenger DOUBLE NOT NULL DEFAULT 0"
            )
            con.execute(
                "UPDATE links SET cost_eur_per_passenger=cost_eur_per_tonne"
            )
    con.execute(
        "INSERT OR REPLACE INTO network_metadata(key,value) VALUES "
        "('cost_unit','Estimated EUR per metric tonne')"
    )
    con.execute(
        "INSERT OR REPLACE INTO network_metadata(key,value) VALUES "
        "('passenger_cost_unit','Estimated EUR per passenger per link')"
    )
    con.commit()
    con.close()
    sys.exit(0)

with open(src, "r", encoding="utf-8") as f:
    data = json.load(f)

con = sqlite3.connect(dst)
con.execute("PRAGMA application_id=1196437808")
con.execute("PRAGMA user_version=1")
con.executescript("""
CREATE TABLE gpkg_contents (
 table_name TEXT NOT NULL PRIMARY KEY, data_type TEXT NOT NULL,
 identifier TEXT UNIQUE, description TEXT DEFAULT '',
 last_change DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 min_x DOUBLE, min_y DOUBLE, max_x DOUBLE, max_y DOUBLE, srs_id INTEGER
);
CREATE TABLE gpkg_geometry_columns (
 table_name TEXT NOT NULL, column_name TEXT NOT NULL,
 geometry_type_name TEXT NOT NULL, srs_id INTEGER NOT NULL,
 z INTEGER NOT NULL, m INTEGER NOT NULL,
 PRIMARY KEY(table_name,column_name)
);
CREATE TABLE gpkg_spatial_ref_sys (
 srs_name TEXT NOT NULL, srs_id INTEGER NOT NULL PRIMARY KEY,
 organization TEXT NOT NULL, organization_coordsys_id INTEGER NOT NULL,
 definition TEXT NOT NULL, description TEXT
);
INSERT OR IGNORE INTO gpkg_spatial_ref_sys VALUES (
 'WGS 84 geodetic',4326,'EPSG',4326,
 'GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433],AUTHORITY["EPSG","4326"]]',
 'World Geodetic System 1984');
CREATE TABLE network_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE nodes (
 node_id TEXT PRIMARY KEY, name TEXT NOT NULL, kind TEXT NOT NULL,
 continent TEXT NOT NULL, longitude DOUBLE NOT NULL, latitude DOUBLE NOT NULL,
 geom_wkt TEXT NOT NULL, geom BLOB NOT NULL
);
CREATE TABLE links (
 link_id TEXT PRIMARY KEY, from_node TEXT NOT NULL, to_node TEXT NOT NULL,
 mode TEXT NOT NULL, stage TEXT NOT NULL, name TEXT NOT NULL,
 distance_km DOUBLE NOT NULL, travel_time_hours DOUBLE NOT NULL,
 cost_eur_per_tonne DOUBLE NOT NULL, cost_eur_per_passenger DOUBLE NOT NULL,
 bidirectional INTEGER NOT NULL,
 physical_possible INTEGER NOT NULL, geom_wkt TEXT NOT NULL, geom BLOB NOT NULL
);
INSERT INTO gpkg_contents(table_name,data_type,identifier,description,min_x,min_y,max_x,max_y,srs_id)
VALUES ('nodes','features','world_lod0_nodes','Multimodal network nodes',-180,-90,180,90,4326);
INSERT INTO gpkg_contents(table_name,data_type,identifier,description,min_x,min_y,max_x,max_y,srs_id)
VALUES ('links','features','world_lod0_links','Bidirectional multimodal transport links',-180,-90,180,90,4326);
INSERT INTO gpkg_geometry_columns VALUES ('nodes','geom','POINT',4326,0,0);
INSERT INTO gpkg_geometry_columns VALUES ('links','geom','LINESTRING',4326,0,0);
INSERT INTO network_metadata VALUES ('description','World LOD0 multimodal transport network: 7 continental centroids, local airport and seaport gateways, and simplified road/rail/sea/air links');
INSERT INTO network_metadata VALUES ('model','Access-egress and linehaul stages; links are bidirectional where the table flag is set');
INSERT INTO network_metadata VALUES ('source','Open public representative coordinates and deliberately estimated speeds/costs; no proprietary routing data');
INSERT INTO network_metadata VALUES ('cost_unit','Estimated EUR per metric tonne');
INSERT INTO network_metadata VALUES ('passenger_cost_unit','Estimated EUR per passenger per link');
""")

def gpkg_header():
    # GeoPackage Binary header: GP magic (2 bytes), version, flags,
    # and EPSG:4326 SRID. No envelope bytes are present (8-byte header total).
    return b"GP" + bytes([0x00, 0x01]) + struct.pack("<i", 4326)

def point(lon, lat):
    wkb = bytes([1]) + struct.pack('<I', 1) + struct.pack('<dd', lon, lat)
    return gpkg_header() + wkb

def line(ax, ay, bx, by):
    wkb = bytes([1]) + struct.pack('<I', 2) + struct.pack('<I', 2)
    wkb += struct.pack('<dd', ax, ay) + struct.pack('<dd', bx, by)
    return gpkg_header() + wkb

con.execute("BEGIN")
for n in data["nodes"]:
    wkt = "POINT ({} {})".format(n["lon"], n["lat"])
    con.execute("INSERT INTO nodes VALUES (?,?,?,?,?,?,?,?)", (
        n["id"],n["name"],n["kind"],n["continent"],n["lon"],n["lat"],wkt,point(n["lon"],n["lat"])))
for e in data["links"]:
    wkt = "LINESTRING ({} {}, {} {})".format(
        e["a_lon"],e["a_lat"],e["b_lon"],e["b_lat"])
    con.execute("INSERT INTO links VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)", (
        e["id"],e["from"],e["to"],e["mode"],e["stage"],e["name"],e["distance"],e["time"],e["cost_eur_per_tonne"],e["cost_eur_per_passenger"],1,e["physical"],wkt,line(e["a_lon"],e["a_lat"],e["b_lon"],e["b_lat"])))
con.execute("CREATE INDEX idx_links_from ON links(from_node)")
con.execute("CREATE INDEX idx_links_to ON links(to_node)")
con.execute("CREATE INDEX idx_links_mode ON links(mode)")
con.commit()
con.close()
