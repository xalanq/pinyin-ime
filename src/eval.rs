use crate::load;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::sync::Mutex;
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

struct State {
    pub pos: usize,        // position of next word's beginning
    pub w1: usize,         // last and nearest word
    pub w2: Option<usize>, // ahead of w1
    // pub w3: Option<usize>, // ahead of w2
    pub prev: usize, // previous index in pool
}

#[derive(Copy, Clone, PartialEq, PartialOrd)]
struct HeapState {
    pub p: f64,     // prob
    pub idx: usize, // idx for memory pool
}

impl Eq for HeapState {}

impl Ord for HeapState {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.p.partial_cmp(&rhs.p).unwrap()
    }
}

#[derive(Copy, Clone)]
struct AnsState {
    pub p: f64,
    pub w: usize,
    pub prev: usize,
}

/*
struct Node {
    pub pos: usize,              // position of next word's beginning
    pub w1: usize,               // last and nearest word
    pub w2: Option<usize>,       // ahead of w1
    pub max_p: Option<f64>,      // max prob
    pub edge: Vec<(usize, f64)>, // Each element is an (index of node's pool, weight)
}
*/

impl PinyinIME {
    fn preprocess(&self, pinyin: &str) -> (usize, Vec<Vec<(usize, &Vec<usize>)>>) {
        let items: Vec<_> = pinyin.split_whitespace().collect();
        let mut pre = Vec::new();
        for i in 0..items.len() {
            let mut ava = Vec::new();
            let mut s = String::new();
            for j in i..items.len().min(self.max_len + i) {
                if j != i {
                    s.push('\'');
                }
                s.push_str(&items[j].to_lowercase());
                if let Some(v) = self.pinyin_m.get(&s) {
                    ava.push((j + 1, v));
                }
            }
            pre.push(ava);
        }
        (items.len(), pre)
    }

    /*
    pub fn dp(&self, pinyin: &str, max_ans: usize) -> Vec<(String, f64)> {
        let (len, pre) = self.preprocess(pinyin);
        let mut pool = vec![Node { pos: 0, w1: 0, w2: None, max_p: None, edge: vec![] }];
        let mut vis = HashMap::new();
        let mut idx = 0;
        let lambda_ln = self.lambda.ln();
        while idx < pool.len() {
            let Node { pos, w1, w2, max_p: _, edge: _ } = pool[idx];
            if pos == len {
                idx += 1;
                continue;
            }
            pre[pos].iter().for_each(|candi| {
                candi.1.iter().for_each(|&w| {
                    let y = (candi.0, w, Some(w1));
                    let y = *vis.entry(y).or_insert_with(|| {
                        pool.push(Node { pos: y.0, w1: y.1, w2: y.2, max_p: None, edge: vec![] });
                        pool.len() - 1
                    });
                    let mut ls = None;
                    let mut weight = match self.gram_2.get(&(w1, w)) {
                        Some(&t) => t,
                        None => {
                            ls = Some(lambda_ln + self.gram_1.get(&w).unwrap());
                            ls.unwrap()
                        }
                    };
                    if w2 != None {
                        weight = weight.max(match self.gram_3.get(&(w2.unwrap(), w1, w)) {
                            Some(&t) => t,
                            None => {
                                if ls.is_none() {
                                    ls = Some(lambda_ln + self.gram_1.get(&w).unwrap());
                                }
                                ls.unwrap()
                            }
                        });
                    }
                    pool[idx].edge.push((y, weight));
                });
            });
            idx += 1;
        }
        println!("nodes: {}, edges: {}", idx, pool.iter().fold(0, |sum, x| sum + x.edge.len()));
        // 跑了几个例子发现状态和边太tm多了，这跑一次k短路会爆炸，还是用慢慢拓展的dij吧
        // nodes: 49804, edges: 2433368
        vec![("hehe".to_owned(), 0.0)]
    }
    */

