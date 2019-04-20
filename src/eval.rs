use crate::{load, max_lines::*, HB};
use rayon::current_num_threads;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
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
    word_v: Vec<Vec<usize>>,
    pinyin_m: HashMap<String, Vec<usize>, HB>,
    gram_1: HashMap<usize, f64, HB>, // w
    // gram: HashMap<(usize, usize), f64, HB>, // (w1, w)
    // gram: HashMap<(usize, usize, usize), f64, HB>, // (w2, w1, w)
    // gram: HashMap<(usize, usize, usize, usize), f64, HB>, // (w3, w2, w1, w)
    gram_2: HashMap<(usize, usize), f64, HB>, // (w1, w)
    gram_3: HashMap<(usize, usize, usize), f64, HB>, // (w2, w1, w)
    max_len: usize,
}

struct Node {
    pub pos: usize, // position of next word's beginning
    pub w1: usize,  // last and nearest word
    pub w2: usize,  // ahead of w1
    // pub w3: usize,   // ahead of w2
    pub prev: usize, // previous index in pool
    pub vis: bool,
    pub p: f64,
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
    fn preprocess(&self, pinyin: &str) -> (usize, Vec<Vec<(usize, &Vec<usize>, Vec<f64>)>>) {
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
                    let g: Vec<_> = v.iter().map(|w| *self.gram_1.get(w).unwrap()).collect();
                    ava.push((j + 1, v, g));
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

    /*
    pub fn kth_shortest_small(&self, pinyin: &str, max_ans: usize) -> Vec<(String, f64)> {
        let (len, pre) = self.preprocess(pinyin);
        let mut heap = BinaryHeap::new();
        let mut pool = vec![];
        let mut ans: Vec<AnsState> = Vec::new(); // left -> right, max -> min , rust has no min-max-heap
        pool.push(State { pos: 0, w1: 0, w2: 0, prev: 0 });
        // pool.push(SaveState { pos: 0, w1: 0, w2: None, w3: None, prev: 0 });
        heap.push(HeapState { p: 0.0, idx: 0 });
        // let mut cnt = 0;
        while let Some(HeapState { p, idx }) = heap.pop() {
            // cnt += 1;
            let State { pos, w1, w2, prev: _ } = pool[idx];
            // let State { pos, w1, w2, w3, prev: _ } = pool[idx];

            let fm_2 = if let Some(&fm) = self.gram_1.get(&w1) {
                Some(self.sum_1 / self.sum_2 / fm as f64)
            } else {
                None
            };
            let fm_3 = if let Some(w2) = w2 {
                if let Some(&fm) = self.gram_2.get(&(w2, w1)) {
                    Some(self.sum_2 / self.sum_3 / fm as f64)
                } else {
                    None
                }
            } else {
                None
            };
            pre[pos].iter().for_each(|candi| {
                let (nxt, words) = candi;
                let nxt = *nxt;
                let mut probs: Vec<Option<f64>> = vec![None; words.len()];

                if let Some(fm) = fm_2 {
                    words.iter().enumerate().for_each(|(i, &w)| {
                        let t = *self.gram_2.get(&(w1, w)).unwrap_or(&0);
                        let p = lbd * t as f64 * fm;
                        probs[i] = Some(probs[i].as_ref().map_or(p, |g| g.max(p)));
                    });
                }

                if let Some(fm) = fm_3 {
                    words.iter().enumerate().for_each(|(i, &w)| {
                        let t = *self.gram_3.get(&(w2.unwrap(), w1, w)).unwrap_or(&0);
                        let p = lbd * t as f64 * fm;
                        probs[i] = Some(probs[i].as_ref().map_or(p, |g| g.max(p)));
                    });
                }

                words.iter().enumerate().for_each(|(i, w)| {
                    if let Some(&t) = self.gram_1.get(w) {
                        let p = ilb * t as f64 / self.sum_1;
                        probs[i] = Some(probs[i].unwrap_or(0.0) + p);
                    }
                    if let Some(ref mut p) = probs[i] {
                        *p = p.ln();
                    }
                });

                words.iter().enumerate().for_each(|(i, &w)| {
                    if let Some(cp) = probs[i] {
                        let np = cp + p;
                        if ans.len() < max_ans || np > ans.last().unwrap().p {
                            if nxt == len {
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
                                let y = State { pos: nxt, w1: w, w2: Some(w1), prev: idx };
                                pool.push(y);
                                heap.push(HeapState { p: np, idx: pool.len() - 1 });
                            }
                        }
                    }
                });
            });
        }
        pool.push(Node { pos: 0, w1: 0, w2: 0, prev: 0, vis: false, p: 0.0 });
        heap.push(HeapState { p: 0.0, idx: 0 });
        node2id.insert((0, 0, 0), 0);
        // let mut cnt = 0;
        while let Some(HeapState { p: _, idx }) = heap.pop() {
            // cnt += 1;
            if pool[idx].vis {
                continue;
            }
            pool[idx].vis = true;
            let Node { pos, w1, w2, prev: _, vis: _, p } = pool[idx];
            pre[pos].iter().for_each(|candi| {
                let nxt = candi.0;
                let words = candi.1;
                let g_1 = &candi.2;
                let mut probs: Vec<f64> = vec![0.0; words.len()];

                for i in 0..words.len() {
                    if let Some(&p) = self.gram_2.get(&(w1, words[i])) {
                        probs[i] = probs[i].max(p);
                    }
                }

                for i in 0..words.len() {
                    if let Some(&p) = self.gram_3.get(&(w2, w1, words[i])) {
                        probs[i] = probs[i].max(p);
                    }
                }

                for i in 0..words.len() {
                    let np = p + (probs[i] + g_1[i]).ln();
                    if ans.is_none() || np > ans.unwrap().p {
                        let w = words[i];
                        if nxt == len {
                            ans = Some(AnsState { p: np, w, prev: idx });
                        } else {
                            let nid = *node2id.entry((nxt, w, w1)).or_insert_with(|| {
                                pool.push(Node {
                                    pos: nxt,
                                    w1: w,
                                    w2: w1,
                                    prev: idx,
                                    vis: false,
                                    p: np - 1.0,
                                });
                                pool.len() - 1
                            });
                            if pool[nid].p < np {
                                assert!(!pool[nid].vis);
                                pool[nid].prev = idx;
                                pool[nid].p = np;
                                heap.push(HeapState { p: np, idx: nid });
                            }
                        }
                    }
                }
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
            ans_s.push((s, st.p));
        });
        ans_s
    }
    */

    /*
    pub fn evals(&self, pinyin: &str, max_ans: usize) -> Vec<(String, f64)> {
        if pinyin.len() == 0 || max_ans == 0 {
            return Vec::new();
        }
        vec![]
        // self.kth_shortest_small(pinyin, max_ans)
    }
    */

    pub fn eval(&self, pinyin: &str) -> Option<(String, f64)> {
        if pinyin.len() == 0 {
            return None;
        }
        let (len, pre) = self.preprocess(pinyin);
        let mut heap = BinaryHeap::new();
        let mut pool = vec![];
        let mut node2id = HashMap::with_hasher(HB::default());
        let mut ans: Option<AnsState> = None;
        /*
        pool.push(Node { pos: 0, w1: 0, prev: 0, vis: false, p: 0.0 });
        heap.push(HeapState { p: 0.0, idx: 0 });
        node2id.insert((0, 0), 0);
        // let mut cnt = 0;
        while let Some(HeapState { p: _, idx }) = heap.pop() {
            // cnt += 1;
            if pool[idx].vis {
                continue;
            }
            pool[idx].vis = true;
            let Node { pos, w1, prev: _, vis: _, p } = pool[idx];
            pre[pos].iter().for_each(|candi| {
                let nxt = candi.0;
                let words = candi.1;
                let g_1 = &candi.2;
                let mut probs: Vec<f64> = vec![0.0; words.len()];

                for i in 0..words.len() {
                    if let Some(&p) = self.gram.get(&(w1, words[i])) {
                        probs[i] = probs[i].max(p);
                    }
                }

                for i in 0..words.len() {
                    let np = p + (probs[i] + g_1[i]).ln();
                    if ans.is_none() || np > ans.unwrap().p {
                        let w = words[i];
                        if nxt == len {
                            ans = Some(AnsState { p: np, w, prev: idx });
                        } else {
                            let nid = *node2id.entry((nxt, w)).or_insert_with(|| {
                                pool.push(Node {
                                    pos: nxt,
                                    w1: w,
                                    prev: idx,
                                    vis: false,
                                    p: np - 1.0,
                                });
                                pool.len() - 1
                            });
                            if pool[nid].p < np {
                                assert!(!pool[nid].vis);
                                pool[nid].prev = idx;
                                pool[nid].p = np;
                                heap.push(HeapState { p: np, idx: nid });
                            }
                        }
                    }
                }
            });
        }
        */

        /*
        pool.push(Node { pos: 0, w1: 0, w2: 0, prev: 0, vis: false, p: 0.0 });
        heap.push(HeapState { p: 0.0, idx: 0 });
        node2id.insert((0, 0, 0), 0);
        // let mut cnt = 0;
        while let Some(HeapState { p: _, idx }) = heap.pop() {
            // cnt += 1;
            if pool[idx].vis {
                continue;
            }
            pool[idx].vis = true;
            let Node { pos, w1, w2, prev: _, vis: _, p } = pool[idx];
            pre[pos].iter().for_each(|candi| {
                let nxt = candi.0;
                let words = candi.1;
                let g_1 = &candi.2;
                let mut probs: Vec<f64> = vec![0.0; words.len()];

                for i in 0..words.len() {
                    if let Some(&p) = self.gram.get(&(w2, w1, words[i])) {
                        probs[i] = probs[i].max(p);
                    }
                }

                for i in 0..words.len() {
                    let np = p + (probs[i] + g_1[i]).ln();
                    if ans.is_none() || np > ans.unwrap().p {
                        let w = words[i];
                        if nxt == len {
                            ans = Some(AnsState { p: np, w, prev: idx });
                        } else {
                            let nid = *node2id.entry((nxt, w, w1)).or_insert_with(|| {
                                pool.push(Node {
                                    pos: nxt,
                                    w1: w,
                                    w2: w1,
                                    prev: idx,
                                    vis: false,
                                    p: np - 1.0,
                                });
                                pool.len() - 1
                            });
                            if pool[nid].p < np {
                                assert!(!pool[nid].vis);
                                pool[nid].prev = idx;
                                pool[nid].p = np;
                                heap.push(HeapState { p: np, idx: nid });
                            }
                        }
                    }
                }
            });
        }
        */

        /*
        pool.push(Node { pos: 0, w1: 0, w2: 0, w3: 0, prev: 0, vis: false, p: 0.0 });
        heap.push(HeapState { p: 0.0, idx: 0 });
        node2id.insert((0, 0, 0, 0), 0);
        // let mut cnt = 0;
        while let Some(HeapState { p: _, idx }) = heap.pop() {
            // cnt += 1;
            if pool[idx].vis {
                continue;
            }
            pool[idx].vis = true;
            let Node { pos, w1, w2, w3, prev: _, vis: _, p } = pool[idx];
            pre[pos].iter().for_each(|candi| {
                let nxt = candi.0;
                let words = candi.1;
                let g_1 = &candi.2;
                let mut probs: Vec<f64> = vec![0.0; words.len()];

                for i in 0..words.len() {
                    if let Some(&p) = self.gram.get(&(w3, w2, w1, words[i])) {
                        probs[i] = probs[i].max(p);
                    }
                }

                for i in 0..words.len() {
                    let np = p + (probs[i] + g_1[i]).ln();
                    if ans.is_none() || np > ans.unwrap().p {
                        let w = words[i];
                        if nxt == len {
                            ans = Some(AnsState { p: np, w, prev: idx });
                        } else {
                            let nid = *node2id.entry((nxt, w, w1, w2)).or_insert_with(|| {
                                pool.push(Node {
                                    pos: nxt,
                                    w1: w,
                                    w2: w1,
                                    w3: w2,
                                    prev: idx,
                                    vis: false,
                                    p: np - 1.0,
                                });
                                pool.len() - 1
                            });
                            if pool[nid].p < np {
                                assert!(!pool[nid].vis);
                                pool[nid].prev = idx;
                                pool[nid].p = np;
                                heap.push(HeapState { p: np, idx: nid });
                            }
                        }
                    }
                }
            });
        }
        */

        pool.push(Node { pos: 0, w1: 0, w2: 0, prev: 0, vis: false, p: 0.0 });
        heap.push(HeapState { p: 0.0, idx: 0 });
        node2id.insert((0, 0, 0), 0);
        // let mut cnt = 0;
        while let Some(HeapState { p: _, idx }) = heap.pop() {
            // cnt += 1;
            if pool[idx].vis {
                continue;
            }
            pool[idx].vis = true;
            let Node { pos, w1, w2, prev: _, vis: _, p } = pool[idx];
            pre[pos].iter().for_each(|candi| {
                let nxt = candi.0;
                let words = candi.1;
                let g_1 = &candi.2;
                let mut probs: Vec<f64> = vec![0.0; words.len()];

                for i in 0..words.len() {
                    if let Some(&p) = self.gram_2.get(&(w1, words[i])) {
                        probs[i] = probs[i].max(p);
                    }
                }

                for i in 0..words.len() {
                    if let Some(&p) = self.gram_3.get(&(w2, w1, words[i])) {
                        probs[i] = probs[i].max(p);
                    }
                }

                for i in 0..words.len() {
                    let np = p + (probs[i] + g_1[i]).ln();
                    if ans.is_none() || np > ans.unwrap().p {
                        let w = words[i];
                        if nxt == len {
                            ans = Some(AnsState { p: np, w, prev: idx });
                        } else {
                            let nid = *node2id.entry((nxt, w, w1)).or_insert_with(|| {
                                pool.push(Node {
                                    pos: nxt,
                                    w1: w,
                                    w2: w1,
                                    prev: idx,
                                    vis: false,
                                    p: np - 1.0,
                                });
                                pool.len() - 1
                            });
                            if pool[nid].p < np {
                                assert!(!pool[nid].vis);
                                pool[nid].prev = idx;
                                pool[nid].p = np;
                                heap.push(HeapState { p: np, idx: nid });
                            }
                        }
                    }
                }
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
        // println!("{}", st.p);
        Some((s, st.p))
    }

    pub fn eval_from(&self, input_path: &str, answer_path: &str, output_path: &str) {
        println!("Predicting...");
        let s_time = time::Instant::now();
        let num = current_num_threads();
        let max_lines = 200;
        let input = File::open(input_path).expect(&format!("Cannot read {}", input_path));
        let reader = BufReader::with_capacity(1024 * 1024 * 32, input);
        let ans = File::open(answer_path).expect(&format!("Cannot read {}", answer_path));
        let answer = BufReader::with_capacity(1024 * 1024 * 32, ans);
        let output = File::create(output_path).expect(&format!("Cannot open {}", output_path));
        let mut writer = BufWriter::with_capacity(1024 * 1024 * 32, output);
        let mut tot = 0;
        let (mut sum_acc, mut sum_f1) = (0, 0.0);
        reader.max_lines(max_lines).zip(answer.max_lines(max_lines)).for_each(|(a, b)| {
            let slice: Vec<_> = a.into_iter().zip(b.into_iter()).enumerate().collect();
            let preds = Mutex::new(vec![String::new(); slice.len()]);
            let (a, b) = (Mutex::new(0), Mutex::new(0.0));
            slice.par_chunks((slice.len() + num - 1) / num).for_each(|lines| {
                let mut tmp_preds = Vec::with_capacity(lines.len());
                let mut tmp_acc = 0;
                let mut tmp_f1 = 0.0;
                lines.iter().for_each(|(i, (line, ans))| {
                    if let Ok(line) = line {
                        let pred = self.eval(line);
                        let pred = if pred.is_none() { String::new() } else { pred.unwrap().0 };
                        let mut a = &String::new();
                        if let Ok(ans) = ans {
                            a = ans;
                        }
                        let (acc, f1) = score(a, &pred);
                        tmp_acc += acc;
                        tmp_f1 += f1;
                        tmp_preds.push((i, pred));
                    } else {
                        tmp_preds.push((i, String::new()));
                    }
                });
                let mut r = preds.lock().unwrap();
                tmp_preds.into_iter().for_each(|(i, s)| r[*i] = s);
                *a.lock().unwrap() += tmp_acc;
                *b.lock().unwrap() += tmp_f1;
            });
            tot += slice.len();
            writer.write(preds.lock().unwrap().join("\n").as_bytes()).unwrap();
            sum_acc += *a.lock().unwrap();
            sum_f1 += *b.lock().unwrap();
            println!("...Finished {}", tot);
        });
        println!("Total lines: {}", tot);
        println!("acc:   {:.3}", sum_acc as f64 / tot as f64);
        println!("f1:    {:.3}", sum_f1 as f64 / tot as f64);
        let mils = (time::Instant::now() - s_time).as_millis();
        let mins = mils / 1000 / 60;
        let secs = mils / 1000 - mins * 60;
        println!("Total cost {}m {}s.", mins, secs);
        println!("Exit...");
    }

    pub fn eval_from_only(&self, input_path: &str, output_path: &str) {
        println!("Predicting...");
        let s_time = time::Instant::now();
        let num = current_num_threads();
        let max_lines = 200;
        let input = File::open(input_path).expect(&format!("Cannot read {}", input_path));
        let reader = BufReader::with_capacity(1024 * 1024 * 32, input);
        let output = File::create(output_path).expect(&format!("Cannot open {}", output_path));
        let mut writer = BufWriter::with_capacity(1024 * 1024 * 32, output);
        let mut tot = 0;
        reader.max_lines(max_lines).for_each(|slice| {
            let slice: Vec<_> = slice.into_iter().enumerate().collect();
            let preds = Mutex::new(vec![String::new(); slice.len()]);
            slice.par_chunks((slice.len() + num - 1) / num).for_each(|lines| {
                let mut tmp_preds = Vec::with_capacity(lines.len());
                lines.iter().for_each(|(i, line)| {
                    if let Ok(line) = line {
                        let pred = self.eval(line);
                        let pred = if pred.is_none() { String::new() } else { pred.unwrap().0 };
                        tmp_preds.push((i, pred));
                    } else {
                        tmp_preds.push((i, String::new()));
                    }
                });
                let mut r = preds.lock().unwrap();
                tmp_preds.into_iter().for_each(|(i, s)| r[*i] = s);
            });
            tot += slice.len();
            writer.write(preds.lock().unwrap().join("\n").as_bytes()).unwrap();
            println!("...Finished {}", tot);
        });
        println!("Total lines: {}", tot);
        let mils = (time::Instant::now() - s_time).as_millis();
        let mins = mils / 1000 / 60;
        let secs = mils / 1000 - mins * 60;
        println!("Total cost {}m {}s.", mins, secs);
        println!("Exit...");
    }

    pub fn new(config_path: &str, data_path: &str) -> Self {
        macro_rules! pt {
            ($b:expr) => {
                &format!("{}/{}", data_path, $b)
            };
        }

        let (lambda, max_len) = load::load_config(config_path);
        let s_time = time::Instant::now();

        println!("Initializing Pinyin IME (lambda: {}, max_len: {})", lambda, max_len);
        let (hanzi_v, hanzi_m) = load::load_hanzi(pt!("hanzi.txt"));
        let (word_v, pinyin_m) = load::load_eval_word(pt!("word.txt"), &hanzi_m);

        let mut gram_1 = load::load_gram_1(pt!("gram_1.txt"));
        for i in 0..word_v.len() {
            gram_1.entry(i).or_insert(1.0);
        }
        let sum_1: f64 = gram_1.iter().map(|(_, v)| *v).sum();
        gram_1.iter_mut().for_each(|(_, v)| *v = *v * (1.0 - lambda) / sum_1);

        /*
        let mut gram = load::load_gram_2(pt!("gram_2.txt"));
        let mut sum = HashMap::with_hasher(HB::default());
        gram.iter().for_each(|(k, v)| {
            *sum.entry(k.0).or_insert(0.0) += v;
        });
        gram.iter_mut().for_each(|(k, v)| *v = *v * lambda / sum.get(&k.0).unwrap());
        */

        /*
        let mut gram = load::load_gram_3(pt!("gram_3.txt"));
        let mut sum = HashMap::with_hasher(HB::default());
        println!("Pre-processing");
        gram.iter().for_each(|(k, v)| {
            *sum.entry((k.0, k.1)).or_insert(0.0) += v;
        });
        gram.iter_mut().for_each(|(k, v)| *v = *v * lambda / sum.get(&(k.0, k.1)).unwrap());
        */

        /*
        let mut gram = load::load_gram_4(pt!("gram_4.txt"));
        let mut sum = HashMap::with_hasher(HB::default());
        gram.iter().for_each(|(k, v)| {
            *sum.entry((k.0, k.1, k.2)).or_insert(0.0) += v;
        });
        gram.iter_mut().for_each(|(k, v)| *v = *v * lambda / sum.get(&(k.0, k.1, k.2)).unwrap());
        */

        let mut gram = load::load_gram_2(pt!("gram_2.txt"));
        let mut sum = HashMap::with_hasher(HB::default());
        gram.iter().for_each(|(k, v)| {
            *sum.entry(k.0).or_insert(0.0) += v;
        });
        gram.iter_mut().for_each(|(k, v)| *v = *v * lambda / sum.get(&k.0).unwrap());
        let gram_2 = gram;

        let mut gram = load::load_gram_3(pt!("gram_3.txt"));
        let mut sum = HashMap::with_hasher(HB::default());
        println!("Pre-processing");
        gram.iter().for_each(|(k, v)| {
            *sum.entry((k.0, k.1)).or_insert(0.0) += v;
        });
        gram.iter_mut().for_each(|(k, v)| *v = *v * lambda / sum.get(&(k.0, k.1)).unwrap());
        let gram_3 = gram;

        let mils = (time::Instant::now() - s_time).as_millis();
        let mins = mils / 1000 / 60;
        let secs = mils / 1000 - mins * 60;
        println!("Done! Total cost {}m {}s.", mins, secs);

        Self { hanzi_v, word_v, pinyin_m, gram_1, gram_2, gram_3, max_len }
    }
}
