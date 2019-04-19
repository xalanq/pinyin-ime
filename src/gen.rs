use crate::load::{hanzi2word, word2hanzi};
use crate::max_lines::*;
use crate::HB;
use jieba_rs::Jieba;
use pbr::ProgressBar;
use pinyin;
use rand::{seq::SliceRandom, thread_rng};
use rayon::current_num_threads;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::sync::Mutex;
use std::time::{self, Duration};

fn gen_gram_one(
    path: &str,
    gram_1: &mut HashMap<usize, usize, HB>,
    gram_2: &mut HashMap<(usize, usize), usize, HB>,
    gram_3: &mut HashMap<(usize, usize, usize), usize, HB>,
    gram_4: &mut HashMap<(usize, usize, usize, usize), usize, HB>,
    hanzi_m: &HashMap<char, usize, HB>,
    word_m: &HashMap<Vec<usize>, usize, HB>,
    jb: &Jieba,
    limit: (usize, usize, usize),
) {
    println!("...Working on {}", path);
    let file = File::open(path).expect(&format!("......Cannot open {}", path));
    let buf = BufReader::with_capacity(1024 * 1024 * 32, file);
    let max_lines = 100000;
    let mut it = buf.max_lines(max_lines);
    let tot_lines = it.single().unwrap().unwrap().trim().parse::<usize>().unwrap();
    let num = current_num_threads();
    let mut pb = ProgressBar::new(((tot_lines + max_lines - 1) / max_lines) as u64);
    pb.set_max_refresh_rate(Some(Duration::from_secs(1)));

    let t1 = Mutex::new(HashMap::with_hasher(HB::default()));
    let t2 = Mutex::new(HashMap::with_hasher(HB::default()));
    let t3 = Mutex::new(HashMap::with_hasher(HB::default()));
    let t4 = Mutex::new(HashMap::with_hasher(HB::default()));
    it.for_each(|slice| {
        slice.par_chunks((slice.len() + num - 1) / num).for_each(|lines| {
            let mut g1 = HashMap::with_hasher(HB::default());
            let mut g2 = HashMap::with_hasher(HB::default());
            let mut g3 = HashMap::with_hasher(HB::default());
            let mut g4 = HashMap::with_hasher(HB::default());
            let mut line = String::new();
            lines.iter().for_each(|l| {
                if let Ok(l) = l {
                    line.push_str(l);
                    line.push(' ');
                }
            });
            jb.cut(&line, false).split(|c| *c == " ").for_each(|ws| {
                let sen: Vec<_> =
                    ws.iter().map(|w| hanzi2word(*w, hanzi_m, word_m).unwrap()).collect();
                // start
                if sen.len() > 0 && sen[0] != 1 {
                    *g2.entry((0, sen[0])).or_insert(0) += 1;
                    *g3.entry((0, 0, sen[0])).or_insert(0) += 1;
                    *g4.entry((0, 0, 0, sen[0])).or_insert(0) += 1;
                    if sen.len() > 1 && sen[1] != 1 {
                        *g3.entry((0, sen[0], sen[1])).or_insert(0) += 1;
                        *g4.entry((0, 0, sen[0], sen[1])).or_insert(0) += 1;
                        if sen.len() > 2 && sen[2] != 1 {
                            *g4.entry((0, sen[0], sen[1], sen[2])).or_insert(0) += 1;
                        }
                    }
                }
                for i in (0..sen.len()).rev() {
                    let x = sen[i];
                    if x == 1 {
                        continue; // unknown
                    }
                    *g1.entry(x).or_insert(0) += 1;
                    if i + 1 < sen.len() {
                        let y = sen[i + 1];
                        if y == 1 {
                            continue; // unknown
                        }
                        *g2.entry((x, y)).or_insert(0) += 1;
                        if i + 2 < sen.len() {
                            let z = sen[i + 2];
                            if z == 1 {
                                continue; // unknown
                            }
                            *g3.entry((x, y, z)).or_insert(0) += 1;
                            if i + 3 < sen.len() {
                                let t = sen[i + 3];
                                if t == 1 {
                                    continue; // unknown
                                }
                                *g4.entry((x, y, z, t)).or_insert(0) += 1;
                            }
                        }
                    }
                }
            });
            let mut tt1 = t1.lock().unwrap();
            let mut tt2 = t2.lock().unwrap();
            let mut tt3 = t3.lock().unwrap();
            let mut tt4 = t4.lock().unwrap();
            g1.iter().for_each(|(k, v)| *tt1.entry(*k).or_insert(0) += v);
            g2.iter().for_each(|(k, v)| *tt2.entry(*k).or_insert(0) += v);
            g3.iter().for_each(|(k, v)| *tt3.entry(*k).or_insert(0) += v);
            g4.iter().for_each(|(k, v)| *tt4.entry(*k).or_insert(0) += v);
        });
        join!(
            || t2.lock().unwrap().retain(|_, v| *v >= limit.0),
            || t3.lock().unwrap().retain(|_, v| *v >= limit.1),
            || t4.lock().unwrap().retain(|_, v| *v >= limit.2)
        );
        pb.add(1);
    });
    join!(
        || t1.into_inner().unwrap().iter().for_each(|(k, v)| *gram_1.entry(*k).or_insert(0) += v),
        || t2.into_inner().unwrap().iter().for_each(|(k, v)| *gram_2.entry(*k).or_insert(0) += v),
        || t3.into_inner().unwrap().iter().for_each(|(k, v)| *gram_3.entry(*k).or_insert(0) += v),
        || t4.into_inner().unwrap().iter().for_each(|(k, v)| *gram_4.entry(*k).or_insert(0) += v)
    );
    join!(|| gram_2.retain(|_, v| *v >= limit.0), || gram_3.retain(|_, v| *v >= limit.1), || {
        gram_4.retain(|_, v| *v >= limit.2)
    });
    pb.finish_println("");
    println!(
        "......len of gram 1: {}\n......len of gram 2: {}\n\
         ......len of gram 3: {}\n......len of gram 4: {}",
        gram_1.len(),
        gram_2.len(),
        gram_3.len(),
        gram_4.len(),
    );
}

