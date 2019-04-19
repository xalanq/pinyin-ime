use crate::HB;
use jieba_rs::Jieba;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};

pub fn load_hanzi(path: &str) -> (Vec<char>, HashMap<char, usize, HB>) {
    println!("Loading hanzi from {}", path);
    let mut x = Vec::new();
    let mut y = HashMap::with_hasher(HB::default());
    fs::read_to_string(path).expect(&format!("...Cannot read {}", path)).chars().for_each(|c| {
        y.entry(c).or_insert_with(|| {
            x.push(c);
            x.len() - 1
        });
    });
    x.shrink_to_fit();
    y.shrink_to_fit();
    println!("...Loaded!");
    (x, y)
}

pub fn load_eval_word(
    path: &str,
    hanzi_m: &HashMap<char, usize, HB>,
) -> (Vec<Vec<usize>>, HashMap<String, Vec<usize>, HB>) {
    println!("Loading word from {}", path);
    let file = File::open(path).expect(&format!("......Cannot open {}", path));
    let buf = BufReader::with_capacity(1024 * 1024 * 32, file);
    let mut word_v = Vec::new();
    let mut pinyin_m = HashMap::with_hasher(HB::default());
    word_v.push(Vec::new()); // start
    word_v.push(Vec::new()); // unknown
    buf.lines().for_each(|line| {
        let data: Vec<_> = line.as_ref().unwrap().split_whitespace().collect();
        if let Some(mut word) = hanzi2vec(data[0], hanzi_m) {
            word.shrink_to_fit();
            word_v.push(word.clone());
            let id = word_v.len() - 1;
            data[1..]
                .iter()
                .for_each(|s| pinyin_m.entry(s.to_string()).or_insert(Vec::new()).push(id));
        }
    });
    pinyin_m.iter_mut().for_each(|(_, v)| {
        v.shrink_to_fit();
    });
    word_v.shrink_to_fit();
    pinyin_m.shrink_to_fit();
    println!("...Loaded!");
    (word_v, pinyin_m)
}

pub fn load_word(
    path: &str,
    hanzi_m: &HashMap<char, usize, HB>,
) -> (
    Vec<Vec<usize>>,
    HashMap<Vec<usize>, usize, HB>,
    Vec<Vec<String>>,
    HashMap<String, Vec<usize>, HB>,
) {
    println!("Loading word from {}", path);
    let file = File::open(path).expect(&format!("......Cannot open {}", path));
    let buf = BufReader::with_capacity(1024 * 1024 * 32, file);
    let mut word_v = Vec::new();
    let mut word_m = HashMap::with_hasher(HB::default());
    let mut pinyin_v = Vec::new();
    let mut pinyin_m = HashMap::with_hasher(HB::default());
    word_v.push(Vec::new()); // start
    word_v.push(Vec::new()); // unknown
    pinyin_v.push(Vec::new());
    pinyin_v.push(Vec::new());
    buf.lines().for_each(|line| {
        let data: Vec<_> = line.as_ref().unwrap().split_whitespace().collect();
        if let Some(mut word) = hanzi2vec(data[0], hanzi_m) {
            word.shrink_to_fit();
            let id = word_m.entry(word.clone()).or_insert_with(|| {
                word_v.push(word.clone());
                pinyin_v.push(data[1..].iter().map(|s| s.to_string()).collect::<Vec<_>>());
                word_v.len() - 1
            });
            data[1..]
                .iter()
                .for_each(|s| pinyin_m.entry(s.to_string()).or_insert(Vec::new()).push(*id));
        }
    });
    pinyin_m.iter_mut().for_each(|(_, v)| {
        v.shrink_to_fit();
    });
    word_v.shrink_to_fit();
    word_m.shrink_to_fit();
    pinyin_v.shrink_to_fit();
    pinyin_m.shrink_to_fit();
    println!("...Loaded!");
    (word_v, word_m, pinyin_v, pinyin_m)
}

pub fn load_jieba(path: &str) -> Jieba {
    println!("Loading jieba");
    let ret = Jieba::with_dict(&mut BufReader::new(
        File::open(path).expect(&format!("...Cannot open {}", path)),
    ))
    .expect("...Cannot build jieba");
    println!("...Loaded!");
    ret
}

