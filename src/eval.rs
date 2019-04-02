use crate::load;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::time;

pub fn score(answer: &str, predict: &str) -> (usize, f64) {
    let mut a: Vec<_> = answer.chars().collect();
    let mut b: Vec<_> = predict.chars().collect();
    a.sort();
    b.sort();
    let mut same = 0;
    let (mut x, mut y) = (0, 0);
    while x < a.len() && y < b.len() {
        if a[x] == b[y] {
            same += 1;
            x += 1;
            y += 1;
        } else if a[x] < b[y] {
            x += 1;
        } else {
            y += 1;
        }
    }
    if same == 0 {
        (0, 0.0)
    } else {
        let precision = 1.0 * same as f64 / b.len() as f64;
        let recall = 1.0 * same as f64 / a.len() as f64;
        (if answer == predict { 1 } else { 0 }, (2.0 * precision * recall) / (precision + recall))
    }
}

pub fn score_list(answer: &[String], predict: &[String]) -> (f64, f64) {
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
    gram_1: HashMap<usize, f64>,                 // w, ln p(w)
    gram_2: HashMap<(usize, usize), f64>,        // (w1, w), ln p(w|w1)
    gram_3: HashMap<(usize, usize, usize), f64>, // (w2, w1, w), ln p(w|w2w1)
    // gram_4: HashMap<(usize, usize, usize, usize), f64>,
    max_len: usize,
    lambda: f64,
}

#[derive(Copy, Clone, PartialEq, PartialOrd)]
struct HeapState {
    pub pos: usize,        // position of next word's beginning
    pub w1: usize,         // last and nearest word
    pub w2: Option<usize>, // ahead of w1
    // pub w3: Option<usize>, // ahead of w2
    pub p: f64,     // prob
    pub idx: usize, // idx for memory pool
}

impl Eq for HeapState {}

impl Ord for HeapState {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.p.partial_cmp(&rhs.p).unwrap()
    }
}

struct SaveState {
    pub w: usize,    // word
    pub prev: usize, // previous index in pool
}

struct AnsState {
    pub p: f64,
    pub w: usize,
    pub prev: usize,
}

impl PinyinIME {
    pub fn evals(&self, pinyin: &str, max_ans: usize) -> Vec<(String, f64)> {
        let items: Vec<_> = pinyin.split_whitespace().collect();
        if items.len() == 0 {
            return Vec::new();
        }
        let mut pre = Vec::new();
        for i in 0..items.len() {
            let mut ava = Vec::new();
            let mut s = String::new();
            for j in i..items.len().min(self.max_len + i) {
                if j != i {
                    s.push('\'');
                }
                s.push_str(items[j]);
                if let Some(v) = self.pinyin_m.get(&s) {
                    ava.push((j + 1, v));
                }
            }
            pre.push(ava);
        }
        let lambda_ln = self.lambda.ln();
        let mut heap = BinaryHeap::new();
        let mut pool = vec![];
        let mut ans: VecDeque<AnsState> = VecDeque::new(); // left -> right, min -> max
        pool.push(SaveState { w: 0, prev: 0 });
        heap.push(HeapState { pos: 0, w1: 0, w2: None, p: 0.0, idx: 0 });
        // heap.push(HeapState { pos: 0, w1: 0, w2: None, w3: None, p: 0.0, idx: 0 });
        while let Some(HeapState { pos, w1, w2, p, idx }) = heap.pop() {
            // while let Some(HeapState { pos, w1, w2, w3, p, idx }) = heap.pop() {
            // println!("here pos: {}, w1: {}, w2: {:?}, p: {}, idx: {}", pos, w1, w2, p, idx);
            pre[pos].iter().for_each(|candi| {
                candi.1.iter().for_each(|&w| {
                    let mut ls = None;
                    let mut cp = match self.gram_2.get(&(w1, w)) {
                        Some(&t) => t,
                        None => {
                            ls = Some(lambda_ln + self.gram_1.get(&w).unwrap());
                            ls.unwrap()
                        }
                    };
                    if w2 != None {
                        cp = cp.max(match self.gram_3.get(&(w2.unwrap(), w1, w)) {
                            Some(&t) => t,
                            None => {
                                if ls.is_none() {
                                    ls = Some(lambda_ln + self.gram_1.get(&w).unwrap());
                                }
                                ls.unwrap()
                            }
                        });
                    }
                    // cut
                    if ans.len() == 0 || cp + p > ans.back().unwrap().p {
                        let np = cp + p;
                        if candi.0 == items.len() {
                            if ans.len() == max_ans {
                                ans.pop_front();
                            }
                            ans.push_back(AnsState { p: np, w, prev: idx });
                        } else {
                            pool.push(SaveState { w, prev: idx });
                            heap.push(HeapState {
                                pos: candi.0,
                                w1: w,
                                w2: Some(w1),
                                // w3: w2,
                                p: np,
                                idx: pool.len() - 1,
                            });
                        }
                    }
                });
            });
        }
        let mut ans_s = Vec::new();
        ans.iter().rev().for_each(|st| {
            let mut s = String::new();
            let mut ws = vec![st.w];
            let mut x = st.prev;
            while x != 0 {
                ws.push(pool[x].w);
                x = pool[x].prev;
            }
            ws.iter()
                .rev()
                .for_each(|w| s.push_str(&load::word2hanzi(*w, &self.hanzi_v, &self.word_v)));
            ans_s.push((s, st.p.exp()));
        });
        ans_s
    }

    pub fn eval(&self, pinyin: &str) -> (String, f64) {
        self.evals(pinyin, 1)[0].clone()
    }
}

impl Default for PinyinIME {
    fn default() -> Self {
        let s_time = time::Instant::now();

        println!("Initialize Pinyin IME (default)");
        let max_len = 7;
        let lambda = 0.5;
        let (hanzi_v, hanzi_m) = load::load_hanzi("./data/hanzi.txt");
        let (word_v, word_m, pinyin_m) = load::load_word("./data/word.txt", &hanzi_m);
        let (gram_1, gram_2, gram_3) = load::load_gram("./data", &hanzi_m, &word_m, lambda);
        // let (gram_1, gram_2, gram_3, gram_4) = load::load_gram("./data", &hanzi_m, &word_m);

        let mils = (time::Instant::now() - s_time).as_millis();
        let days = mils / 1000 / 60 / 60 / 24;
        let hours = mils / 1000 / 60 / 60 - days * 24;
        let mins = mils / 1000 / 60 - days * 24 * 60 - hours * 60;
        let secs = mils / 1000 - days * 24 * 60 * 60 - hours * 60 * 60 - mins * 60;
        println!("Done! Total cost {}d {}h {}m {}s.", days, hours, mins, secs);

        Self { hanzi_v, word_v, pinyin_m, gram_1, gram_2, gram_3, max_len, lambda }
        // Self { hanzi_v, word_v, pinyin_m, gram_1, gram_2, gram_3, gram_4, max_len, lambda }
    }
}