pub fn gen_gram(
    path: &str,
    save_path: &str,
    k_th: (usize, usize, usize),
    hanzi_m: &HashMap<char, usize, HB>,
    word_m: &HashMap<Vec<usize>, usize, HB>,
    jb: &Jieba,
    gc: (usize, usize, usize),
) {
    println!("Generating gram-n");
    let s_time = time::Instant::now();
    let paths = fs::read_dir(path).unwrap();
    let mut gram_1 = HashMap::with_hasher(HB::default());
    let mut gram_2 = HashMap::with_hasher(HB::default());
    let mut gram_3 = HashMap::with_hasher(HB::default());
    let mut gram_4 = HashMap::with_hasher(HB::default());
    for path in paths {
        let path = path.unwrap();
        if path.metadata().unwrap().is_file() {
            gen_gram_one(
                &path.path().display().to_string(),
                &mut gram_1,
                &mut gram_2,
                &mut gram_3,
                &mut gram_4,
                hanzi_m,
                word_m,
                jb,
                gc,
            );
        }
    }

    println!("...summaring");

    let (mut gram_1, mut gram_2, mut gram_3, mut gram_4) = join!(
        || gram_1.into_iter().collect::<Vec<_>>(),
        || gram_2.into_iter().collect::<Vec<_>>(),
        || gram_3.into_iter().collect::<Vec<_>>(),
        || gram_4.into_iter().collect::<Vec<_>>()
    );

    macro_rules! short {
        ($g:ident, $kk:expr) => {{
            let k = $g.len().min($kk);
            $g.sort_unstable_by(|(_, b), (_, d)| d.cmp(b));
            $g.truncate(k);
        }};
    }

    gram_1.sort_unstable_by(|(_, b), (_, d)| d.cmp(b));
    join!(|| short!(gram_2, k_th.0), || short!(gram_3, k_th.1), || short!(gram_4, k_th.2));
    let gram_1: Vec<_> = gram_1.into_iter().map(|(k, v)| format!("{} {}\n", k, v)).collect();
    let gram_2: Vec<_> =
        gram_2.into_iter().map(|(k, v)| format!("{} {} {}\n", k.0, k.1, v)).collect();
    let gram_3: Vec<_> =
        gram_3.into_iter().map(|(k, v)| format!("{} {} {} {}\n", k.0, k.1, k.2, v)).collect();
    let gram_4: Vec<_> = gram_4
        .into_iter()
        .map(|(k, v)| format!("{} {} {} {} {}\n", k.0, k.1, k.2, k.3, v))
        .collect();

    println!(
        "......len of gram 1: {}\n......len of gram 2: {}\n\
         ......len of gram 3: {}\n......len of gram 4: {}",
        gram_1.len(),
        gram_2.len(),
        gram_3.len(),
        gram_4.len(),
    );

    macro_rules! save {
        ($g:ident, $i:expr) => {{
            println!("...writing gram_{}.txt", $i);
            let fname = &format!("{}/gram_{}.txt", save_path, $i);
            let file = File::create(fname).expect(&format!("Cannot save to {}", fname));
            let mut writer = BufWriter::with_capacity(1024 * 1024 * 32, file);
            $g.iter().for_each(|s| {
                writer.write(s.as_bytes()).unwrap();
            });
        }};
    }
    join!(|| save!(gram_1, 1), || save!(gram_2, 2), || save!(gram_3, 3), || save!(gram_4, 4));

    println!("...Done!");
    let mils = (time::Instant::now() - s_time).as_millis();
    let mins = mils / 1000 / 60;
    let secs = mils / 1000 - mins * 60;
    println!("Total cost {}m {}s.", mins, secs);
}