    pub fn kth_shortest_small(&self, pinyin: &str, max_ans: usize) -> Vec<(String, f64)> {
        let (len, pre) = self.preprocess(pinyin);
        let lambda_ln = self.lambda.ln();
        let mut heap = BinaryHeap::new();
        let mut pool = vec![];
        let mut ans: Vec<AnsState> = Vec::new(); // left -> right, max -> min , rust has no min-max-heap
        pool.push(State { pos: 0, w1: 0, w2: None, prev: 0 });
        // pool.push(SaveState { pos: 0, w1: 0, w2: None, w3: None, prev: 0 });
        heap.push(HeapState { p: 0.0, idx: 0 });
        // let mut cnt = 0;
        while let Some(HeapState { p, idx }) = heap.pop() {
            // cnt += 1;
            let State { pos, w1, w2, prev: _ } = pool[idx];
            // let State { pos, w1, w2, w3, prev: _ } = pool[idx];
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
                    /*
                    if w3 != None {
                        cp = cp.max(match self.gram_4.get(&(w3.unwrap(), w2.unwrap(), w1, w)) {
                            Some(&t) => t,
                            None => {
                                if ls.is_none() {
                                    ls = Some(lambda_ln + self.gram_1.get(&w).unwrap());
                                }
                                ls.unwrap()
                            }
                        });
                    }
                    */
                    // cut
                    /*if w == 66540 {
                        println!(
                            "pos: {}, w2: {}, w1: {}, p: {}, cp: {}",
                            pos,
                            load::word2hanzi(w2.unwrap(), &self.hanzi_v, &self.word_v),
                            load::word2hanzi(w1, &self.hanzi_v, &self.word_v),
                            p,
                            cp
                        );
                    }*/
                    if ans.len() < max_ans || cp + p > ans.last().unwrap().p {
                        let np = cp + p;
                        if candi.0 == len {
                            ans.push(AnsState { p: np, w, prev: idx });
                            let mut i = ans.len() - 1;
                            // idiot insert sort, i need a min-max heap
                            while i > 0 && ans[i - 1].p < ans[i].p {
                                ans.swap(i - 1, i);
                                i -= 1;
                            }
                            if ans.len() > max_ans {
                                ans.pop();
                            }
                        } else {
                            let y = State { pos: candi.0, w1: w, w2: Some(w1), prev: idx };
                            pool.push(y);
                            heap.push(HeapState { p: np, idx: pool.len() - 1 });
                        }
                    }
                });
            });
        }
        // println!("cnt: {}", cnt);
        let mut ans_s = Vec::new();
        ans.iter().for_each(|st| {
            let mut ws = vec![st.w];
            let mut x = st.prev;
            while x != 0 {
                ws.push(pool[x].w1);
                x = pool[x].prev;
            }
            let mut s = String::new();
            ws.iter()
                .rev()
                .for_each(|w| s.push_str(&load::word2hanzi(*w, &self.hanzi_v, &self.word_v)));
            ans_s.push((s, st.p.exp()));
        });
        ans_s
    }

    pub fn evals(&self, pinyin: &str, max_ans: usize) -> Vec<(String, f64)> {
        if pinyin.len() == 0 || max_ans == 0 {
            return Vec::new();
        }
        self.kth_shortest_small(pinyin, max_ans)
    }

    pub fn eval(&self, pinyin: &str) -> Option<(String, f64)> {
        if pinyin.len() == 0 {
            return None;
        }
        let (len, pre) = self.preprocess(pinyin);
        let lambda_ln = self.lambda.ln();
        let mut heap = BinaryHeap::new();
        let mut vis = HashSet::new();
        let mut pool = vec![];
        let mut ans: Option<AnsState> = None;
        pool.push(State { pos: 0, w1: 0, w2: None, prev: 0 });
        // pool.push(SaveState { pos: 0, w1: 0, w2: None, w3: None, prev: 0 });
        heap.push(HeapState { p: 0.0, idx: 0 });
        // let mut cnt = 0;
        while let Some(HeapState { p, idx }) = heap.pop() {
            // cnt += 1;
            let State { pos, w1, w2, prev: _ } = pool[idx];
            // let State { pos, w1, w2, w3, prev: _ } = pool[idx];
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
                    /*
                    if w3 != None {
                        cp = cp.max(match self.gram_4.get(&(w3.unwrap(), w2.unwrap(), w1, w)) {
                            Some(&t) => t,
                            None => {
                                if ls.is_none() {
                                    ls = Some(lambda_ln + self.gram_1.get(&w).unwrap());
                                }
                                ls.unwrap()
                            }
                        });
                    }
                    */
                    // cut
                    if ans.is_none() || cp + p > ans.unwrap().p {
                        let np = cp + p;
                        if candi.0 == len {
                            ans = Some(AnsState { p: np, w, prev: idx });
                        } else {
                            let y = (candi.0, w, Some(w1));
                            if !vis.contains(&y) {
                                vis.insert(y);
                                let y = State { pos: y.0, w1: w, w2: y.2, prev: idx };
                                pool.push(y);
                                heap.push(HeapState { p: np, idx: pool.len() - 1 });
                            }
                        }
                    }
                });
            });
        }
        // println!("cnt: {}", cnt);
        if ans.is_none() {
            return None;
        }
        let st = ans.unwrap();
        let mut ws = vec![st.w];
        let mut x = st.prev;
        while x != 0 {
            ws.push(pool[x].w1);
            x = pool[x].prev;
        }
        let mut s = String::new();
        ws.iter()
            .rev()
            .for_each(|w| s.push_str(&load::word2hanzi(*w, &self.hanzi_v, &self.word_v)));
        Some((s, st.p.exp()))
    }

    pub fn eval_from(&self, input_path: &str, answer_path: &str, output_path: &str) {
        println!("Predicting...");
        let s_time = time::Instant::now();
        let mut output = File::create(output_path).expect(&format!("Cannot open {}", output_path));
        let input = fs::read_to_string(input_path).expect(&format!("Cannot read {}", input_path));
        let ans = fs::read_to_string(answer_path).expect(&format!("Cannot read {}", answer_path));
        let input: Vec<_> = input.lines().collect();
        let ans: Vec<_> = ans.lines().collect();
        let preds = Mutex::new(vec![String::new(); input.len()]);
        let tot = Mutex::new(0);
        let (sum_acc, sum_f1) = (Mutex::new(0), Mutex::new(0.0));
        input.par_iter().zip(ans).enumerate().for_each(|(i, (input, ans))| {
            let pred = self.eval(input);
            let pred = if pred.is_none() { String::new() } else { pred.unwrap().0 };
            let (acc, f1) = score(ans, &pred);
            preds.lock().unwrap()[i] = pred;
            *sum_acc.lock().unwrap() += acc;
            *sum_f1.lock().unwrap() += f1;
            let mut r = tot.lock().unwrap();
            *r += 1;
            if *r % 500 == 0 {
                println!("...Finished {}", *r);
            }
        });
        output.write_all(preds.lock().unwrap().join("\n").as_bytes()).unwrap();
        let tot = *tot.lock().unwrap();
        println!("Total lines: {}", tot);
        println!("acc:   {:.2}", *sum_acc.lock().unwrap() as f64 / tot as f64);
        println!("f1:    {:.2}", *sum_f1.lock().unwrap() as f64 / tot as f64);
        let mils = (time::Instant::now() - s_time).as_millis();
        let mins = mils / 1000 / 60;
        let secs = mils / 1000 - mins * 60;
        println!("Total cost {}m {}s.", mins, secs);
        println!("Exit...");
    }

    pub fn eval_from_only(&self, input_path: &str, output_path: &str) {
        println!("Predicting...");
        let s_time = time::Instant::now();
        let mut output = File::create(output_path).expect(&format!("Cannot open {}", output_path));
        let input = fs::read_to_string(input_path).expect(&format!("Cannot read {}", input_path));
        let input: Vec<_> = input.lines().collect();
        let preds = Mutex::new(vec![String::new(); input.len()]);
        let tot = Mutex::new(0);
        input.par_iter().enumerate().for_each(|(i, input)| {
            let pred = self.eval(input);
            let pred = if pred.is_none() { String::new() } else { pred.unwrap().0 };
            preds.lock().unwrap()[i] = pred;
            let mut r = tot.lock().unwrap();
            *r += 1;
            if *r % 500 == 0 {
                println!("...Finished {}", *r);
            }
        });
        output.write_all(preds.lock().unwrap().join("\n").as_bytes()).unwrap();
        println!("Total lines: {}", tot.lock().unwrap());
        let mils = (time::Instant::now() - s_time).as_millis();
        let mins = mils / 1000 / 60;
        let secs = mils / 1000 - mins * 60;
        println!("Total cost {}m {}s.", mins, secs);
        println!("Exit...");
    }

    pub fn new(config_path: &str) -> Self {
        let lambda = load::load_config(config_path);
        let s_time = time::Instant::now();

        println!("Initializing Pinyin IME (lambda: {})", lambda);
        let max_len = 7;
        let (hanzi_v, hanzi_m) = load::load_hanzi("./data/hanzi.txt");
        let (word_v, word_m, pinyin_m) = load::load_word("./data/word.txt", &hanzi_m);
        let (gram_1, gram_2, gram_3) = load::load_gram("./data", &hanzi_m, &word_m, lambda);
        // let (gram_1, gram_2, gram_3, gram_4) = load::load_gram("./data", &hanzi_m, &word_m);

        let mils = (time::Instant::now() - s_time).as_millis();
        let mins = mils / 1000 / 60;
        let secs = mils / 1000 - mins * 60;
        println!("Done! Total cost {}m {}s.", mins, secs);

        Self { hanzi_v, word_v, pinyin_m, gram_1, gram_2, gram_3, max_len, lambda }
        // Self { hanzi_v, word_v, pinyin_m, gram_1, gram_2, gram_3, gram_4, max_len, lambda }
    }
}
