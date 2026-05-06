/// Connect to an iiod instance and dump the full device/channel/attribute tree.
///
/// Usage: cargo run --example scan_attrs -- [IP_ADDRESS]
///
/// Default IP: 192.168.2.1 (PlutoSDR)
use iiod_rs::Context;

fn main() {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "192.168.2.1".into());

    println!("Connecting to iiod at {addr}...");

    let ctx = match Context::connect(&addr) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Failed to connect: {e}");
            std::process::exit(1);
        }
    };

    println!(
        "Connected! Protocol version: {}.{} ({})\n",
        ctx.version.major, ctx.version.minor, ctx.version.git_tag
    );

    println!("Found {} device(s):\n", ctx.devices.len());

    for dev in &ctx.devices {
        let name = dev.name.as_deref().unwrap_or("(unnamed)");
        println!("  Device: {} [{}]", name, dev.id);

        if !dev.attrs.is_empty() {
            println!("    Device attributes:");
            for attr in &dev.attrs {
                println!("      - {} ({})", attr.name, attr.filename);
            }
        }

        for ch in &dev.channels {
            let dir = if ch.is_output { "output" } else { "input" };
            let ch_name = ch
                .name
                .as_deref()
                .map(|n| format!(" ({n})"))
                .unwrap_or_default();
            println!("    Channel: {}{} [{}]", ch.id, ch_name, dir);
            for attr in &ch.attrs {
                println!("      - {} ({})", attr.name, attr.filename);
            }
        }
        println!();
    }
}
