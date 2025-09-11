fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = std::env::args().nth(1).expect("missing source");
    println!("Parsing data from {} ...", source);

    let file = std::fs::File::open(&source)?;
    let mut file = std::io::BufReader::new(file);

    let iter = dasl::drisl::de::iter_from_reader::<dasl::drisl::Value, _>(&mut file);
    let now = std::time::Instant::now();
    let mut count = 0;
    for (i, el) in iter.enumerate() {
        let el = el?;
        if i == 0 {
            println!("{:?}", el);
        }
        if i % 100 == 0 {
            print!(".");
        }
        count += 1;
    }
    println!("\n");

    let done = now.elapsed();

    let meta = std::fs::metadata(&source)?;
    let mbs = meta.len() as f64 / done.as_secs_f64() / 1024. / 1024.;
    let values_per_sec = count as f64 / done.as_secs_f64();

    println!(
        "File '{}' ({:.01}MiB)\nParsed {} values in {}ms\n{:.02} Values/s\n{:.02} MiB/s",
        source,
        meta.len() as f64 / 1024. / 1024.,
        count,
        done.as_millis(),
        values_per_sec,
        mbs,
    );
    Ok(())
}
