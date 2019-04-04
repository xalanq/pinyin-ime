use jieba_rs::Jieba;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};

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
    let file = File::open(path).expect(&format!("......Cannot open {}", path));
    let buf = BufReader::with_capacity(1024 * 1024 * 32, file);
    let mut word_v = Vec::new();
    let mut word_m = HashMap::new();
    let mut pinyin = HashMap::new();
    word_v.push(Vec::new()); // start
    word_v.push(Vec::new()); // unknown
    buf.lines().for_each(|line| {
        let data: Vec<_> = line.as_ref().unwrap().split_whitespace().collect();
        if let Some(mut word) = hanzi2vec(data[0], hanzi_m) {
            word.shrink_to_fit();
            let id = word_m.entry(word.clone()).or_insert_with(|| {
                word_v.push(word.clone());
                word_v.len() - 1
            });
            data[1..]
                .iter()
                .for_each(|s| pinyin.entry(s.to_string()).or_insert(Vec::new()).push(*id));
        }
    });
    pinyin.iter_mut().for_each(|(_, v)| {
        v.shrink_to_fit();
    });
    word_v.shrink_to_fit();
    word_m.shrink_to_fit();
    pinyin.shrink_to_fit();
    println!("...Loaded!");
    (word_v, word_m, pinyin)
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
) -> (
    HashMap<usize, usize>,
    HashMap<(usize, usize), usize>,
    HashMap<(usize, usize, usize), usize>,
    // HashMap<(usize, usize, usize, usize), usize>,
) {
    println!("Loading gram");
    let mut gram_1 = HashMap::new();
    let mut gram_2 = HashMap::new();
    let mut gram_3 = HashMap::new();
    // let mut gram_4 = HashMap::new();
    macro_rules! word {
        ($s:expr) => {
            hanzi2word(&$s, hanzi_m, word_m).unwrap()
        };
    }
    macro_rules! load {
        ($i:expr, $c:expr) => {{
            let fname = &format!("{}/gram_{}.txt", path, $i);
            println!("...Working on {}", fname);
            let file = File::open(fname).expect(&format!("......Cannot open {}", fname));
            let buf = BufReader::with_capacity(1024 * 1024 * 32, file);
            buf.lines().for_each($c);
        }};
    }
    join!(
        || load!(1, |line| {
            let data: Vec<_> = line.as_ref().unwrap().split_whitespace().collect();
            let num = data[data.len() - 1].parse::<usize>().unwrap();
            gram_1.insert(word!(data[0]), num);
        }),
        || load!(2, |line| {
            let data: Vec<_> = line.as_ref().unwrap().split_whitespace().collect();
            let num = data[data.len() - 1].parse::<usize>().unwrap();
            gram_2.insert((word!(data[0]), word!(data[1])), num);
        }),
        || load!(3, |line| {
            let data: Vec<_> = line.as_ref().unwrap().split_whitespace().collect();
            let num = data[data.len() - 1].parse::<usize>().unwrap();
            gram_3.insert((word!(data[0]), word!(data[1]), word!(data[2])), num);
        }) /*,
           || load!(1, |line| {
               let data: Vec<_> = line.as_ref().unwrap().split_whitespace().collect();
               let num = data[data.len() - 1].parse::<usize>().unwrap();
               gram_4.insert((word!(data[0]), word!(data[1]), word!(data[2]), word!(data[3])), num);
           })*/
    );
    gram_1.shrink_to_fit();
    gram_2.shrink_to_fit();
    gram_3.shrink_to_fit();
    // gram_4.shrink_to_fit();
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

pub fn hanzi2vec(s: &str, hanzi_m: &HashMap<char, usize>) -> Option<Vec<usize>> {
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
    hanzi_m: &HashMap<char, usize>,
    word_m: &HashMap<Vec<usize>, usize>,
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
