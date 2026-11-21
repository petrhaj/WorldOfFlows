import json
import sqlite3

SPATIAL_DIR = "data/spatial"
DATABASE = f"{SPATIAL_DIR}/world_lod0.gpkg"

con = sqlite3.connect(DATABASE)
con.row_factory = sqlite3.Row

nodes = {"type": "FeatureCollection", "features": []}
for row in con.execute("SELECT * FROM nodes"):
    properties = {key: row[key] for key in row.keys() if key not in ("geom", "geom_wkt")}
    nodes["features"].append({
        "type": "Feature",
        "properties": properties,
        "geometry": {"type": "Point", "coordinates": [row["longitude"], row["latitude"]]},
    })

with open(f"{SPATIAL_DIR}/nodes.geojson", "w", encoding="utf-8") as f:
    json.dump(nodes, f)

links = {"type": "FeatureCollection", "features": []}
coordinates = {row["node_id"]: (row["longitude"], row["latitude"])
               for row in con.execute("SELECT node_id, longitude, latitude FROM nodes")}
for row in con.execute("SELECT * FROM links"):
    properties = {key: row[key] for key in row.keys() if key not in ("geom", "geom_wkt")}
    ax, ay = coordinates[row["from_node"]]
    bx, by = coordinates[row["to_node"]]
    links["features"].append({
        "type": "Feature",
        "properties": properties,
        "geometry": {"type": "LineString", "coordinates": [[ax, ay], [bx, by]]},
    })

with open(f"{SPATIAL_DIR}/links.geojson", "w", encoding="utf-8") as f:
    json.dump(links, f)
con.close()