pub fn load_gram(path: &str, idx: usize) -> Vec<Vec<usize>> {
    println!("Loading gram {}", idx);
    let file = File::open(path).expect(&format!("......Cannot open {}", path));
    let buf = BufReader::with_capacity(1024 * 1024 * 32, file);
    let mut gram = Vec::new();
    buf.lines().for_each(|line| {
        let data: Vec<_> = line
            .as_ref()
            .unwrap()
            .split_whitespace()
            .map(|s| s.parse::<usize>().unwrap())
            .collect();
        gram.push(data);
    });
    gram.shrink_to_fit();
    gram
}

pub fn load_gram_1(path: &str) -> HashMap<usize, f64, HB> {
    let v = load_gram(path, 1);
    let mut gram = HashMap::with_capacity_and_hasher(v.len(), HB::default());
    v.iter().for_each(|k| {
        gram.insert(k[0], k[1] as f64);
    });
    println!("...Loaded!");
    gram
}

pub fn load_gram_2(path: &str) -> HashMap<(usize, usize), f64, HB> {
    let v = load_gram(path, 2);
    let mut gram = HashMap::with_capacity_and_hasher(v.len(), HB::default());
    v.iter().for_each(|k| {
        gram.insert((k[0], k[1]), k[2] as f64);
    });
    println!("...Loaded!");
    gram
}

pub fn load_gram_3(path: &str) -> HashMap<(usize, usize, usize), f64, HB> {
    let v = load_gram(path, 3);
    let mut gram = HashMap::with_capacity_and_hasher(v.len(), HB::default());
    v.iter().for_each(|k| {
        gram.insert((k[0], k[1], k[2]), k[3] as f64);
    });
    println!("...Loaded!");
    gram
}

pub fn load_gram_4(path: &str) -> HashMap<(usize, usize, usize, usize), f64, HB> {
    let v = load_gram(path, 4);
    let mut gram = HashMap::with_capacity_and_hasher(v.len(), HB::default());
    v.iter().for_each(|k| {
        gram.insert((k[0], k[1], k[2], k[3]), k[4] as f64);
    });
    println!("...Loaded!");
    gram
}

pub fn load_config(config_path: &str) -> (f64, usize) {
    println!("Loading config from {}", config_path);
    let data =
        fs::read_to_string(config_path).expect(&format!("...Unable to read {}", config_path));
    let data = serde_json::from_str::<Value>(&data).expect("...Cannot convert to json");
    let data = data.as_object().expect("...Invalid json");
    let lambda = data
        .get("lambda")
        .expect("...No lambda field")
        .as_f64()
        .expect("...lambda is not a number");
    let max_len = data
        .get("max_len")
        .expect("...No max_len field")
        .as_u64()
        .expect("...max_len is not a number");
    (lambda, max_len as usize)
}

pub fn hanzi2vec(s: &str, hanzi_m: &HashMap<char, usize, HB>) -> Option<Vec<usize>> {
    let mut v = Vec::new();
    for c in s.chars() {
        match hanzi_m.get(&c) {
            Some(&w) => v.push(w),
            None => return None,
        }
    }
    Some(v)
}

pub fn word2hanzi(word: usize, hanzi_v: &Vec<char>, word_v: &Vec<Vec<usize>>) -> String {
    if word == 0 {
        return "^".to_owned(); // start
    } else if word == 1 {
        return "_".to_owned(); // unknown
    }
    let mut ret = String::new();
    word_v[word].iter().for_each(|i| {
        ret.push(hanzi_v[*i]);
    });
    ret
}

pub fn hanzi2word(
    word: &str,
    hanzi_m: &HashMap<char, usize, HB>,
    word_m: &HashMap<Vec<usize>, usize, HB>,
) -> Option<usize> {
    if word == "^" {
        return Some(0); // start
    }
    if word == "_" {
        return Some(1); // unknown
    }
    if let Some(v) = hanzi2vec(word, &hanzi_m) {
        if let Some(w) = word_m.get(&v) {
            return Some(*w);
        }
    }
    None
}
