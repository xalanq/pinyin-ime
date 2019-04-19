use crate::gen::line_filter;
use crate::max_lines::*;
use rayon::current_num_threads;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::sync::Mutex;

pub fn gen_sina(path: &str, writer: &mut BufWriter<File>, hanzi_m: &HashMap<char, usize>) -> usize {
    println!("...Working on {}", path);
    let file = File::open(path).expect(&format!("......Cannot open {}", path));
    let buf = BufReader::with_capacity(1024 * 1024 * 32, file);
    let max_lines = 100000;
    let num = current_num_threads();
    let data = Mutex::new(Vec::new());
    let count = Mutex::new(0);
    buf.max_lines(max_lines).for_each(|slice| {
        slice.par_chunks((slice.len() + num - 1) / num).for_each(|lines| {
            let mut data_tmp = vec![];
            let mut count_tmp = 0;
            lines.iter().for_each(|line| {
                if let Ok(mut raw) = serde_json::from_str::<Value>(line.as_ref().unwrap()) {
                    let mut tot = String::new();
                    let mut cnt = 0;
                    macro_rules! go {
                        ($key:expr) => {{
                            let s: String =
                                serde_json::from_value(raw[$key].take()).expect("Invalid html");
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
                        data_tmp.push(tot);
                        count_tmp += cnt;
                    }
                }
            });
            data.lock().unwrap().extend(data_tmp.into_iter());
            *count.lock().unwrap() += count_tmp;
        });
    });
    data.lock().unwrap().iter().for_each(|s| {
        if s.len() > 0 {
            writer.write(s.as_bytes()).unwrap();
        }
    });
    count.into_inner().unwrap()
}

pub fn gen_raw(path: &str, writer: &mut BufWriter<File>, hanzi_m: &HashMap<char, usize>) -> usize {
    println!("...Working on {}", path);
    let file = File::open(path).expect(&format!("......Cannot open {}", path));
    let buf = BufReader::with_capacity(1024 * 1024 * 32, file);
    let max_lines = 100000;
    let num = current_num_threads();
    let data = Mutex::new(Vec::new());
    let count = Mutex::new(0);
    buf.max_lines(max_lines).for_each(|slice| {
        slice.par_chunks((slice.len() + num - 1) / num).for_each(|lines| {
            let mut data_tmp = vec![];
            let mut count_tmp = 0;
            lines.iter().for_each(|line| {
                let mut s = line_filter(line.as_ref().unwrap(), hanzi_m);
                if s.len() > 0 {
                    s.push('\n');
                    data_tmp.push(s);
                    count_tmp += 1;
                }
            });
            data.lock().unwrap().extend(data_tmp.into_iter());
            *count.lock().unwrap() += count_tmp;
        });
    });
    data.lock().unwrap().iter().for_each(|s| {
        if s.len() > 0 {
            writer.write(s.as_bytes()).unwrap();
        }
    });
    count.into_inner().unwrap()
}
