use crate::load::{hanzi2word, word2hanzi};
use jieba_rs::Jieba;
use pbr::ProgressBar;
use pinyin;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;

fn gen_gram_filter(
    s: &str,
    gram_1: &mut HashMap<usize, usize>,
    gram_2: &mut HashMap<(usize, usize), usize>,
    gram_3: &mut HashMap<(usize, usize, usize), usize>,
    gram_4: &mut HashMap<(usize, usize, usize, usize), usize>,
    hanzi_m: &HashMap<char, usize>,
    word_m: &HashMap<Vec<usize>, usize>,
    jb: &Jieba,
) {
    jb.cut(&s, false).split(|c| *c == " ").for_each(|ws| {
        let sen: Vec<_> = ws.iter().map(|w| hanzi2word(*w, hanzi_m, word_m)).collect();
        // start
        if sen.len() > 0 && sen[0] != 1 {
            *gram_2.entry((0, sen[0])).or_insert(0) += 1;
            if sen.len() > 1 && sen[1] != 1 {
                *gram_3.entry((0, sen[0], sen[1])).or_insert(0) += 1;
                if sen.len() > 2 && sen[2] != 1 {
                    *gram_4.entry((0, sen[0], sen[1], sen[2])).or_insert(0) += 1;
                }
            }
        }
        for i in (0..sen.len()).rev() {
            let x = sen[i];
            if x == 1 {
                continue; // unknown
            }
            *gram_1.entry(x).or_insert(0) += 1;
            if i + 1 < sen.len() {
                let y = sen[i + 1];
                if y == 1 {
                    continue; // unknown
                }
                *gram_2.entry((x, y)).or_insert(0) += 1;
                if i + 2 < sen.len() {
                    let z = sen[i + 2];
                    if z == 1 {
                        continue; // unknown
                    }
                    *gram_3.entry((x, y, z)).or_insert(0) += 1;
                    if i + 3 < sen.len() {
                        let t = sen[i + 3];
                        if t == 1 {
                            continue; // unknown
                        }
                        *gram_4.entry((x, y, z, t)).or_insert(0) += 1;
                    }
                }
            }
        }
    });
}

fn gen_gram_one(
    path: &str,
    gram_1: &mut HashMap<usize, usize>,
    gram_2: &mut HashMap<(usize, usize), usize>,
    gram_3: &mut HashMap<(usize, usize, usize), usize>,
    gram_4: &mut HashMap<(usize, usize, usize, usize), usize>,
    hanzi_m: &HashMap<char, usize>,
    word_m: &HashMap<Vec<usize>, usize>,
    jb: &Jieba,
) {
    println!("...Working on {}", path);
    let file = File::open(path).expect(&format!("......Cannot open {}", path));
    let mut iter = BufReader::with_capacity(1024 * 1024 * 32, file).lines();
    let mut pb = ProgressBar::new(iter.next().unwrap().unwrap().parse().unwrap());
    pb.set_max_refresh_rate(Some(Duration::from_secs(1)));
    let mut gc = 0;
    let limit = 5;
    macro_rules! gc {
        () => {
            gram_1.retain(|_, v| *v >= limit);
            gram_2.retain(|_, v| *v >= limit);
            gram_3.retain(|_, v| *v >= limit);
            gram_4.retain(|_, v| *v >= limit);
        };
    }
    for line in iter {
        gen_gram_filter(&line.unwrap(), gram_1, gram_2, gram_3, gram_4, hanzi_m, word_m, jb);
        pb.add(1);
        gc += 1;
        if gc % 30000 == 0 {
            gc!();
        }
    }
    gc!();
    pb.finish_println("");
    println!(
        "......len of gram 1: {}\n......len of gram 2: {}\n......len of gram 3: {}\n\
         ......len of gram 4: {}",
        gram_1.len(),
        gram_2.len(),
        gram_3.len(),
        gram_4.len(),
    );
}

