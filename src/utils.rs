use rand::seq::SliceRandom;

pub fn get_random_line(word_bank: &[String], max_width: usize) -> Vec<&str> {
    let mut rng = rand::thread_rng();

    let mut line = Vec::new();
    let mut line_size = 0;

    while line_size < max_width {
        let mut word = word_bank.choose(&mut rng).unwrap();
        while Some(&word.as_str()) == line.last() {
            word = word_bank.choose(&mut rng).unwrap();
        }
        line.push(word.as_str());
        line_size += word.len() + 1;
    }

    line
}
