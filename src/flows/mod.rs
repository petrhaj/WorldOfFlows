use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::process::Command;

#[derive(Debug, Clone)]
struct GpkgNode {
    node_id: String,
    kind: String,
    continent: String,
}

#[derive(Debug, Clone)]
struct GpkgLink {
    link_id: String,
    from_node: String,
    to_node: String,
    mode: String,
    travel_time_hours: f64,
    cost_usd_per_tonne: f64,
}

#[derive(Debug, Clone)]
struct GpkgLoad {
    nodes: Vec<GpkgNode>,
    links: Vec<GpkgLink>,
}

struct Population {
    continent: String,
    population: f64,
}

fn read_populations() -> Result<Vec<Population>, Box<dyn std::error::Error>> {
    let text = fs::read_to_string("data/populations.csv")?;
    let mut populations = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if index == 0 { continue; }
        let fields: Vec<&str> = line.splitn(3, ',').collect();
        if fields.len() >= 2 {
            populations.push(Population {
                continent: fields[0].trim().to_string(),
                population: fields[1].trim().parse()?,
            });
        }
    }
    Ok(populations)
}

fn read_gpkg(source: &str) -> Result<GpkgLoad, Box<dyn std::error::Error>> {
    fs::create_dir_all("data/spatial")?;
    let network_load = "data/spatial/network_load.json";
    let status = Command::new("python")
        .args(["src/read_gpkg.py", source, network_load])
        .status()?;
    if !status.success() { return Err("Failed to read the GeoPackage through Python/SQLite".into()); }
    let text = fs::read_to_string(network_load)?;
    let value: Value = serde_json::from_str(&text)?;
    fs::remove_file(network_load)?;

    let mut nodes = Vec::new();
    if let Some(items) = value.get("nodes").and_then(Value::as_array) {
        for item in items {
            nodes.push(GpkgNode {
                node_id: item.get("node_id").and_then(Value::as_str).unwrap_or_default().to_string(),
                kind: item.get("kind").and_then(Value::as_str).unwrap_or_default().to_string(),
                continent: item.get("continent").and_then(Value::as_str).unwrap_or_default().to_string(),
            });
        }
    }

    let mut links = Vec::new();
    if let Some(items) = value.get("links").and_then(Value::as_array) {
        for item in items {
            links.push(GpkgLink {
                link_id: item.get("link_id").and_then(Value::as_str).unwrap_or_default().to_string(),
                from_node: item.get("from_node").and_then(Value::as_str).unwrap_or_default().to_string(),
                to_node: item.get("to_node").and_then(Value::as_str).unwrap_or_default().to_string(),
                mode: item.get("mode").and_then(Value::as_str).unwrap_or_default().to_string(),
                travel_time_hours: item.get("travel_time_hours").and_then(Value::as_f64).unwrap_or(0.0),
                cost_usd_per_tonne: item.get("cost_usd_per_tonne").and_then(Value::as_f64).unwrap_or(0.0),
            });
        }
    }

    Ok(GpkgLoad { nodes, links })
}

fn dijkstra(
    centroid_ids: &[String],
    nodes: &HashMap<String, GpkgNode>,
    links: &[GpkgLink],
    cost_attribute: CostAttribute,
) -> HashMap<(String, String), f64> {
    let mut adjacency: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    for centroid in centroid_ids {
        adjacency.entry(centroid.clone()).or_default();
    }
    for link in links {
        if !nodes.contains_key(&link.from_node) || !nodes.contains_key(&link.to_node) { continue; }
        let allowed = link.mode.as_str() == "connector"
            || link.mode.as_str() == "road"
            || link.mode.as_str() == "rail"
            || link.mode.as_str() == "air"
            || link.mode.as_str() == "sea";
        if !allowed { continue; }
        let weight = match cost_attribute {
            CostAttribute::Time => link.travel_time_hours,
            CostAttribute::Cost => link.cost_usd_per_tonne,
        };
        adjacency.entry(link.from_node.clone()).or_default().push((link.to_node.clone(), weight));
        adjacency.entry(link.to_node.clone()).or_default().push((link.from_node.clone(), weight));
    }

    let mut result = HashMap::new();
    for source in centroid_ids {
        let mut dist: HashMap<String, f64> = HashMap::new();
        let mut settled: HashMap<String, bool> = HashMap::new();
        dist.insert(source.clone(), 0.0);
        for _ in 0..nodes.len() {
            let mut best: Option<(String, f64)> = None;
            for (node, distance) in &dist {
                if !settled.get(node).copied().unwrap_or(false)
                    && best.as_ref().map(|(_, d)| *distance < *d).unwrap_or(true)
                {
                    best = Some((node.clone(), *distance));
                }
            }
            let Some((current, distance)) = best else { break };
            settled.insert(current.clone(), true);
            if let Some(neighbors) = adjacency.get(&current) {
                for (next, weight) in neighbors {
                    if !dist.contains_key(next) || distance + weight < dist[next] {
                        dist.insert(next.clone(), distance + weight);
                    }
                }
            }
        }
        for destination in centroid_ids {
            result.insert((source.clone(), destination.clone()), dist.get(destination).copied().unwrap_or(f64::INFINITY));
        }
    }
    result
}