pub fn gen_gram(
    path: &str,
    save_path: &str,
    k_th: (usize, usize, usize, usize),
    hanzi_v: &Vec<char>,
    hanzi_m: &HashMap<char, usize>,
    word_v: &Vec<Vec<usize>>,
    word_m: &HashMap<Vec<usize>, usize>,
    pinyin_m: &HashMap<String, Vec<usize>>,
    jb: &Jieba,
) {
    println!("Generating gram-n");
    let paths = fs::read_dir(path).unwrap();
    let mut gram_1 = HashMap::new();
    let mut gram_2 = HashMap::new();
    let mut gram_3 = HashMap::new();
    let mut gram_4 = HashMap::new();
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
            );
        }
    }

    println!("...summaring");
    let mut word2pinyin = HashMap::new();
    pinyin_m.iter().for_each(|(k, v)| {
        v.iter().for_each(|w| word2pinyin.entry(*w).or_insert(Vec::new()).push(k))
    });
    macro_rules! short {
        ($g:ident, $kk:expr) => {
            let mut $g: Vec<_> = $g.into_iter().collect();
            let k = $g.len().min($kk);
            kth(&mut $g, k);
            $g.truncate(k);
        };
    }
    short!(gram_2, k_th.1);
    short!(gram_3, k_th.2);
    short!(gram_4, k_th.3);
    (2..word_v.len()).for_each(|w| {
        gram_1.entry(w).or_insert(1);
    });
    let gram_1: Vec<_> = gram_1.into_iter().collect();

    macro_rules! cmp {
        ($a:expr, $c:expr) => {{
            for i in 0..$a.len() {
                if $a[i].len() != $c[i].len() {
                    return $a[i].len().cmp(&$c[i].len());
                } else if $a[i] != $c[i] {
                    return $a[i].cmp(&$c[i]);
                }
            }
            Ordering::Equal
        }};
    }
    macro_rules! word {
        ($w:expr) => {
            word2hanzi($w, hanzi_v, word_v)
        };
    }

    // 一个词若有多种读音，很难去算每种读音的概率是多少
    // 故只能暂时这样近似：
    // 每个词的每个读音都累加出现次数
    // 然后每个词计算所有读音自己出现的最大频率，作为所有读音的概率
    let mut cnt = HashMap::new();
    gram_1.iter().for_each(|(k, v)| {
        word2pinyin.get(k).unwrap().iter().for_each(|py| {
            *cnt.entry(*py).or_insert(0) += *v;
        });
    });
    let mut gram_1: Vec<_> = gram_1
        .iter()
        .map(|(k, v)| {
            let pys = word2pinyin.get(k).unwrap();
            let mn = pys.iter().map(|py| *cnt.get(*py).unwrap()).min().unwrap();
            (vec![word!(*k)], *v as f64 / mn as f64)
        })
        .collect();
    gram_1.sort_unstable_by(|(a, _), (c, _)| cmp!(a, c));

    // 和gram_1差不多，多音的单词只能找最大的
    // 比如说
    // 我们 再 0.2777777777777778
    // 我们 在 0.7222222222222222
    let mut cnt = HashMap::new();
    gram_2.iter().for_each(|(k, v)| {
        word2pinyin.get(&k.1).unwrap().iter().for_each(|py| {
            *cnt.entry((k.0, *py)).or_insert(0) += *v;
        });
    });
    let mut gram_2: Vec<_> = gram_2
        .iter()
        .map(|(k, v)| {
            let pys = word2pinyin.get(&k.1).unwrap();
            let mn = pys.iter().map(|py| *cnt.get(&(k.0, *py)).unwrap()).min().unwrap();
            (vec![word!(k.0), word!(k.1)], *v as f64 / mn as f64)
        })
        .collect();
    gram_2.sort_unstable_by(|(a, _), (c, _)| cmp!(a, c));

    // 同理
    let mut cnt = HashMap::new();
    gram_3.iter().for_each(|(k, v)| {
        word2pinyin.get(&k.2).unwrap().iter().for_each(|py| {
            *cnt.entry((k.0, k.1, *py)).or_insert(0) += *v;
        });
    });
    let mut gram_3: Vec<_> = gram_3
        .iter()
        .map(|(k, v)| {
            let pys = word2pinyin.get(&k.2).unwrap();
            let mn = pys.iter().map(|py| *cnt.get(&(k.0, k.1, *py)).unwrap()).min().unwrap();
            (vec![word!(k.0), word!(k.1), word!(k.2)], *v as f64 / mn as f64)
        })
        .collect();
    gram_3.sort_unstable_by(|(a, _), (c, _)| cmp!(a, c));

    // 同理
    let mut cnt = HashMap::new();
    gram_4.iter().for_each(|(k, v)| {
        word2pinyin.get(&k.3).unwrap().iter().for_each(|py| {
            *cnt.entry((k.0, k.1, k.2, *py)).or_insert(0) += *v;
        });
    });
    let mut gram_4: Vec<_> = gram_4
        .iter()
        .map(|(k, v)| {
            let pys = word2pinyin.get(&k.3).unwrap();
            let mn = pys.iter().map(|py| *cnt.get(&(k.0, k.1, k.2, *py)).unwrap()).min().unwrap();
            (vec![word!(k.0), word!(k.1), word!(k.2), word!(k.3)], *v as f64 / mn as f64)
        })
        .collect();
    gram_4.sort_unstable_by(|(a, _), (c, _)| cmp!(a, c));

    println!(
        "......len of gram 1: {}\n......len of gram 2: {}\n......len of gram 3: {}\n\
         ......len of gram 4: {}",
        gram_1.len(),
        gram_2.len(),
        gram_3.len(),
        gram_4.len(),
    );
    let errmsg = &format!("......Cannot save to {}", save_path);
    macro_rules! save {
        ($g:ident, $i:expr) => {
            println!("...writing gram_{}.txt", $i);
            File::create(&format!("{}/gram_{}.txt", save_path, $i))
                .expect(errmsg)
                .write_all(
                    $g.iter()
                        .fold(&mut String::new(), |s, (a, b)| {
                            s.push_str(&format!("{} {}\n", a.join(" "), b));
                            s
                        })
                        .as_bytes(),
                )
                .expect(errmsg);
        };
    }
    save!(gram_1, 1);
    save!(gram_2, 2);
    save!(gram_3, 3);
    save!(gram_4, 4);
    println!("...Done!");
}

