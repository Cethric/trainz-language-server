use std::fs;
use trainz_tdx::{TdxReader, TdxValue};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-tdx-file>", args[0]);
        return;
    }
    let path = &args[1];
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to read {}: {}", path, e);
            return;
        }
    };

    println!("Read {} bytes", data.len());
    let mut reader = TdxReader::new(&data);
    match reader.parse_all() {
        Ok(results) => {
            println!("Parsed {} top-level entries", results.len());
            for (name, value) in &results {
                print_value(name, value, 0);
            }

            let kuids = TdxValue::find_kuids(&results);
            if !kuids.is_empty() {
                println!("\nFound {} KUIDs with metadata:", kuids.len());
                for info in kuids {
                    println!(
                        "  KUID: {}:{}:{}",
                        info.user_id, info.content_id, info.version
                    );
                    if let Some(username) = info.username {
                        println!("    Username: {}", username);
                    }
                    if !info.files.is_empty() {
                        println!("    Files: {:?}", info.files);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error during parsing: {}", e);
        }
    }
}

fn print_value(name: &str, value: &TdxValue, indent: usize) {
    let padding = "  ".repeat(indent);
    match value {
        TdxValue::Container(vec) => {
            println!("{}{}: {{", padding, name);
            for (k, v) in vec {
                print_value(k, v, indent + 1);
            }
            println!("{}}}", padding);
        }
        TdxValue::Int32(i) => println!("{}{}: {} (int32)", padding, name, i),
        TdxValue::Int64(i) => println!("{}{}: {} (int64)", padding, name, i),
        TdxValue::Float64(f) => println!("{}{}: {} (float64)", padding, name, f),
        TdxValue::String(s) => println!("{}{}: \"{}\" (string)", padding, name, s),
        TdxValue::Bool(b) => println!("{}{}: {} (bool)", padding, name, b),
        TdxValue::Kuid(val) => {
            let (u, c, v) = TdxValue::split_kuid(*val);
            println!("{}{}: <kuid2:{}:{}:{}> (kuid)", padding, name, u, c, v);
        }
        TdxValue::AssetDescriptor {
            marker,
            name: dname,
            paths,
            params,
            ..
        } => {
            let path_str = paths.join(", ");
            println!(
                "{}{}: AssetDescriptor {{ marker: \"{}\", name: \"{}\", paths: [\"{}\"], params: {} bytes }}",
                padding,
                name,
                marker,
                dname,
                path_str,
                params.len()
            );
        }
        TdxValue::AssetHeader {
            marker,
            name: dname,
        } => {
            println!(
                "{}{}: AssetHeader {{ marker: \"{}\", name: \"{}\" }}",
                padding, name, marker, dname
            );
        }
        TdxValue::AssetPath(path) => {
            println!("{}{}: AssetPath(\"{}\")", padding, name, path);
        }
        TdxValue::Binary(data) => {
            println!("{}{}: Binary({} bytes)", padding, name, data.len());
        }
        TdxValue::TypedProperty { value, signature } => {
            println!(
                "{}{}: TypedProperty {{ value: 0x{:08x}, signature: 0x{:08x} }}",
                padding, name, value, signature
            );
        }
        TdxValue::Footer { strings } => {
            println!("{}{}: Footer {{ strings: {:?} }}", padding, name, strings);
        }
        TdxValue::Unknown(kind, data) => {
            if data.len() > 100 {
                println!(
                    "{}{}: Unknown(0x{:02x}, {} bytes)",
                    padding,
                    name,
                    kind,
                    data.len()
                );
                // Show first 32 bytes of hex
                let hex: Vec<String> = data.iter().take(32).map(|b| format!("{:02x}", b)).collect();
                println!("{}  Hex: {}...", padding, hex.join(" "));
            } else {
                let hex: Vec<String> = data.iter().map(|b| format!("{:02x}", b)).collect();
                println!(
                    "{}{}: Unknown(0x{:02x}, {} bytes) Hex: {}",
                    padding,
                    name,
                    kind,
                    data.len(),
                    hex.join(" ")
                );
            }
        }
        TdxValue::Array(arr) => {
            println!("{}{}: Array [", padding, name);
            for (i, v) in arr.iter().enumerate() {
                print_value(&format!("{}", i), v, indent + 1);
            }
            println!("{}]", padding);
        }
        TdxValue::Float32(f) => println!("{}{}: {} (float32)", padding, name, f),
    }
}
