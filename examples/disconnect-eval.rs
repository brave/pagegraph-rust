//! Prints out all URLs from a page that match the given network filter adblock rule.

use pagegraph::from_xml::read_from_file;
use pagegraph::types::NodeType;

use std::collections::HashSet;
use std::io::{Read, Write};

// (Url, Request Type)
type BlockedRequests = HashSet<(String, String)>;

#[derive(serde::Deserialize, serde::Serialize)]
struct PageReport {
    page_url: String,
    total_resources: usize,
    brave_count: usize,
    brave_no_disconnect_count: usize,
    brave_no_tags_count: usize,
    brave_no_tags_no_disconnect_count: usize,
    ublock_origin_count: usize,
    brave_blocked: BlockedRequests,
    brave_no_disconnect_blocked: BlockedRequests,
    brave_no_tags_blocked: BlockedRequests,
    brave_no_tags_no_disconnect_blocked: BlockedRequests,
    ublock_origin_blocked: BlockedRequests,
}

fn run_adblock_configuration(graph: &pagegraph::graph::PageGraph, engine: &adblock::engine::Engine) -> BlockedRequests {
    let root_url = graph.root_url();

    let mut blocked_requests = BlockedRequests::new();

    graph.nodes
        .iter()
        .for_each(|(id, node)| match &node.node_type {
            NodeType::Resource { url } => {
                let request_types = graph.resource_request_types(id);
                request_types.into_iter().for_each(|request_type| {
                    let block_result = engine.check_network_urls(&url, &root_url, &request_type);
                    // If the resource matches without an exception, or with an exception and important
                    if block_result.matched && (block_result.exception.is_none() || block_result.important) {
                        // Get all downstream resources
                        let downstream_resources = graph.all_downstream_effects_of(&id);
                        // Flag this resource as blocked
                        blocked_requests.insert((url.to_string(), request_type));
                        // Flag each of its downstream resources as blocked
                        downstream_resources.into_iter().for_each(|(id, node)| { match &node.node_type {
                            NodeType::Resource { url } => {
                                let request_types = graph.resource_request_types(&id);
                                request_types.into_iter().for_each(|request_type| {
                                    blocked_requests.insert((url.to_string(), request_type));
                                });
                            }
                            _ => (),
                        }});
                    }
                });
            }
            _ => (),
        });

    blocked_requests
}