pub fn gen_total_gram(path: &str) {
    println!("Generating total gram-n");
    let config = &format!("{}/gram.json", path);
    let data = fs::read_to_string(config).expect(&format!("...Unable to read {}", config));
    let data = serde_json::from_str::<Value>(&data).expect("...Cannot convert to json");
    let data = data.as_object().expect("...Invalid json");
    let mut sum = 0.0;
    let mut gram_1 = HashMap::with_hasher(HB::default());
    let mut gram_2 = HashMap::with_hasher(HB::default());
    let mut gram_3 = HashMap::with_hasher(HB::default());
    let mut gram_4 = HashMap::with_hasher(HB::default());
    data.iter().for_each(|(k, v)| {
        let w = v.as_f64().expect("...Invalid number");
        macro_rules! go {
            ($g:expr, $i:expr) => {{
                let fname = &format!("{}/{}/gram_{}.txt", path, k, $i);
                println!("...Working on {}", fname);
                let file = File::open(fname).expect(&format!("......Cannot open {}", fname));
                let buf = BufReader::with_capacity(1024 * 1024 * 32, file);
                buf.lines().for_each(|line| {
                    let line = line.unwrap();
                    let pos = line.rfind(" ").unwrap();
                    let p = line[pos + 1..].parse::<f64>().unwrap();
                    let ws = line[0..pos].to_string();
                    *$g.entry(ws).or_insert(0.0) += p * w;
                });
            }};
        }
        join!(|| go!(gram_1, 1), || go!(gram_2, 2), || go!(gram_3, 3), || go!(gram_4, 4));
        sum += w;
    });
    let (mut gram_1, mut gram_2, mut gram_3, mut gram_4) = join!(
        || gram_1.iter().map(|(k, v)| (k, *v as usize)).collect::<Vec<_>>(),
        || gram_2.iter().map(|(k, v)| (k, *v as usize)).collect::<Vec<_>>(),
        || gram_3.iter().map(|(k, v)| (k, *v as usize)).collect::<Vec<_>>(),
        || gram_4.iter().map(|(k, v)| (k, *v as usize)).collect::<Vec<_>>()
    );
    macro_rules! save {
        ($g:expr, $i:expr) => {{
            $g.sort_unstable_by(|(_, b), (_, d)| d.cmp(b));
            let fname = &format!("{}/gram_{}.txt", path, $i);
            let file = File::create(fname).expect(&format!("Cannot save to {}", fname));
            let mut writer = BufWriter::with_capacity(1024 * 1024 * 32, file);
            $g.iter().for_each(|(a, b)| {
                if *b != 0 {
                    writer.write(format!("{} {}\n", a, b).as_bytes()).unwrap();
                }
            })
        }};
    }
    join!(|| save!(gram_1, 1), || save!(gram_2, 2), || save!(gram_3, 3), || save!(gram_4, 4));
    println!("...Done!");
}

fn word_filter(
    s: &str,
    data: &mut HashSet<String>,
    hanzi_m: &HashMap<char, usize, HB>,
    max_len: usize,
) {
    s.split_whitespace().for_each(|a| {
        let mut valid = true;
        let mut len = 0;
        for c in a.chars() {
            if hanzi_m.get(&c).is_none() {
                valid = false;
                break;
            }
            len += 1;
        }
        if valid && len > 1 && len <= max_len {
            data.insert(a.to_string());
        }
    });
}

