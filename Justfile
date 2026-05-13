generate_data:
    cargo run --bin data_generator -- --size 1000000 --seed 53643345

parse_json:
    cargo run --bin parse_json -- --file data_2_haversine.json

test:
    cargo test
