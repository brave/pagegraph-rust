#[allow(dead_code)]
extern crate adblock;
extern crate clap;

use std::vec;

use adblock::filters::network::NetworkFilter;
use clap::{arg_enum, value_t_or_exit, App, Arg};

use pagegraph::{from_xml, queries};

arg_enum! {
  #[derive(PartialEq, Debug)]
  pub enum QueryType {
    Scripts,
    Fingerprinting,
    Storage
  }
}

fn main() {
    let matches = App::new("PageGraph Querier")
        .version("0.1")
        .about("Get useful info out of PageGraph files")
        .arg(
            Arg::with_name("INPUT")
                .short("i")
                .long("input")
                .value_name("INPUT")
                .help("The PageGraph GraphML file to query.")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("QUERY")
                .short("q")
                .long("query")
                .value_name("QUERY")
                .help("The type of query to perform against the graph.")
                .possible_values(&QueryType::variants())
                .case_insensitive(true)
                .required(true),
        )
        .arg(
            Arg::with_name("FILTER")
                .short("f")
                .long("filter")
                .value_name("FILTER")
                .help(
                    "Optional AdBlock Plus filter to select script units by. \
             Otherwise, selects all scripts.",
                )
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .short("o")
                .long("output")
                .value_name("OUTPUT")
                .help("Path to write results to. Otherwise prints to STDOUT."),
        )
        .arg(
            Arg::with_name("VERBOSE")
                .short("v")
                .long("verbose")
                .value_name("FILTER")
                .help("Print descriptive, debugging text."),
        )
        .get_matches();

    let is_verbose = matches.is_present("VERBOSE");

    let filter: Option<NetworkFilter> = match matches.value_of("FILTER") {
        Some(x) => match NetworkFilter::parse(x, is_verbose) {
            Ok(x) => {
                if is_verbose {
                    println!("successfully parsed network filter");
                }
                Some(x)
            }
            Err(e) => panic!(e),
        },
        None => None,
    };

    let graph_path = matches.value_of("INPUT").unwrap();
    let graph = from_xml::read_from_file(graph_path);

    let query_type = value_t_or_exit!(matches, "QUERY", QueryType);

    print!(
        "{}",
        queries::caused_storage(&graph, &filter, is_verbose).len()
    );
}