pub fn gen_total_gram(path: &str) {
    println!("Generating total gram-n");
    let config = &format!("{}/gram.json", path);
    let data = fs::read_to_string(config).expect(&format!("...Unable to read {}", config));
    let data = serde_json::from_str::<Value>(&data).expect("...Cannot convert to json");
    let data = data.as_object().expect("...Invalid json");
    let mut sum = 0.0;
    let mut total_gram = Vec::new();
    for _ in 0..4 {
        total_gram.push(HashMap::new());
    }
    data.iter().for_each(|(k, v)| {
        let w = v.as_f64().expect("...Invalid number");
        for i in 0..4 {
            let fname = &format!("{}/{}/gram_{}.txt", path, k, i + 1);
            println!("...Working on {}", fname);
            fs::read_to_string(fname)
                .expect(&format!("......Cannot read {}", fname))
                .lines()
                .for_each(|line| {
                    let pos = line.rfind(" ").unwrap();
                    let p = line[pos + 1..].parse::<f64>().unwrap();
                    let ws: Vec<_> =
                        line[0..pos].split_whitespace().map(|s| s.to_string()).collect();
                    *total_gram[i].entry(ws).or_insert(0.0) += p * w;
                });
        }
        sum += w;
    });
    let mut gram: Vec<Vec<(&Vec<String>, f64)>> = Vec::new();
    for i in 0..4 {
        gram.push(total_gram[i].iter().map(|(k, v)| (k, *v / sum)).collect());
        gram[i].sort_unstable_by(|(a, _), (c, _)| {
            for i in 0..a.len() {
                if a[i].len() != c[i].len() {
                    return a[i].len().cmp(&c[i].len());
                } else if a[i] != c[i] {
                    return a[i].cmp(&c[i]);
                }
            }
            Ordering::Equal
        });
    }
    for i in 0..4 {
        let fname = &format!("{}/gram_{}.txt", path, i + 1);
        let errmsg = &format!("...Cannot save to {}", fname);
        File::create(fname)
            .expect(errmsg)
            .write_all(
                gram[i]
                    .iter()
                    .fold(&mut String::new(), |s, (a, b)| {
                        s.push_str(&format!("{} {}\n", a.join(" "), b));
                        s
                    })
                    .as_bytes(),
            )
            .expect(errmsg);
    }
    println!("...Done!");
}

fn word_filter(
    s: &str,
    data: &mut HashSet<String>,
    hanzi_m: &HashMap<char, usize>,
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
    hanzi_m: &HashMap<char, usize>,
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
    hanzi_m: &HashMap<char, usize>,
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

    let mut ref_py = HashMap::new();
    fs::read_to_string(ref_path).expect(&format!("...Cannot read {}", ref_path)).lines().for_each(
        |line| {
            let data: Vec<_> = line.split_whitespace().collect();
            let mut valid = true;
            for c in data[0].chars() {
                if hanzi_m.get(&c).is_none() {
                    valid = false;
                    break;
                }
            }
            if valid {
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

fn kth<T>(a: &mut Vec<(T, usize)>, k: usize) {
    if a.len() == 0 {
        return;
    }
    let (mut l, mut r) = (0, a.len() - 1);
    let k = k - 1;
    while l <= r {
        let mut pos = l;
        for i in l..r {
            if a[i].1 > a[r].1 {
                a.swap(i, pos);
                pos += 1;
            }
        }
        a.swap(pos, r);
        if pos == k {
            return;
        } else if pos > k {
            r = pos - 1;
        } else {
            l = pos + 1;
        }
    }
}
