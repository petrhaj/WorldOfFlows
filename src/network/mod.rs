use std::collections::HashMap;
use std::fs;
use std::process::Command;

#[derive(Clone)]
struct Node {
    id: String,
    name: String,
    kind: String,
    continent: String,
    lon: f64,
    lat: f64,
}

#[derive(Clone)]
struct Link {
    id: String,
    from: String,
    to: String,
    mode: String,
    stage: String,
    name: String,
}

fn haversine_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    let r = 6371.0088;
    let dlat = (b.1 - a.1).to_radians();
    let dlon = (b.0 - a.0).to_radians();
    let lat1 = a.1.to_radians();
    let lat2 = b.1.to_radians();
    let h = (dlat / 2.0).sin().powi(2)
        + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * r * h.sqrt().asin()
}

fn js(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn n(id: &str, name: &str, kind: &str, continent: &str, lon: f64, lat: f64) -> Node {
    Node { id: id.into(), name: name.into(), kind: kind.into(), continent: continent.into(), lon, lat }
}

fn l(id: &str, from: &str, to: &str, mode: &str, stage: &str, name: &str) -> Link {
    Link { id: id.into(), from: from.into(), to: to.into(), mode: mode.into(), stage: stage.into(), name: name.into() }
}

pub fn generate_network() -> Result<(), Box<dyn std::error::Error>> {
    let nodes = vec![
        n("NA_CTR", "North America centroid", "centroid", "North America", -84.4277, 33.6407),
        n("EU_CTR", "Europe centroid", "centroid", "Europe", 8.6821, 50.1109),
        n("AS_CTR", "Asia centroid", "centroid", "Asia", 103.8198, 1.3521),
        n("AF_CTR", "Africa centroid", "centroid", "Africa", 3.3792, 6.5244),
        n("SA_CTR", "South America centroid", "centroid", "South America", -43.1729, -22.9068),
        n("AU_CTR", "Australia centroid", "centroid", "Australia", 151.2093, -33.8688),
        n("AN_CTR", "Antarctica centroid", "centroid", "Antarctica", 166.6863, -77.8419),

        n("NA_AP", "Hartsfield-Jackson Atlanta Airport", "airport", "North America", -84.4277, 33.6407),
        n("EU_AP", "Frankfurt Airport", "airport", "Europe", 8.5706, 50.0379),
        n("AS_AP", "Singapore Changi Airport", "airport", "Asia", 103.9915, 1.3644),
        n("AF_AP", "Murtala Muhammed Airport", "airport", "Africa", 3.3211, 6.5774),
        n("SA_AP", "Rio de Janeiro/Galeao Airport", "airport", "South America", -43.2495, -22.8090),
        n("AU_AP", "Sydney Kingsford Smith Airport", "airport", "Australia", 151.1772, -33.9461),
        n("AN_AP", "Phoenix Airfield / Mt McMurdo gateway", "airport", "Antarctica", 166.6680, -77.8610),

        n("NA_SP", "Port of Savannah", "seaport", "North America", -81.0912, 32.0809),
        n("EU_SP", "Port of Rotterdam", "seaport", "Europe", 4.1267, 51.9224),
        n("AS_SP", "Port of Singapore", "seaport", "Asia", 103.8000, 1.2600),
        n("AF_SP", "Port of Lagos / Apapa", "seaport", "Africa", 3.3667, 6.4333),
        n("SA_SP", "Port of Rio de Janeiro", "seaport", "South America", -43.1736, -22.8948),
        n("AU_SP", "Port Botany / Sydney", "seaport", "Australia", 151.2200, -33.9700),
        n("AN_SP", "McMurdo seasonal sea ice access", "seaport", "Antarctica", 166.5000, -77.8500),

        n("EU_AS_1", "Istanbul land gateway", "gateway", "Europe/Asia", 28.9784, 41.0082),
        n("EU_AS_2", "Almaty land gateway", "gateway", "Asia", 76.9450, 43.2380),
    ];

    let mut links: Vec<Link> = Vec::new();
    for (ct, ap, sp) in [
        ("NA_CTR","NA_AP","NA_SP"),("EU_CTR","EU_AP","EU_SP"),("AS_CTR","AS_AP","AS_SP"),
        ("AF_CTR","AF_AP","AF_SP"),("SA_CTR","SA_AP","SA_SP"),("AU_CTR","AU_AP","AU_SP"),("AN_CTR","AN_AP","AN_SP")
    ] {
        links.push(l(&format!("C_{ct}_{ap}"), ct, ap, "connector", "access-egress", "centroid to airport connector"));
        links.push(l(&format!("C_{ct}_{sp}"), ct, sp, "connector", "access-egress", "centroid to seaport connector"));
        if ct != "AN_CTR" {
            links.push(l(&format!("RD_{ct}_{ap}"), ct, ap, "road", "access-egress", "local road access"));
            links.push(l(&format!("RD_{ct}_{sp}"), ct, sp, "road", "access-egress", "local road access"));
        }
    }

    for (ct, ap, sp) in [
        ("NA_CTR","NA_AP","NA_SP"),("EU_CTR","EU_AP","EU_SP"),("AS_CTR","AS_AP","AS_SP"),
        ("AF_CTR","AF_AP","AF_SP"),("SA_CTR","SA_AP","SA_SP"),("AU_CTR","AU_AP","AU_SP")
    ] {
        links.push(l(&format!("RL_{ct}_{ap}"), ct, ap, "rail", "access-egress", "local rail access"));
        links.push(l(&format!("RL_{ct}_{sp}"), ct, sp, "rail", "access-egress", "local rail access"));
    }

    let mesh = [
        ("NA_AP","NA_SP","North America"),("EU_AP","EU_SP","Europe"),("AS_AP","AS_SP","Asia"),
        ("AF_AP","AF_SP","Africa"),("SA_AP","SA_SP","South America"),("AU_AP","AU_SP","Australia"),("AN_AP","AN_SP","Antarctica")
    ];
    for i in 0..mesh.len() {
        for j in (i + 1)..mesh.len() {
            links.push(l(&format!("A_{i}_{j}"), mesh[i].0, mesh[j].0, "air", "linehaul", &format!("{} to {} air line", mesh[i].2, mesh[j].2)));
            links.push(l(&format!("S_{i}_{j}"), mesh[i].1, mesh[j].1, "sea", "linehaul", &format!("{} to {} sea line", mesh[i].2, mesh[j].2)));
        }
    }

    links.push(l("RD_EU_IST", "EU_CTR", "EU_AS_1", "road", "linehaul", "Europe to Istanbul road corridor"));
    links.push(l("RD_IST_ALM", "EU_AS_1", "EU_AS_2", "road", "linehaul", "Istanbul to Almaty road corridor"));
    links.push(l("RD_ALM_AS", "EU_AS_2", "AS_CTR", "road", "linehaul", "Almaty to Asia road corridor"));
    links.push(l("RL_EU_IST", "EU_CTR", "EU_AS_1", "rail", "linehaul", "Europe to Istanbul rail corridor"));
    links.push(l("RL_IST_ALM", "EU_AS_1", "EU_AS_2", "rail", "linehaul", "Istanbul to Almaty rail corridor"));
    links.push(l("RL_ALM_AS", "EU_AS_2", "AS_CTR", "rail", "linehaul", "Almaty to Asia rail corridor"));

    let node_map: HashMap<String, &Node> = nodes.iter().map(|n| (n.id.clone(), n)).collect();

    let mut json = String::from("{\"nodes\":[");
    for (i, n) in nodes.iter().enumerate() {
        if i > 0 { json.push(','); }
        json.push_str(&format!("{{\"id\":{},\"name\":{},\"kind\":{},\"continent\":{},\"lon\":{},\"lat\":{}}}", js(&n.id), js(&n.name), js(&n.kind), js(&n.continent), n.lon, n.lat));
    }
    json.push_str("],\"links\":[");
    for (i, e) in links.iter().enumerate() {
        if i > 0 { json.push(','); }
        let a = node_map[&e.from];
        let b = node_map[&e.to];
        let mut distance = haversine_km((a.lon, a.lat), (b.lon, b.lat));
        let (speed, cost_per_km) = match e.mode.as_str() {
            "air" => (850.0, 0.18), "sea" => (24.0, 0.050), "road" => (60.0, 1.20),
            "rail" => (50.0, 0.55), _ => (40.0, 0.85),
        };
        if e.mode == "connector" { distance = distance.max(15.0); }
        let time = distance / speed;
        let mut cost = distance * cost_per_km;
        if e.mode == "connector" { cost += 25.0; }
        if e.mode == "air" { cost += 180.0; }
        if e.mode == "sea" { cost += 60.0; }
        let physical = if e.mode == "air" || e.mode == "sea" || e.mode == "connector" || e.from != "AN_CTR" && e.to != "AN_CTR" { 1 } else { 0 };
        json.push_str(&format!("{{\"id\":{},\"from\":{},\"to\":{},\"mode\":{},\"stage\":{},\"name\":{},\"distance\":{},\"time\":{},\"cost\":{},\"physical\":{},\"a_lon\":{},\"a_lat\":{},\"b_lon\":{},\"b_lat\":{}}}", js(&e.id), js(&e.from), js(&e.to), js(&e.mode), js(&e.stage), js(&e.name), distance, time, cost, physical, a.lon, a.lat, b.lon, b.lat));
    }
    json.push_str("]}");

    fs::create_dir_all("data/spatial")?;
    let network_json = "data/spatial/world_lod0_network.json";
    let gpkg = "data/spatial/world_lod0.gpkg";

    fs::write(network_json, &json)?;
    if fs::metadata(gpkg).is_err() {
        let status = Command::new("python")
            .args(["src/write_gpkg.py", network_json, gpkg])
            .status()?;
        if !status.success() { return Err("GeoPackage writer failed".into()); }
        println!("Created {} ({} nodes, {} links)", gpkg, nodes.len(), links.len());
    } else {
        println!("Preserved existing {} ({} nodes, {} links)", gpkg, nodes.len(), links.len());
    }

    fs::remove_file(network_json)?;
    Ok(())
}
