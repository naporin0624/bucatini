use anyhow::{anyhow, Result};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use bucatini::cli::{self, SourceMatch};
use bucatini::ndi::{Finder, Ndi, Receiver, Source};
use bucatini::output::{self, output_kind, SharedTextureOutput};

fn main() -> Result<()> {
    let args = cli::parse();
    let ndi = Ndi::new()?;
    let finder = Finder::new(&ndi)?;
    let sources = finder.list(args.timeout_ms);

    if args.list {
        print_sources(&sources);
        return Ok(());
    }
    if sources.is_empty() {
        return Err(anyhow!(
            "no NDI sources found within {} ms (is a source online?)",
            args.timeout_ms
        ));
    }

    let source = select_source(&sources, &args.source)?;
    let server_name = args.name.clone().unwrap_or_else(|| source.name.clone());

    let receiver = Receiver::new(&ndi, &source, "Bucatini")?;
    let mut out = output::make_output(&server_name)?;

    println!(
        "Publishing '{}' as {} server '{}'. Ctrl-C to stop.",
        source.name,
        output_kind(),
        server_name
    );
    run_loop(&receiver, &mut *out, args.verbose)
}

fn print_sources(sources: &[Source]) {
    if sources.is_empty() {
        println!("(no NDI sources found)");
        return;
    }
    for (i, s) in sources.iter().enumerate() {
        println!("{}: {} ({})", i + 1, s.name, s.url);
    }
}

fn select_source(sources: &[Source], query: &Option<String>) -> Result<Source> {
    let names: Vec<String> = sources.iter().map(|s| s.name.clone()).collect();
    match query {
        Some(q) => match cli::match_source(&names, q) {
            SourceMatch::One(i) => Ok(sources[i].clone()),
            SourceMatch::None => Err(anyhow!(
                "no source matches '{}'. Available:\n{}",
                q,
                list_str(sources)
            )),
            SourceMatch::Many(v) => Err(anyhow!(
                "'{}' is ambiguous ({} matches). Be more specific:\n{}",
                q,
                v.len(),
                list_str(sources)
            )),
        },
        None => prompt_select(sources),
    }
}

fn list_str(sources: &[Source]) -> String {
    sources
        .iter()
        .enumerate()
        .map(|(i, s)| format!("  {}: {}", i + 1, s.name))
        .collect::<Vec<_>>()
        .join("\n")
}

fn prompt_select(sources: &[Source]) -> Result<Source> {
    print_sources(sources);
    print!("Select source [1-{}]: ", sources.len());
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    match cli::parse_selection(&line, sources.len()) {
        Ok(i) => Ok(sources[i].clone()),
        Err(e) => Err(anyhow!("invalid selection: {e}")),
    }
}

fn run_loop(receiver: &Receiver, out: &mut dyn SharedTextureOutput, verbose: bool) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    {
        let r = running.clone();
        ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))?;
    }
    let frames = AtomicU64::new(0);
    bucatini::run::run_capture_loop(receiver, out, &running, &frames, verbose)?;
    println!("\nStopped.");
    Ok(())
}