/// If no args are supplied, serialize engine configurations and exit.
///
/// If one arg is supplied, interpret it as a graph file and generate a `PageReport` for it,
/// writing it to the same directory with the `.blocked` suffix
///
/// If more than one arg is supplied, interpret them as multiple PageReport files and run analysis
/// on the entire set.
fn main() {
    if std::env::args().len() <= 1 {
        let brave_engine = {
            let rule_locations: Vec<String> = adblock::filter_lists::default::default_lists().iter().map(|fl| fl.url.to_owned()).collect();
            let rules = rule_locations.iter().map(|url| {
                let body = reqwest::get(url).unwrap().text().unwrap();
                body.split('\n').map(|line| {
                    line.to_owned()
                }).collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();
            adblock::engine::Engine::from_rules(&rules)
        };
        let mut file = std::fs::File::create("brave_engine.bin").unwrap();
        file.write_all(&brave_engine.serialize().unwrap()).unwrap();

        let brave_no_disconnect_engine = {
            let rule_locations: Vec<String> = adblock::filter_lists::default::default_lists().iter().map(|fl| fl.url.to_owned()).collect();
            let rules = rule_locations.iter()
                .filter(|url| *url != "https://raw.githubusercontent.com/brave/adblock-lists/master/brave-disconnect.txt")
                .map(|url| {
                    let body = reqwest::get(url).unwrap().text().unwrap();
                    body.split('\n').map(|line| {
                        line.to_owned()
                    }).collect::<Vec<_>>()
                })
                .flatten()
                .collect::<Vec<_>>();
            adblock::engine::Engine::from_rules(&rules)
        };
        let mut file = std::fs::File::create("brave_no_disconnect_engine.bin").unwrap();
        file.write_all(&brave_no_disconnect_engine.serialize().unwrap()).unwrap();

        let ublock_origin_engine = {
            let rule_locations: Vec<&str> = vec![
                "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/unbreak.txt",
                "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/resource-abuse.txt",
                "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/privacy.txt",
                "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/badware.txt",
                "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters.txt",
                "https://easylist.to/easylist/easylist.txt",
                "https://easylist.to/easylist/easyprivacy.txt",
                "https://www.malwaredomainlist.com/hostslist/hosts.txt",
                "http://malwaredomains.lehigh.edu/files/justdomains",
                "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=hosts&showintro=1&mimetype=plaintext",
            ];
            let rules = rule_locations.iter().map(|url| {
                let body = reqwest::get(&url.to_string()).unwrap().text().unwrap();
                body.split('\n').map(|line| {
                    line.to_owned()
                }).collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();
            adblock::engine::Engine::from_rules(&rules)
        };
        let mut file = std::fs::File::create("ublock_origin_engine.bin").unwrap();
        file.write_all(&ublock_origin_engine.serialize().unwrap()).unwrap();

        return;
    } else if std::env::args().len() == 2 {
        fn engine_from_file(file: &str) -> adblock::engine::Engine {
            let mut file = std::fs::File::open(file).unwrap();
            let mut v = vec![];
            file.read_to_end(&mut v).unwrap();
            let mut engine = adblock::engine::Engine::from_rules(&[]);
            engine.deserialize(&v).unwrap();
            engine
        }

        let mut brave_engine = engine_from_file("brave_engine.bin");
        brave_engine.tags_enable(&["fb-embeds", "twitter-embeds"]);
        let mut brave_no_disconnect_engine = engine_from_file("brave_no_disconnect_engine.bin");
        brave_no_disconnect_engine.tags_enable(&["fb-embeds", "twitter-embeds"]);
        let brave_no_tags_engine = engine_from_file("brave_engine.bin");
        let brave_no_tags_no_disconnect_engine = engine_from_file("brave_no_disconnect_engine.bin");
        let ublock_origin_engine = engine_from_file("ublock_origin_engine.bin");

        std::env::args().skip(1).for_each(|graph_file| {
            let graph = read_from_file(&graph_file);

            let brave_blocked = run_adblock_configuration(&graph, &brave_engine);
            let brave_no_disconnect_blocked = run_adblock_configuration(&graph, &brave_no_disconnect_engine);
            let brave_no_tags_blocked = run_adblock_configuration(&graph, &brave_no_tags_engine);
            let brave_no_tags_no_disconnect_blocked = run_adblock_configuration(&graph, &brave_no_tags_no_disconnect_engine);
            let ublock_origin_blocked = run_adblock_configuration(&graph, &ublock_origin_engine);

            dbg!(brave_blocked.difference(&brave_no_disconnect_blocked));

            let report = PageReport {
                page_url: graph.root_url(),
                total_resources: graph.nodes.iter().filter(|(_, node)| match &node.node_type {
                    NodeType::Resource { .. } => true,
                    _ => false,
                }).count(),
                brave_count: brave_blocked.len(),
                brave_no_disconnect_count: brave_no_disconnect_blocked.len(),
                brave_no_tags_count: brave_no_tags_blocked.len(),
                brave_no_tags_no_disconnect_count: brave_no_tags_no_disconnect_blocked.len(),
                ublock_origin_count: ublock_origin_blocked.len(),
                brave_blocked,
                brave_no_disconnect_blocked,
                brave_no_tags_blocked,
                brave_no_tags_no_disconnect_blocked,
                ublock_origin_blocked,
            };

            let mut file = std::fs::File::create(format!("{}.blocked", graph_file)).unwrap();
            file.write_all(serde_json::to_string(&report).unwrap().as_bytes()).unwrap();
        });
    } else {
        let mut total_number_reports = 0;
        let mut total_number_identical = 0;
        let mut total_number_blocked_requests_a = 0;
        let mut total_number_blocked_requests_b = 0;
        let mut total_number_differences = 0;
        let mut num_sites_by_num_differences = std::collections::BTreeMap::<usize, usize>::new();
        let mut all_missed_endpoints = HashSet::new();
        let mut commonly_missed_domains = std::collections::HashMap::<String, usize>::new();
        std::env::args().skip(1).for_each(|report_file| {
            let file = std::fs::File::open(report_file).unwrap();
            let report: PageReport = serde_json::from_reader(std::io::BufReader::new(file)).unwrap();

            // Change these to modify the A/B comparison
            let a_count = report.brave_count;
            let a_blocked = report.brave_blocked;
            let b_count = report.ublock_origin_count;
            let b_blocked = report.ublock_origin_blocked;

            total_number_reports += 1;
            if a_count == b_count {
                total_number_identical += 1;
            }
            total_number_blocked_requests_a += a_count;
            total_number_blocked_requests_b += b_count;
            let differences = a_blocked.difference(&b_blocked).collect::<HashSet<_>>();
            total_number_differences += differences.len();
            *num_sites_by_num_differences.entry(differences.len()).or_insert(0) += 1;
            differences.iter().for_each(|(missed_endpoint, _type)| {
                all_missed_endpoints.insert(missed_endpoint.to_string());
                let endpoint_host = url::Url::parse(&missed_endpoint).ok().map(|url| url.host_str().map(|host_str| host_str.to_string())).flatten();
                if let Some(endpoint_host) = endpoint_host {
                    let domain = get_domain(&endpoint_host);
                    *commonly_missed_domains.entry(domain).or_insert(0) += 1;
                }
            });
        });
        dbg!(total_number_reports);
        dbg!(total_number_identical);
        dbg!(total_number_blocked_requests_a);
        dbg!(total_number_blocked_requests_b);
        dbg!(total_number_differences);
        dbg!(num_sites_by_num_differences);
        // domains of most commonly missed endpoints + number of times missed
        let mut commonly_missed_domains = commonly_missed_domains.iter().collect::<Vec<_>>();
        commonly_missed_domains.sort_by(|(_, a), (_ , b)| b.cmp(a));
        for i in 0..commonly_missed_domains.len() {
            let (domain, count) = commonly_missed_domains[i];
            println!("{}, {}", domain, count);
        }
    }
}

pub fn get_domain(host: &str) -> String {
    let source_hostname = host;
    let source_domain = source_hostname.parse::<addr::DomainName>().expect("Source URL domain could not be parsed");
    let source_domain = &source_hostname[source_hostname.len() - source_domain.root().to_str().len()..];
    source_domain.to_string()
}
