use anyhow::{Context, Result};
use clap::Parser;
use haversine_distance::json::{BufferedJsonLexer, JsonParser};

/// Generates haversine data points.
#[derive(Parser, Debug)]
#[command(version, about = "Parses a JSON file into an internal object structure.", long_about = None)]
struct Args {
    /// The file containing the json to parse
    #[arg(short, long)]
    file: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let lexer = BufferedJsonLexer::from_file(&args.file)?;
    let mut parser = JsonParser::new();

    for tokens in lexer {
        println!("{:?}", tokens);
        parser
            .consume_tokens(tokens)
            .with_context(|| "failed consuming tokens in JsonParser")?;
    }

    let value = parser.value()?;

    println!("Value: {:?}", value);

    Ok(())
}
