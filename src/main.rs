use birdoftheday::*;

fn main() {
    // TODO: Allow command line arguments to periodically update local copy of birds.json
    // For now, just set to not run unless desired
    if false {
        get_all_birds();
    }
    // Do 3 attempts because it sometimes fails
    let mut num_attempts = 0;
    while num_attempts < 3 {
        num_attempts += 1;
        if run() {
            break;
        }
    }
    if num_attempts == 3 {
        eprintln!("After 3 attempts, unable to create post");
    }
}
