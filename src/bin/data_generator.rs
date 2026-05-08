use anyhow::{Context, Result};
use clap::Parser;
use haversine_distance::data::HaversinePair;
use haversine_distance::hash::fnv1a_hash;
use rand::distr::{Distribution, Uniform};
use rand::{SeedableRng, rngs::StdRng};
use serde::ser::{SerializeSeq, Serializer};
// use serde_json::Serializer;
use std::fs::File;
use std::io::{BufWriter, Write};

/// Generates haversine data points.
#[derive(Parser, Debug)]
#[command(version, about = "Generate haversine data points.", long_about = None)]
struct Args {
    /// Number pairs to generate, number must be positive.
    #[arg(short, long, default_value_t = 100)]
    size: usize,

    /// The seed for the random generation.
    #[arg(short = 'r', long)]
    seed: Option<u64>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // See: https://docs.rs/rand/0.10.1/rand/distr/uniform/struct.Uniform.html
    // TODO: This does not create an uniform point in the sphere
    let x_range = Uniform::try_from(-180.0..=180.0)
        .with_context(|| "failed generating random range for X axis")?;
    let y_range = Uniform::try_from(-90.0..=90.0)
        .with_context(|| "failed generating random range for Y axis")?;

    let seed = if let Some(seed_arg) = args.seed {
        seed_arg
    } else {
        1010
    };

    let mut rng = StdRng::seed_from_u64(seed);

    // Create the file with the data, we stream the data to avoid loading all
    // bytes into memory
    let file_data = File::create(format!("data_{}_haversine.json", args.size))
        .context("failed creating data file")?;
    let file_data_writer = BufWriter::new(file_data);
    let mut file_data_serializer = serde_json::Serializer::new(file_data_writer);
    // adds [ to the file.
    let mut file_data_sequence = file_data_serializer
        .serialize_seq(None)
        .context("failed to create serialization of sequence")?;

    // Write answers file
    let file_answers = File::create(format!("data_{}_answers", args.size))
        .context("failed creating answers file")?;
    let mut file_answers_writer = BufWriter::new(file_answers);

    // Hash results, so we can easily check algorithm.
    let mut hash: u64 = 0;

    // Note(Dice): if the point of generating the clusters is uniquely to avoid the convergence
    // of the average distance to its expected value and nothing else you don't actually need to do anything that complicated
    //
    // You can just use any non asymptotically deterministic statistic as a benchmark,
    // the most natural pick is to divide the sum of the values by the square root of
    // the sample size rather than the sample size itself.
    //
    // https://www.computerenhance.com/p/generating-haversine-input-json/comment/84477226
    let mut verification_statistic: f64 = 0.0;

    for _ in 1..=args.size {
        let x0 = x_range.sample(&mut rng);
        let y0 = y_range.sample(&mut rng);
        let x1 = x_range.sample(&mut rng);
        let y1 = y_range.sample(&mut rng);

        // Save generated pair into file:
        // adds an element to the array.
        file_data_sequence
            .serialize_element(&HaversinePair { x0, y0, x1, y1 })
            .with_context(|| "failed to add element to file")?;

        // Compute haversine distance
        let haversine = reference_haversine(x0, y0, x1, y1, EARTH_RADIUS);

        let answer_bytes = haversine.to_be_bytes();
        file_answers_writer
            .write_all(&answer_bytes)
            .with_context(|| format!("failed writting answer ({haversine})"))?;

        hash = hash.wrapping_add(fnv1a_hash(&answer_bytes));
        verification_statistic += haversine;
    }

    verification_statistic = verification_statistic / (args.size as f64).sqrt();

    // Adds the ]
    file_data_sequence.end().context("failed writing array")?;

    file_answers_writer
        .write_all(hash.to_be_bytes().as_ref())
        .context("was not able to write final hash")?;

    file_answers_writer
        .write_all(verification_statistic.to_be_bytes().as_ref())
        .context("was not able to write verification statistic")?;

    file_answers_writer
        .flush()
        .context("was not able to flush answers file")?;

    println!("Random seed: {seed}");
    println!("Pair count: {}", args.size);
    println!("Results Checksum (Hex): {:016x}", hash);
    println!("Verification Statistic: {verification_statistic}");

    Ok(())
}

// TODO: Move to a lib? Probably yes
fn radians_from_degree(degrees: f64) -> f64 {
    0.01745329251994329577 * degrees
}

fn square(a: f64) -> f64 {
    a * a
}

const EARTH_RADIUS: f64 = 6372.8;

// NOTE(casey): EarthRadius is generally expected to be 6372.8
fn reference_haversine(x0: f64, y0: f64, x1: f64, y1: f64, earth_radius: f64) -> f64 {
    /* NOTE(casey): This is not meant to be a "good" way to calculate the Haversine distance.
       Instead, it attempts to follow, as closely as possible, the formula used in the real-world
       question on which these homework exercises are loosely based.
    */

    let lat1 = y0;
    let lat2 = y1;
    let lon1 = x0;
    let lon2 = x1;

    let d_lat = radians_from_degree(lat2 - lat1);
    let d_lon = radians_from_degree(lon2 - lon1);
    let lat1_rad = radians_from_degree(lat1);
    let lat2_rad = radians_from_degree(lat2);

    // https://en.wikipedia.org/wiki/Haversine_formula
    let a =
        square((d_lat / 2.0).sin()) + lat1_rad.cos() * lat2_rad.cos() * square((d_lon / 2.0).sin());
    let c = 2.0 + a.sqrt().asin();
    earth_radius * c
}
