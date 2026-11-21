import shutil
import sqlite3
import struct
import sys
import os

source, destination, flows_json = sys.argv[1], sys.argv[2], sys.argv[3]
import json

if not os.path.exists(destination):
    shutil.copy2(source, destination)
con = sqlite3.connect(destination)
with open(flows_json, "r", encoding="utf-8") as f:
    payload = json.load(f)

columns = {row[1] for row in con.execute("PRAGMA table_info(links)")}
if "pax_flow" not in columns:
    con.execute("ALTER TABLE links ADD COLUMN pax_flow REAL NOT NULL DEFAULT 0")
else:
    con.execute("UPDATE links SET pax_flow=0")
con.execute("""
CREATE TABLE IF NOT EXISTS intercontinental_flows (
    flow_id TEXT PRIMARY KEY,
    origin_continent TEXT NOT NULL,
    destination_continent TEXT NOT NULL,
    origin_airport TEXT NOT NULL,
    destination_airport TEXT NOT NULL,
    pax_tours REAL NOT NULL,
    mode TEXT NOT NULL DEFAULT 'air',
    geom BLOB NOT NULL
)
""")
con.execute("DELETE FROM intercontinental_flows")


def gpkg_header():
    return b"GP" + bytes([0x00, 0x01]) + struct.pack("<i", 4326)


def line(ax, ay, bx, by):
    wkb = bytes([0x01]) + struct.pack("<I", 2) + struct.pack("<I", 2)
    wkb += struct.pack("<dd", ax, ay) + struct.pack("<dd", bx, by)
    return gpkg_header() + wkb


con.execute("INSERT OR REPLACE INTO network_metadata(key,value) VALUES ('flow_model','Two tours per person; multinomial logit destination choice with 99.99 percent same-continent retention target')")
con.execute("INSERT OR REPLACE INTO network_metadata(key,value) VALUES ('population_source','United Nations World Population Prospects 2024; simplified 2025 continental population totals')")
con.execute("INSERT OR REPLACE INTO network_metadata(key,value) VALUES ('logit_parameters','size coefficient=1.0 on ln(destination population), time coefficient=-0.01 per hour, cost coefficient=-0.00002 per USD/tonne')")

for row in payload["link_flows"]:
    con.execute("UPDATE links SET pax_flow=pax_flow+? WHERE link_id=?",
                (row["pax_tours"], row["link_id"]))

node_map = {
    row[0]: (row[1], row[2])
    for row in con.execute("SELECT node_id, longitude, latitude FROM nodes")
}
for row in payload["intercontinental_flows"]:
    ax, ay = node_map[row["origin_airport"]]
    bx, by = node_map[row["destination_airport"]]
    con.execute("INSERT OR REPLACE INTO intercontinental_flows VALUES (?,?,?,?,?,?,?,?)", (
        row["flow_id"], row["origin_continent"], row["destination_continent"],
        row["origin_airport"], row["destination_airport"], row["pax_tours"],
        "air", line(ax, ay, bx, by)))

con.execute("INSERT OR IGNORE INTO gpkg_contents(table_name,data_type,identifier,description,last_change,min_x,min_y,max_x,max_y,srs_id) VALUES ('intercontinental_flows','features','world_lod0_intercontinental_flows','Intercontinental passenger tours assigned to direct air legs',(strftime('%Y-%m-%dT%H:%M:%fZ','now')),-180,-90,180,90,4326)")
con.execute("INSERT OR IGNORE INTO gpkg_geometry_columns(table_name,column_name,geometry_type_name,srs_id,z,m) VALUES ('intercontinental_flows','geom','LINESTRING',4326,0,0)")
con.commit()
con.close()
