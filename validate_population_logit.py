import sqlite3

DATABASE = "data/spatial/world_lod0_routed.gpkg"
con = sqlite3.connect(DATABASE)
antarctica = con.execute(
    "SELECT sum(pax_tours) FROM intercontinental_flows WHERE destination_continent='Antarctica'"
).fetchone()[0]
total = con.execute("SELECT sum(pax_tours) FROM intercontinental_flows").fetchone()[0]
print("Antarctica inbound:", antarctica)
print("Antarctica share:", antarctica / total)
print("Largest destinations:", con.execute(
    "SELECT destination_continent, sum(pax_tours) FROM intercontinental_flows "
    "GROUP BY destination_continent ORDER BY 2 DESC"
).fetchall())
print("Antarctica origins:", con.execute(
    "SELECT origin_continent, pax_tours FROM intercontinental_flows "
    "WHERE destination_continent='Antarctica' ORDER BY pax_tours DESC"
).fetchall())
con.close()
