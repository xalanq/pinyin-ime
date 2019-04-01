use crate::load;
use std::collections::HashMap;
use std::time;

pub fn score(answer: &str, predict: &str) -> (usize, f64) {
    let mut m = HashMap::new();
    answer.chars().for_each(|c| *m.entry(c).or_insert(0) += 1);
    predict.chars().for_each(|c| *m.entry(c).or_insert(0) -= 1);
    let same = m.iter().filter(|x| *x.1 == 0).count();
    if same == 0 {
        (0, 0.0)
    } else {
        let precision = 1.0 * same as f64 / predict.chars().count() as f64;
        let recall = 1.0 * same as f64 / answer.chars().count() as f64;
        (if answer == predict { 1 } else { 0 }, (2.0 * precision * recall) / (precision + recall))
    }
}

pub fn score_list(answer: &[&str], predict: &[&str]) -> (f64, f64) {
    assert!(answer.len() == predict.len());
    let (mut a, mut b) = (0.0, 0.0);
    answer.iter().zip(predict).for_each(|(x, y)| {
        let (ta, tb) = score(x, y);
        a += ta as f64;
        b += tb as f64;
    });
    (a / answer.len() as f64, b / answer.len() as f64)
}

pub struct PinyinIME {
    hanzi_v: Vec<char>,
    // hanzi_m: HashMap<char, usize>,
    word_v: Vec<Vec<usize>>,
    // word_m: HashMap<Vec<usize>, usize>,
    pinyin_m: HashMap<String, Vec<usize>>,
    gram_1: HashMap<usize, f64>,
    gram_2: HashMap<(usize, usize), f64>,
    gram_3: HashMap<(usize, usize, usize), f64>,
    gram_4: HashMap<(usize, usize, usize, usize), f64>,
}

impl Default for PinyinIME {
    fn default() -> Self {
        let s_time = time::Instant::now();

        println!("Initialize Pinyin IME (default)");
        let (hanzi_v, hanzi_m) = load::load_hanzi("./data/hanzi.txt");
        let (word_v, word_m, pinyin_m) = load::load_word("./data/word.txt", &hanzi_m);
        let (gram_1, gram_2, gram_3, gram_4) = load::load_gram("./data", &hanzi_m, &word_m);

        let mils = (time::Instant::now() - s_time).as_millis();
        let days = mils / 1000 / 60 / 60 / 24;
        let hours = mils / 1000 / 60 / 60 - days * 24;
        let mins = mils / 1000 / 60 - days * 24 * 60 - hours * 60;
        let secs = mils / 1000 - days * 24 * 60 * 60 - hours * 60 * 60 - mins * 60;
        println!("Done! Total cost {}d {}h {}m {}s.", days, hours, mins, secs);

        Self { hanzi_v, word_v, pinyin_m, gram_1, gram_2, gram_3, gram_4 }
    }
}
