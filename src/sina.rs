use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;

fn filter(s: &str, data: &mut String, hanzi_m: &HashMap<char, usize>) -> usize {
    let mut len = 0;
    let mut valid = false;
    let s: Vec<_> = s.chars().collect();
    let mut i = 0;
    while i < s.len() {
        let mut c = s[i];
        if c.is_digit(10) {
            // use last digit
            while i + 1 < s.len() && s[i + 1].is_digit(10) {
                i += 1;
            }
            c = match s[i] {
                '0' => '〇',
                '1' => '一',
                '2' => '二',
                '3' => '三',
                '4' => '四',
                '5' => '五',
                '6' => '六',
                '7' => '七',
                '8' => '八',
                '9' => '九',
                _ => s[i],
            };
        }
        if let Some(_) = hanzi_m.get(&c) {
            if len == 0 && valid {
                data.push(' ');
            }
            valid = true;
            data.push(c);
            len += 1;
        } else if c.is_alphabetic() {
            if len == 0 && valid {
                data.push(' ');
            }
            valid = true;
            data.push('_'); // unknown
            len += 1;
        } else {
            len = 0;
        }
        i += 1;
    }
    if valid {
        data.push('\n');
        1
    } else {
        0
    }
}

fn gen_sen_one(path: &str, data: &mut String, hanzi_m: &HashMap<char, usize>) -> usize {
    println!("...Working on {}", path);
    let mut count = 0;
    fs::read_to_string(path).expect(&format!("......Cannot read {}", path)).lines().for_each(
        |line| {
            if let Ok(mut raw) = serde_json::from_str::<Value>(&line) {
                let html: String =
                    serde_json::from_value(raw["html"].take()).expect("Invalid html");
                let title: String =
                    serde_json::from_value(raw["title"].take()).expect("Invalid title");
                count += filter(&html, data, hanzi_m);
                count += filter(&title, data, hanzi_m);
            }
        },
    );
    count
}

pub fn gen_sen(path: &str, save_path: &str, hanzi_m: &HashMap<char, usize>) {
    println!("Generating sina sentence");
    let paths = fs::read_dir(path).unwrap();
    let mut data = String::new();
    let mut count = 0;
    for path in paths {
        let path = path.unwrap();
        if path.metadata().unwrap().is_file() {
            count += gen_sen_one(&path.path().display().to_string(), &mut data, hanzi_m);
        }
        println!("......total len: {}\n......total line: {}", data.len(), count);
    }
    let errmsg = &format!("...Cannot save to {}", save_path);
    let mut file = File::create(save_path).expect(errmsg);
    println!("...writing to {}", save_path);
    file.write(format!("{}\n", count).as_bytes()).expect(errmsg);
    file.write_all(data.as_bytes()).expect(errmsg);
    println!("...Done!");
}
