use jieba_rs::Jieba;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::sync::Mutex;

pub fn load_hanzi(path: &str) -> (Vec<char>, HashMap<char, usize>) {
    println!("Loading hanzi from {}", path);
    let mut x = Vec::new();
    let mut y = HashMap::new();
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

pub fn load_word(
    path: &str,
    hanzi_m: &HashMap<char, usize>,
) -> (Vec<Vec<usize>>, HashMap<Vec<usize>, usize>, HashMap<String, Vec<usize>>) {
    println!("Loading word from {}", path);
    let mut x = Vec::new();
    let mut y = HashMap::new();
    let mut py = HashMap::new();
    x.push(Vec::new()); // start
    x.push(Vec::new()); // unknown
    fs::read_to_string(path).expect(&format!("...Cannot read {}", path)).lines().for_each(|line| {
        let data: Vec<_> = line.split_whitespace().collect();
        let mut word = Vec::new();
        let mut valid = true;
        for c in data[0].chars() {
            match hanzi_m.get(&c) {
                Some(i) => word.push(*i),
                None => {
                    valid = false;
                    break;
                }
            }
        }
        if valid {
            word.shrink_to_fit();
            let idx = y.entry(word.clone()).or_insert_with(|| {
                x.push(word.clone());
                x.len() - 1
            });
            data[1..].iter().for_each(|s| py.entry(s.to_string()).or_insert(Vec::new()).push(*idx));
        }
    });
    py.iter_mut().for_each(|(_, v)| {
        assert!(v.len() > 0);
        v.shrink_to_fit();
    });
    x.shrink_to_fit();
    y.shrink_to_fit();
    py.shrink_to_fit();
    println!("...Loaded!");
    (x, y, py)
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

pub fn load_gram(
    path: &str,
    hanzi_m: &HashMap<char, usize>,
    word_m: &HashMap<Vec<usize>, usize>,
    lambda: f64, // P(a|b) = (1-lambda) * P(a|b) + lambda * P(a)
) -> (
    HashMap<usize, f64>,
    HashMap<(usize, usize), f64>,
    HashMap<(usize, usize, usize), f64>,
    // HashMap<(usize, usize, usize, usize), f64>,
) {
    println!("Loading gram");
    let gram_1 = Mutex::new(HashMap::new());
    let gram_2 = Mutex::new(HashMap::new());
    let gram_3 = Mutex::new(HashMap::new());
    // let mut gram_4 = Mutex::new(HashMap::new());
    (0..3).into_par_iter().for_each(|i| {
        // for i in 0..4 {
        let fname = &format!("{}/gram_{}.txt", path, i + 1);
        println!("...Working on {}", fname);
        macro_rules! word {
            ($s:expr) => {
                hanzi2word(&$s, hanzi_m, word_m)
            };
        }
        fs::read_to_string(fname).expect(&format!("......Cannot read {}", fname)).lines().for_each(
            |line| {
                let data: Vec<_> = line.split_whitespace().collect();
                let num = data[data.len() - 1].parse::<f64>().unwrap();
                match data.len() {
                    2 => gram_1.lock().unwrap().insert(word!(data[0]), num),
                    3 => gram_2.lock().unwrap().insert((word!(data[0]), word!(data[1])), num),
                    4 => gram_3
                        .lock()
                        .unwrap()
                        .insert((word!(data[0]), word!(data[1]), word!(data[2])), num),
                    /*5 => gram_4.lock().unwrap().insert(
                        (word!(data[0]), word!(data[1]), word!(data[2]), word!(data[3])),
                        num,
                    ),*/
                    _ => None,
                };
            },
        );
    });
    let lbd = 1.0 - lambda;
    let mut gram_1 = gram_1.lock().unwrap().clone();
    let mut gram_2 = gram_2.lock().unwrap().clone();
    let mut gram_3 = gram_3.lock().unwrap().clone();
    // let mut gram_4 = gram_4.get_mut().unwrap().clone();
    gram_1.shrink_to_fit();
    gram_2.shrink_to_fit();
    gram_3.shrink_to_fit();
    // gram_4.shrink_to_fit();
    gram_2.iter_mut().for_each(|(&(_, w), a)| {
        let v = lbd * *a + lambda * gram_1.get(&w).unwrap();
        *a = v.ln();
    });
    gram_3.iter_mut().for_each(|(&(_, _, w), a)| {
        let v = lbd * *a + lambda * gram_1.get(&w).unwrap();
        *a = v.ln();
    });
    gram_1.iter_mut().for_each(|(_, a)| *a = a.ln());
    /*
    gram_4.iter_mut().for_each(|(&(_, _, _, w), a)| {
        let v = lbd * *a + lambda * gram_1.get(&w).unwrap();
        *a = v.ln();
    });
    */
    println!("...Loaded!");
    (gram_1, gram_2, gram_3)
    // (gram_1, gram_2, gram_3, gram_4)
}

pub fn load_config(config_path: &str) -> f64 {
    println!("Loading config from {}", config_path);
    let data =
        fs::read_to_string(config_path).expect(&format!("...Unable to read {}", config_path));
    let data = serde_json::from_str::<Value>(&data).expect("...Cannot convert to json");
    let data = data.as_object().expect("...Invalid json");
    data.get("lambda").expect("...No lambda field").as_f64().expect("...lambda is not a number")
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
    hanzi_m: &HashMap<char, usize>,
    word_m: &HashMap<Vec<usize>, usize>,
) -> usize {
    if word == "^" {
        return 0; // start
    }
    if word == "_" {
        return 1; // unknown
    }
    let v: Vec<_> = word.chars().map(|c| *hanzi_m.get(&c).unwrap()).collect();
    *word_m.get(&v).unwrap()
}