fn gen_word_one(
    path: &str,
    data: &mut HashSet<String>,
    hanzi_m: &HashMap<char, usize, HB>,
    max_len: usize,
) {
    println!("...Working on {}", path);
    word_filter(
        &fs::read_to_string(path).expect(&format!("......Cannot read {}", path)),
        data,
        hanzi_m,
        max_len,
    );
    println!("......total len: {}", data.len());
}

pub fn gen_word(
    path: &str,
    ref_path: &str,
    save_path: &str,
    hanzi_m: &HashMap<char, usize, HB>,
    max_len: usize,
) {
    println!("Generating word");
    let paths = fs::read_dir(path).unwrap();
    let mut data = HashSet::new();
    hanzi_m.iter().for_each(|c| {
        data.insert(c.0.to_string());
    });
    for path in paths {
        let path = path.unwrap();
        if path.metadata().unwrap().is_file() {
            gen_word_one(&path.path().display().to_string(), &mut data, hanzi_m, max_len);
        }
    }
    let mut data: Vec<_> = data.into_iter().collect();
    data.sort_by(|a, b| if a.len() != b.len() { a.len().cmp(&b.len()) } else { a.cmp(b) });

    let mut ref_py = HashMap::with_hasher(HB::default());
    fs::read_to_string(ref_path).expect(&format!("...Cannot read {}", ref_path)).lines().for_each(
        |line| {
            let mut data: Vec<_> = line.split_whitespace().collect();
            let mut valid = true;
            for c in data[0].chars() {
                if hanzi_m.get(&c).is_none() {
                    valid = false;
                    break;
                }
            }
            if valid {
                if data[2].contains("%") {
                    data[2..].sort_by(|a, b| {
                        let x: String = a.chars().filter(|c| c.is_digit(10) || *c == '.').collect();
                        let y: String = b.chars().filter(|c| c.is_digit(10) || *c == '.').collect();
                        y.parse::<f64>().unwrap().partial_cmp(&x.parse::<f64>().unwrap()).unwrap()
                    })
                }
                data[2..].iter().for_each(|s| {
                    ref_py
                        .entry(data[0].to_string())
                        .or_insert(Vec::new())
                        .push(s.split(':').next().unwrap().to_string())
                });
            }
        },
    );

    let mut args = pinyin::Args::new();
    args.heteronym = true;
    let errmsg = &format!("...Cannot save to {}", save_path);
    let mut pb = ProgressBar::new(data.len() as u64);
    pb.set_max_refresh_rate(Some(Duration::from_secs(1)));
    File::create(save_path)
        .expect(errmsg)
        .write_all(
            data.iter()
                .fold(&mut String::new(), |s, a| {
                    match ref_py.get(a) {
                        Some(p) => s.push_str(&format!("{} {}\n", a, p.join(" "))),
                        None => s.push_str(&format!(
                            "{} {}\n",
                            a,
                            pinyin::lazy_pinyin(a, &args).join("'")
                        )),
                    }
                    pb.add(1);
                    s
                })
                .as_bytes(),
        )
        .expect(errmsg);
    pb.finish_println("");
    println!("...Done!");
}

pub fn gen_jieba_dict(save_path: &str, hanzi_v: &Vec<char>, word_v: &Vec<Vec<usize>>) {
    println!("Generating jieba dictionary");
    let jb = Jieba::new();
    let errmsg = &format!("...Cannot save to {}", save_path);
    File::create(save_path)
        .expect(errmsg)
        .write_all(
            (2..word_v.len())
                .fold(&mut String::new(), |s, i| {
                    let w = word2hanzi(i, hanzi_v, word_v);
                    let freq = jb.suggest_freq(&w);
                    s.push_str(&format!("{} {}\n", w, freq));
                    s
                })
                .as_bytes(),
        )
        .expect(errmsg);
    println!("...Done!");
}

pub fn gen_sen<F>(path: &str, save_path: &str, hanzi_m: &HashMap<char, usize, HB>, gen_sen_one: F)
where
    F: Fn(&str, &mut BufWriter<File>, &HashMap<char, usize, HB>) -> usize,
{
    println!("Generating sina sentence");
    let paths = fs::read_dir(path).unwrap();
    let mut count = 0;
    let file = File::create(save_path).expect(&format!("Cannot save to {}", save_path));
    let mut writer = BufWriter::with_capacity(1024 * 1024 * 32, file);
    writer.write(format!("{}\n", " ".repeat(20)).as_bytes()).unwrap();
    for path in paths {
        let path = path.unwrap();
        if path.metadata().unwrap().is_file() {
            count += gen_sen_one(&path.path().display().to_string(), &mut writer, hanzi_m);
        }
        println!("......total line: {}", count);
    }
    writer.seek(SeekFrom::Start(0)).unwrap();
    writer.write(&format!("{}", count).as_bytes()).unwrap();
    println!("...Done!");
}

