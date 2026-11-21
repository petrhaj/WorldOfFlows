import os
import sqlite3

DATABASE = "data/spatial/world_lod0_routed.gpkg"
con = sqlite3.connect(DATABASE)
print("size:", os.path.getsize(DATABASE), "bytes")
print("tables:", [r[0] for r in con.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY 1")])
print("pax column:", [r for r in con.execute("PRAGMA table_info(links)") if r[1] == "pax_flow"])
print("pax total:", con.execute("SELECT sum(pax_flow) FROM links").fetchone()[0])
print("air pax:", con.execute("SELECT sum(pax_flow) FROM links WHERE mode='air'").fetchone()[0])
print("flow rows:", con.execute("SELECT count(*) FROM intercontinental_flows").fetchone()[0])
print("flow tours:", con.execute("SELECT sum(pax_tours) FROM intercontinental_flows").fetchone()[0])
print("top links:", con.execute("SELECT link_id,pax_flow FROM links WHERE pax_flow>0 ORDER BY pax_flow DESC LIMIT 5").fetchall())
print("geometry metadata:", con.execute("SELECT * FROM gpkg_geometry_columns WHERE table_name='intercontinental_flows'").fetchall())
con.close()