#[derive(Clone, Copy)]
enum CostAttribute { Time, Cost }

fn find_air_link(links: &[GpkgLink], from_airport: &str, to_airport: &str) -> Option<String> {
    links.iter().find(|link| {
        link.mode == "air" &&
        ((link.from_node == from_airport && link.to_node == to_airport)
            || (link.from_node == to_airport && link.to_node == from_airport))
    }).map(|link| link.link_id.clone())
}

pub fn calculate_and_route_flows(source: &str, destination: &str) -> Result<(), Box<dyn std::error::Error>> {
    let loaded = read_gpkg(source)?;
    let populations = read_populations()?;

    let mut population_map = HashMap::new();
    for population in &populations {
        population_map.insert(population.continent.clone(), population.population);
    }

    let nodes: HashMap<String, GpkgNode> = loaded.nodes.iter()
        .map(|node| (node.node_id.clone(), node.clone()))
        .collect();

    let centroid_ids: Vec<String> = loaded.nodes.iter()
        .filter(|node| node.kind == "centroid")
        .map(|node| node.node_id.clone())
        .collect();

    let time_matrix = dijkstra(&centroid_ids, &nodes, &loaded.links, CostAttribute::Time);
    let cost_matrix = dijkstra(&centroid_ids, &nodes, &loaded.links, CostAttribute::Cost);

    let mut airport_by_continent: HashMap<String, String> = HashMap::new();
    for node in &loaded.nodes {
        if node.kind == "airport" {
            airport_by_continent.insert(node.continent.clone(), node.node_id.clone());
        }
    }

    let mut link_flows: HashMap<String, f64> = HashMap::new();
    let mut intercontinental_flows: Vec<Value> = Vec::new();

    for origin in &loaded.nodes {
        if origin.kind != "centroid" { continue; }
        let population = population_map.get(&origin.continent).copied().unwrap_or(0.0);
        let total_tours = 2.0 * population;
        let cross_continent_tours = total_tours * 0.0001;

        let mut destinations = Vec::new();
        let mut log_weights = Vec::new();
        for destination in &loaded.nodes {
            if destination.kind != "centroid" || destination.continent == origin.continent { continue; }
            let time = time_matrix.get(&(origin.node_id.clone(), destination.node_id.clone())).copied().unwrap_or(f64::INFINITY);
            let cost = cost_matrix.get(&(origin.node_id.clone(), destination.node_id.clone())).copied().unwrap_or(f64::INFINITY);
            if !time.is_finite() || !cost.is_finite() { continue; }
            let destination_population = population_map
                .get(&destination.continent)
                .copied()
                .unwrap_or(1.0)
                .max(1.0);
            let utility = destination_population.ln()
                - 0.01 * time
                - 0.00002 * cost;
            destinations.push(destination);
            log_weights.push(utility);
        }

        let max_weight = log_weights.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let mut weights = Vec::new();
        let mut denominator = 0.0;
        if max_weight.is_finite() {
            for weight in &log_weights {
                let w = (weight - max_weight).exp();
                weights.push(w);
                denominator += w;
            }
        }

        for (i, destination) in destinations.iter().enumerate() {
            if denominator == 0.0 { continue; }
            let pax_tours = cross_continent_tours * weights[i] / denominator;
            if pax_tours <= 0.0 { continue; }

            let origin_airport = airport_by_continent.get(&origin.continent).cloned().unwrap_or_default();
            let destination_airport = airport_by_continent.get(&destination.continent).cloned().unwrap_or_default();
            let Some(link_id) = find_air_link(&loaded.links, &origin_airport, &destination_airport) else { continue; };

            *link_flows.entry(link_id.clone()).or_insert(0.0) += pax_tours;
            let flow_id = format!("FL_{}_{}", origin.continent.replace(' ', "_"), destination.continent.replace(' ', "_"));
            intercontinental_flows.push(json!({
                "flow_id": flow_id,
                "origin_continent": origin.continent,
                "destination_continent": destination.continent,
                "origin_airport": origin_airport,
                "destination_airport": destination_airport,
                "pax_tours": pax_tours
            }));
        }
    }

    let mut link_flows_vec = Vec::new();
    for (link_id, pax_tours) in link_flows {
        link_flows_vec.push(json!({"link_id": link_id, "pax_tours": pax_tours}));
    }

    let payload = json!({
        "link_flows": link_flows_vec,
        "intercontinental_flows": intercontinental_flows,
        "same_continent_share": 0.9999,
        "tours_per_person": 2.0
    });
    let routed_flows = "data/spatial/routed_flows.json";
    fs::write(routed_flows, serde_json::to_string_pretty(&payload)?)?;

    let status = Command::new("python")
        .args(["src/write_routed_gpkg.py", source, destination, routed_flows])
        .status()?;
    if !status.success() { return Err("Failed to create the routed GeoPackage".into()); }
    fs::remove_file(routed_flows)?;

    println!("Calculated two-tour flows and wrote {}", destination);
    Ok(())
}
