mod cli;
mod ndi;
mod output;

use anyhow::{anyhow, Result};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use cli::SourceMatch;
use ndi::{CaptureResult, Finder, Ndi, Receiver, Source};
use output::{BgraFrame, SharedTextureOutput};

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

    let receiver = Receiver::new(&ndi, &source, "ndi-share")?;
    let mut out = make_output(&server_name)?;

    println!("Publishing '{}' as Syphon server '{}'. Ctrl-C to stop.", source.name, server_name);
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
            SourceMatch::None => Err(anyhow!("no source matches '{}'. Available:\n{}", q, list_str(sources))),
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

#[cfg(target_os = "macos")]
fn make_output(name: &str) -> Result<Box<dyn SharedTextureOutput>> {
    Ok(Box::new(output::syphon::SyphonOutput::new(name)?))
}

#[cfg(not(target_os = "macos"))]
fn make_output(_name: &str) -> Result<Box<dyn SharedTextureOutput>> {
    Err(anyhow!("only macOS/Syphon output is implemented in v1"))
}

fn run_loop(receiver: &Receiver, out: &mut dyn SharedTextureOutput, verbose: bool) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    {
        let r = running.clone();
        ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))?;
    }

    let mut last_dims = (0u32, 0u32);
    while running.load(Ordering::SeqCst) {
        match receiver.capture(1000) {
            CaptureResult::Video(frame) => {
                let needed = (frame.stride() as usize).saturating_mul(frame.height() as usize);
                if frame.data().len() < needed {
                    if verbose {
                        eprintln!("skipping malformed frame: data {} < needed {}", frame.data().len(), needed);
                    }
                    continue;
                }
                let dims = (frame.width(), frame.height());
                if verbose && dims != last_dims {
                    eprintln!("frame {}x{} stride={}", dims.0, dims.1, frame.stride());
                    last_dims = dims;
                }
                let bgra = BgraFrame {
                    data: frame.data(),
                    width: frame.width(),
                    height: frame.height(),
                    stride: frame.stride(),
                };
                if let Err(e) = out.publish(&bgra) {
                    eprintln!("publish error: {e}");
                }
            }
            CaptureResult::Error => eprintln!("NDI capture error"),
            CaptureResult::None => {} // timeout / non-video frame; keep polling
        }
    }
    println!("\nStopped.");
    Ok(())
}