pub fn digit2chinese(s: &[char]) -> String {
    let v: Vec<_> = s
        .iter()
        .map(|c| match *c {
            '0' => '零',
            '1' => '一',
            '2' => '二',
            '3' => '三',
            '4' => '四',
            '5' => '五',
            '6' => '六',
            '7' => '七',
            '8' => '八',
            '9' => '九',
            _ => *c,
        })
        .collect();
    let mut s = String::new();
    macro_rules! p {
        ( $($c:expr),+ ) => {{
            $(
                s.push($c);
            )+
        }};
    }
    if v.len() != 2 && v[0] == '零' {
        v.iter().for_each(|c| p!(*c));
    } else {
        match v.len() {
            1 => p!(v[0]),
            2 => match v[0] {
                '零' => p!(v[1]),
                _ => {
                    if v[0] != '一' {
                        p!(v[0]);
                    }
                    p!('十');
                    if v[1] != '零' {
                        p!(v[1]);
                    }
                }
            },
            3 => {
                p!(v[0], '百');
                if v[1] == '零' {
                    if v[2] != '零' {
                        p!('零', v[2]);
                    }
                } else {
                    p!(v[1], '十');
                    if v[2] != '零' {
                        p!(v[2]);
                    }
                };
            }
            _ => v.iter().for_each(|c| p!(*c)),
        }
    };
    s
}

pub fn line_filter(s: &str, hanzi_m: &HashMap<char, usize, HB>) -> String {
    let mut len = 0;
    let mut valid = false;
    let s: Vec<_> = s.chars().collect();
    let mut data = String::new();
    let mut i = 0;
    let mut upd = |cc: char| {
        if !cc.is_alphanumeric()
            && !"~@#$%^&*()_+{}|<>`-=[]'/￥…（）—“”《》·【】、‘’.".contains(cc)
        {
            len = 0;
            return;
        }
        let mut c = cc;
        if hanzi_m.get(&c).is_none() {
            c = '_';
        }
        if len == 0 && valid {
            data.push(' ');
        }
        valid = true;
        data.push(c);
        len += 1;
    };
    while i < s.len() {
        if s[i].is_digit(10) {
            // use first and last digit
            let mut j = i + 1;
            while j < s.len() && s[j].is_digit(10) {
                j += 1;
            }
            digit2chinese(&s[i..j]).chars().for_each(|c| upd(c));
            i = j;
            continue;
        }
        upd(s[i]);
        i += 1;
    }
    data
}

pub fn gen_dev(
    path: &str,
    input_path: &str,
    answer_path: &str,
    hanzi_m: &HashMap<char, usize, HB>,
    word_m: &HashMap<Vec<usize>, usize, HB>,
    pinyin_v: &Vec<Vec<String>>,
    jb: &Jieba,
) {
    println!("Generating dev data");
    println!("...Working on {}", path);
    let input = fs::read_to_string(path).expect(&format!("......Cannot read {}", path));
    let mut data = Vec::new();
    let mut line_cnt = true;
    input.lines().for_each(|line| {
        if line_cnt {
            line_cnt = false;
            return;
        }
        line.split_whitespace().for_each(|ws| {
            let cnt = ws.chars().count();
            if ws.contains("_") || cnt < 5 || cnt > 15 {
                return;
            }
            let v: Vec<_> = jb
                .cut(&ws, false)
                .iter()
                .map(|w| pinyin_v[hanzi2word(w, hanzi_m, word_m).unwrap()][0].as_str())
                .collect();
            data.push((v.join("'").replace("'", " "), ws));
        });
    });
    data.shuffle(&mut thread_rng());
    data.truncate(3000);
    let pinyin = data.iter().map(|a| a.0.as_str()).collect::<Vec<_>>();
    let text = data.iter().map(|a| a.1).collect::<Vec<_>>();
    let errmsg = &format!("...Cannot save to {}", input_path);
    File::create(input_path).expect(errmsg).write_all(pinyin.join("\n").as_bytes()).expect(errmsg);
    let errmsg = &format!("...Cannot save to {}", answer_path);
    File::create(answer_path).expect(errmsg).write_all(text.join("\n").as_bytes()).expect(errmsg);
    println!("...Done!")
}
