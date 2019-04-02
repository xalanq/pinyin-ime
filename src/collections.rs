use crate::gen::line_filter;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;

pub fn gen_sina(path: &str, data: &mut String, hanzi_m: &HashMap<char, usize>) -> usize {
    println!("...Working on {}", path);
    let lines = fs::read_to_string(path).expect(&format!("......Cannot read {}", path));
    let mut lines: Vec<_> = lines.lines().collect();
    let sum = Mutex::new(vec![String::new(); lines.len()]);
    let count = Mutex::new(0);
    lines.par_iter().enumerate().for_each(|(i, line)| {
        if let Ok(mut raw) = serde_json::from_str::<Value>(&line) {
            let mut tot = String::new();
            let mut cnt = 0;
            macro_rules! go {
                ($key:expr) => {{
                    let s: String = serde_json::from_value(raw[$key].take()).expect("Invalid html");
                    let s = line_filter(&s, hanzi_m);
                    if s.len() > 0 {
                        tot.push_str(&s);
                        tot.push('\n');
                        cnt += 1;
                    }
                }};
            }
            go!("html");
            go!("title");
            if cnt > 0 {
                sum.lock().unwrap()[i] = tot;
                *count.lock().unwrap() += cnt;
            }
        }
    });
    lines.clear();
    lines.shrink_to_fit();
    sum.lock().unwrap().iter().for_each(|s| {
        if s.len() > 0 {
            data.push_str(s);
        }
    });
    let ret: usize = *count.lock().unwrap();
    ret
}

pub fn gen_raw(path: &str, data: &mut String, hanzi_m: &HashMap<char, usize>) -> usize {
    println!("...Working on {}", path);
    let lines = fs::read_to_string(path).expect(&format!("......Cannot read {}", path));
    let mut lines: Vec<_> = lines.lines().collect();
    let sum = Mutex::new(vec![String::new(); lines.len()]);
    let count = Mutex::new(0);
    lines.par_iter().enumerate().for_each(|(i, line)| {
        let s = line_filter(line, hanzi_m);
        if s.len() > 0 {
            sum.lock().unwrap()[i] = s;
            *count.lock().unwrap() += 1;
        }
    });
    lines.clear();
    lines.shrink_to_fit();
    sum.lock().unwrap().iter().for_each(|s| {
        if s.len() > 0 {
            data.push_str(s);
            data.push('\n');
        }
    });
    let ret: usize = *count.lock().unwrap();
    ret
}
