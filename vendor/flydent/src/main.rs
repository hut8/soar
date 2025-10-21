use flydent::{EntityResult, Parser};
use std::collections::HashMap;
use std::env;

fn entity_result_to_json(result: &EntityResult) -> serde_json::Value {
    match result {
        EntityResult::Country {
            nation,
            description,
            iso2,
            iso3,
            canonical_callsign,
        } => {
            let mut map = std::collections::HashMap::new();
            map.insert(
                "nation".to_string(),
                serde_json::Value::String(nation.clone()),
            );
            map.insert(
                "description".to_string(),
                serde_json::Value::String(description.clone()),
            );
            map.insert("iso2".to_string(), serde_json::Value::String(iso2.clone()));
            map.insert("iso3".to_string(), serde_json::Value::String(iso3.clone()));
            map.insert(
                "canonical_callsign".to_string(),
                serde_json::Value::String(canonical_callsign.clone()),
            );
            serde_json::Value::Object(map.into_iter().collect())
        }
        EntityResult::Organization {
            name,
            description,
            canonical_callsign,
        } => {
            let mut map = std::collections::HashMap::new();
            map.insert("name".to_string(), serde_json::Value::String(name.clone()));
            map.insert(
                "description".to_string(),
                serde_json::Value::String(description.clone()),
            );
            map.insert(
                "canonical_callsign".to_string(),
                serde_json::Value::String(canonical_callsign.clone()),
            );
            serde_json::Value::Object(map.into_iter().collect())
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: {} [--icao24bit] <callsign1> [callsign2] ...",
            args[0]
        );
        eprintln!("       {} --help", args[0]);
        std::process::exit(1);
    }

    if args.len() == 2 && (args[1] == "--help" || args[1] == "-h") {
        println!("Aircraft Registration Parser - Rust port of flydenity");
        println!();
        println!("USAGE:");
        println!("    {} [OPTIONS] <callsign>...", args[0]);
        println!();
        println!("ARGS:");
        println!("    <callsign>...    Aircraft callsign(s) or ICAO 24-bit identifier(s) to parse");
        println!();
        println!("OPTIONS:");
        println!("        --icao24bit    Parse arguments as ICAO 24-bit identifiers instead of callsigns");
        println!("    -h, --help         Print help information");
        println!();
        println!("EXAMPLES:");
        println!(
            "    {} T6ABC N123ABC        Parse aircraft callsigns",
            args[0]
        );
        println!(
            "    {} --icao24bit 700123   Parse ICAO 24-bit identifier",
            args[0]
        );
        return;
    }

    let parser = Parser::new();
    let mut results = HashMap::new();
    let mut icao_mode = false;
    let mut arg_start = 1;

    if args.len() > 1 && args[1] == "--icao24bit" {
        icao_mode = true;
        arg_start = 2;
    }

    if args.len() <= arg_start {
        eprintln!("Error: No callsigns provided");
        std::process::exit(1);
    }

    for arg in args.iter().skip(arg_start) {
        let result = if icao_mode {
            parser.parse(arg, false, true)
        } else {
            parser.parse_simple(arg)
        };

        match result {
            Some(entity_result) => {
                results.insert(arg.clone(), entity_result_to_json(&entity_result));
            }
            None => {
                results.insert(arg.clone(), serde_json::Value::Null);
            }
        }
    }

    let json_output = serde_json::to_string(&results).unwrap();
    println!("{}", json_output);
}
